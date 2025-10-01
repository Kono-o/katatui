use crate::app_err;
use crate::tui::{install_cfg, read_cfg};
use crate::{App, AppOutput, GState, Runtime};
use mlua::Lua;
use ratatui::crossterm::event;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use std::time::{Duration, Instant};
use unicode_width::UnicodeWidthStr;

#[derive(Debug)]
pub struct TUIRef<'a> {
   pub runtime: &'a Runtime,
   pub gstate: &'a GState,
   pub cfg: &'a Lua,
   pub src: &'a str,
}

impl<'a> TUIRef<'a> {
   pub(crate) fn from(
      runtime: &'a Runtime,
      gstate: &'a GState,
      cfg: &'a Lua,
      src: &'a str,
   ) -> TUIRef<'a> {
      TUIRef {
         runtime,
         gstate,
         cfg,
         src,
      }
   }
}

#[derive(Debug)]
pub struct TUIMutRef<'a> {
   pub runtime: &'a mut Runtime,
   pub gstate: &'a mut GState,
   pub cfg: &'a mut Lua,
   pub src: &'a mut str,
}

impl<'a> TUIMutRef<'a> {
   pub(crate) fn from(
      runtime: &'a mut Runtime,
      gstate: &'a mut GState,
      cfg: &'a mut Lua,
      src: &'a mut str,
   ) -> TUIMutRef<'a> {
      TUIMutRef {
         runtime,
         gstate,
         cfg,
         src,
      }
   }
}

#[derive(Debug)]
pub struct TUI<A: App> {
   runtime: Runtime,
   gstate: GState,
   lua: Lua,
   src: String,
   app: A,
}

pub const DEBUG_COLOR: Color = Color::Green;

impl<A: App> Widget for &TUI<A> {
   fn render(self, area: Rect, buf: &mut Buffer) {
      self.app.render(self.as_ref(), buf);

      if self.gstate.is_debug() {
         let line1 = Rect::new(0, area.height - 2, area.width, 1);
         let t_fps = self.runtime.target_fps();
         let t_tps = self.runtime.target_tps();
         let fps_digits = t_fps.to_string().len();
         let tps_digits = t_tps.to_string().len();
         let frame_mod = self.runtime.frame() % t_fps;
         let tick_mod = self.runtime.tick() % t_tps;

         let left = format!(
            " frame: {:0width_fps$} [{}/{}] tick: {:0width_tps$} [{}/{}] ",
            frame_mod,
            self.runtime.fps() as u16,
            t_fps,
            tick_mod,
            self.runtime.tps() as u16,
            t_tps,
            width_fps = fps_digits,
            width_tps = tps_digits,
         );
         let right = format!(" {}: {} ", A::APP_NAME, A::CONFIG_FILE);
         let pad = (line1.width as usize).saturating_sub(left.width() + right.width());

         let style1 = Style::default()
            .bg(DEBUG_COLOR)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD);
         let text1 = Line::from(vec![
            Span::styled(left, style1),
            Span::styled(" ".repeat(pad), style1),
            Span::styled(right, style1),
         ]);

         Paragraph::new(text1)
            .block(Block::default())
            .render(line1, buf);

         let line2 = Rect::new(0, area.height - 1, area.width, 1);
         let style2 = Style::default().fg(self.gstate.msg.msg_type().color());
         let text2 = Text::from(Span::styled(format!(" {}", self.gstate.msg.msg()), style2));
         Paragraph::new(text2)
            .block(Block::default())
            .render(line2, buf);
      }
   }
}

impl<A: App> TUI<A> {
   pub fn run() {
      let mut output = AppOutput::nil();
      let lua_dir = match install_cfg::<A>() {
         AppOutput::Ok(p) => p,
         AppOutput::Err(e) => {
            AppOutput::<()>::Err(e).out();
            return;
         }
         AppOutput::Nil => {
            AppOutput::<()>::Nil.out();
            return;
         }
      };

      let src = match read_cfg::<A>() {
         AppOutput::Ok(s) => s,
         AppOutput::Nil => {
            AppOutput::<()>::Nil.out();
            return;
         }
         AppOutput::Err(e) => {
            AppOutput::<()>::Err(e).out();
            return;
         }
      };
      let lua = Lua::new();

      let pkg_dir = format!("{lua_dir}/?.lua;{lua_dir}/?/?.lua;{lua_dir}/?/?/?.lua");
      let pkg_src = format!("package.path = '{pkg_dir};' .. package.path");

      output = match lua.load(&pkg_src).exec() {
         Err(e) => app_err!("failed to set lua pkg dir {}", e),
         _ => AppOutput::<()>::Nil,
      };

      output = match lua.load(&src).exec() {
         Err(e) => app_err!("failed to load lua {}", e),
         _ => AppOutput::<()>::Nil,
      };

      let mut terminal = ratatui::init();
      output = match TUI::<A>::init(lua, src) {
         AppOutput::Ok(mut tui) => {
            tui.reload_lua();
            tui.lua_update_fn_call();
            tui.run_loop(&mut terminal)
         }
         AppOutput::Err(e) => AppOutput::Err(e),
         AppOutput::Nil => AppOutput::<()>::Nil,
      };
      ratatui::restore();
      output.out()
   }

   pub(crate) fn init(mut lua: Lua, mut src: String) -> AppOutput<TUI<A>> {
      let mut runtime = Runtime::new();
      let mut gstate = GState::new();

      let tui_ref_mut = TUIMutRef {
         runtime: &mut runtime,
         gstate: &mut gstate,
         cfg: &mut lua,
         src: &mut src,
      };

      match A::init(tui_ref_mut) {
         AppOutput::Ok(app) => {
            let mut tui = Self {
               runtime,
               app,
               gstate,
               lua,
               src,
            };
            tui.lua_init_fn_call();
            AppOutput::ok(tui)
         }
         AppOutput::Err(e) => AppOutput::Err(e),
         AppOutput::Nil => AppOutput::nil(),
      }
   }

   pub(crate) fn as_ref(&self) -> TUIRef {
      TUIRef {
         runtime: &self.runtime,
         gstate: &self.gstate,
         cfg: &self.lua,
         src: &self.src,
      }
   }

   pub(crate) fn reload_lua(&mut self) -> AppOutput<()> {
      let _lua_dir = match install_cfg::<A>() {
         AppOutput::Ok(p) => p,
         AppOutput::Err(e) => return AppOutput::<()>::Err(e),
         AppOutput::Nil => return AppOutput::<()>::Nil,
      };

      let new_cfg_src = match read_cfg::<A>() {
         AppOutput::Ok(s) => s,
         AppOutput::Err(e) => return AppOutput::Err(e),
         AppOutput::Nil => return AppOutput::Nil,
      };

      self.src = new_cfg_src;
      self.gstate.set_reload(false);
      self.gstate.set_just_reloaded(true);
      self.gstate.msg.set_event_msg("reloaded cfg!");

      self.load_lua()
   }

   pub(crate) fn load_lua(&mut self) -> AppOutput<()> {
      match self.lua.load(&self.src).exec() {
         Err(e) => app_err!("failed to load lua {}", e),
         _ => AppOutput::<()>::nil(),
      };
      AppOutput::<()>::nil()
   }

   pub(crate) fn lua_update_fn_call(&mut self) -> AppOutput<()> {
      match self.lua.globals().get::<mlua::Function>("update") {
         Ok(f) => {
            let _ = f.call::<()>(());
            AppOutput::nil()
         }
         Err(e) => app_err!("failed to run cfg {}", e),
      }
   }
   pub(crate) fn lua_init_fn_call(&mut self) -> AppOutput<()> {
      match self.lua.globals().get::<mlua::Function>("init") {
         Ok(f) => {
            let _ = f.call::<()>(());
            AppOutput::nil()
         }
         Err(e) => app_err!("failed to run cfg {}", e),
      }
   }

   pub(crate) fn run_loop(&mut self, terminal: &mut DefaultTerminal) -> AppOutput<()> {
      let mut last_update = Instant::now();
      let mut last_render = Instant::now();

      let logic_step = Duration::from_secs_f64(1.0 / self.runtime.t_tps as f64);
      let render_step = Duration::from_secs_f64(1.0 / self.runtime.t_fps as f64);

      let update_budget = std::cmp::max(logic_step, render_step);
      let mut last_tps_check = Instant::now();
      let mut logic_counter = 0;

      while self.gstate.is_running() {
         let now = Instant::now();
         let mut time_used = Duration::ZERO;

         // --- Logic Tick ---
         if now.duration_since(last_update) >= logic_step {
            let tick_start = Instant::now();
            let eve = match event::poll(Duration::ZERO) {
               Ok(ev) => ev,
               Err(e) => {
                  return app_err!("failed to poll event: {e}");
               }
            };
            self.lua_update_fn_call();
            self.gstate.msg.set_info_msg("");
            if eve {
               if let Ok(e) = event::read() {
                  let tui_mut = TUIMutRef::from(
                     &mut self.runtime,
                     &mut self.gstate,
                     &mut self.lua,
                     &mut self.src,
                  );
                  self.app.logic(tui_mut, Some(e));
               }
            } else {
               let tui_mut = TUIMutRef::from(
                  &mut self.runtime,
                  &mut self.gstate,
                  &mut self.lua,
                  &mut self.src,
               );
               self.app.logic(tui_mut, None);
            }

            self.gstate.set_just_reloaded(false);
            if self.gstate.is_reloading() {
               self.reload_lua();
            }

            self.runtime.t_ms = tick_start.elapsed().as_micros();
            self.runtime.tick = self.runtime.tick.wrapping_add(1);

            last_update += logic_step;
            logic_counter += 1;
            time_used += tick_start.elapsed();
         }

         // --- Render Frame ---
         while time_used < update_budget && now.duration_since(last_render) >= render_step {
            let frame_start = Instant::now();

            let delta = frame_start.duration_since(self.runtime.last).as_secs_f32();
            if delta > 0.0 {
               self.runtime.fps = 1.0 / delta;
            }
            self.runtime.last = frame_start;

            match terminal.draw(|frame: &mut Frame| {
               frame.render_widget(&*self, frame.area());
            }) {
               Err(e) => {
                  return app_err!("failed to render frame: {e}");
               }
               _ => {}
            };
            self.runtime.frame = self.runtime.frame.wrapping_add(1);

            self.runtime.f_ms = frame_start.elapsed().as_micros();
            last_render += render_step;
            time_used += frame_start.elapsed();
         }
         if last_tps_check.elapsed() >= Duration::from_secs(1) {
            self.runtime.tps = logic_counter as f32;
            logic_counter = 0;
            last_tps_check = Instant::now();
         }
         self.runtime.budget = update_budget.as_micros();
      }
      AppOutput::nil()
   }
}
