use crate::tui::{install_cfg, read_cfg};
use crate::{app_err, MsgType};
use crate::{App, AppOutput, GLoop, GState};
use mlua::Lua;
use ratatui::crossterm::event;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct TUIRef<'a> {
   pub gloop: &'a GLoop,
   pub gstate: &'a GState,
   pub cfg: &'a Lua,
   pub src: &'a str,
}

impl<'a> TUIRef<'a> {
   pub(crate) fn from(
      gloop: &'a GLoop,
      gstate: &'a GState,
      cfg: &'a Lua,
      src: &'a str,
   ) -> TUIRef<'a> {
      TUIRef {
         gloop,
         gstate,
         cfg,
         src,
      }
   }
}

#[derive(Debug)]
pub struct TUIMutRef<'a> {
   pub gloop: &'a mut GLoop,
   pub gstate: &'a mut GState,
   pub cfg: &'a mut Lua,
   pub src: &'a mut str,
}

impl<'a> TUIMutRef<'a> {
   pub(crate) fn from(
      gloop: &'a mut GLoop,
      gstate: &'a mut GState,
      cfg: &'a mut Lua,
      src: &'a mut str,
   ) -> TUIMutRef<'a> {
      TUIMutRef {
         gloop,
         gstate,
         cfg,
         src,
      }
   }
}

#[derive(Debug)]
pub struct TUI<A: App> {
   gloop: GLoop,
   gstate: GState,
   lua: Lua,
   src: String,
   app: A,
}

impl<A: App> Widget for &TUI<A> {
   fn render(self, area: Rect, buf: &mut Buffer) {
      self.app.render(self.as_ref(), buf);
      if self.gstate.is_debug() {
         let line = Rect {
            x: 0,
            y: 0,
            width: area.width,
            height: 1,
         };

         let style = Style::new()
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD)
            .bg(match self.gstate.msg.msg_type() {
               MsgType::Info => Color::Blue,
               MsgType::Event => Color::Green,
               MsgType::Warn => Color::Yellow,
               MsgType::Error => Color::Red,
            });
         Paragraph::new(self.gstate.msg.msg())
            .block(Block::default())
            .style(style)
            .render(line, buf);
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
      let mut gloop = GLoop::new();
      let mut gstate = GState::new();

      let tui_ref_mut = TUIMutRef {
         gloop: &mut gloop,
         gstate: &mut gstate,
         cfg: &mut lua,
         src: &mut src,
      };

      match A::init(tui_ref_mut) {
         AppOutput::Ok(app) => {
            let mut tui = Self {
               gloop,
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
         gloop: &self.gloop,
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

      let logic_step = Duration::from_secs_f64(1.0 / self.gloop.t_tps as f64);
      let render_step = Duration::from_secs_f64(1.0 / self.gloop.t_fps as f64);

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
            self.gstate.msg.clear();
            if eve {
               if let Ok(e) = event::read() {
                  let tui_mut = TUIMutRef::from(
                     &mut self.gloop,
                     &mut self.gstate,
                     &mut self.lua,
                     &mut self.src,
                  );
                  self.app.logic(tui_mut, Some(e));
               }
            } else {
               let tui_mut = TUIMutRef::from(
                  &mut self.gloop,
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

            self.gloop.t_ms = tick_start.elapsed().as_micros();
            self.gloop.tick = self.gloop.tick.wrapping_add(1);

            last_update += logic_step;
            logic_counter += 1;
            time_used += tick_start.elapsed();
         }

         // --- Render Frame ---
         while time_used < update_budget && now.duration_since(last_render) >= render_step {
            let frame_start = Instant::now();

            let delta = frame_start.duration_since(self.gloop.last).as_secs_f32();
            if delta > 0.0 {
               self.gloop.fps = 1.0 / delta;
            }
            self.gloop.last = frame_start;

            match terminal.draw(|frame: &mut Frame| {
               frame.render_widget(&*self, frame.area());
            }) {
               Err(e) => {
                  return app_err!("failed to render frame: {e}");
               }
               _ => {}
            };
            self.gloop.frame = self.gloop.frame.wrapping_add(1);

            self.gloop.f_ms = frame_start.elapsed().as_micros();
            last_render += render_step;
            time_used += frame_start.elapsed();
         }
         if last_tps_check.elapsed() >= Duration::from_secs(1) {
            self.gloop.tps = logic_counter as f32;
            logic_counter = 0;
            last_tps_check = Instant::now();
         }
         self.gloop.budget = update_budget.as_micros();
      }
      AppOutput::nil()
   }
}
