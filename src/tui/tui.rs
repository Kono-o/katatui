use crate::app_err;
use crate::tui::{install_cfg, read_cfg, Cfg, CfgSrc};
use crate::{App, AppOutput, Debug, Runtime};
use mlua::Lua;
use ratatui::crossterm::event;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use std::env;
use std::time::{Duration, Instant};
use unicode_width::UnicodeWidthStr;

#[derive(Debug)]
pub struct TUIRef<'a> {
   pub runtime: &'a Runtime,
   pub debug: &'a Debug,
   pub cfg: &'a Cfg,
   pub args: &'a Vec<String>,
}

impl<'a> TUIRef<'a> {
   pub(crate) fn from(
      runtime: &'a Runtime,
      debug: &'a Debug,
      cfg: &'a Cfg,
      args: &'a Vec<String>,
   ) -> TUIRef<'a> {
      TUIRef {
         runtime,
         debug,
         cfg,
         args,
      }
   }
}

#[derive(Debug)]
pub struct TUIMutRef<'a> {
   pub runtime: &'a mut Runtime,
   pub debug: &'a mut Debug,
   pub cfg: &'a mut Cfg,
   pub args: &'a mut Vec<String>,
}
impl<'a> TUIMutRef<'a> {
   pub(crate) fn from(
      runtime: &'a mut Runtime,
      debug: &'a mut Debug,
      cfg: &'a mut Cfg,
      args: &'a mut Vec<String>,
   ) -> TUIMutRef<'a> {
      TUIMutRef {
         runtime,
         debug,
         cfg,
         args,
      }
   }
}

#[derive(Debug)]
pub struct TUI<A: App> {
   runtime: Runtime,
   debug: Debug,
   cfg: Cfg,
   args: Vec<String>,
   app: A,
}

impl<A: App> TUI<A> {
   pub fn run() {
      let mut output = AppOutput::void();
      let cfg_path = match install_cfg::<A>() {
         AppOutput::Ok(cfg_p) => cfg_p,
         AppOutput::Err(e) => {
            AppOutput::<()>::Err(e).out();
            return;
         }
         AppOutput::Nil => {
            AppOutput::<()>::void().out();
            return;
         }
      };

      let cfg_src = match read_cfg::<A>() {
         AppOutput::Ok(s) => s,
         AppOutput::Nil => {
            AppOutput::<()>::void().out();
            return;
         }
         AppOutput::Err(e) => {
            AppOutput::<()>::Err(e).out();
            return;
         }
      };
      let mut cfg = None;

      match cfg_path {
         Some(p) => {
            let lua = Lua::new();
            let pkg_dir = format!("{p}/?.lua;{p}/?/?.lua;{p}/?/?/?.lua");
            let pkg_src = format!("package.path = '{pkg_dir};' .. package.path");

            output = match lua.load(&pkg_src).exec() {
               Err(e) => app_err!("failed to set lua pkg dir {}", e),
               _ => AppOutput::<()>::Nil,
            };
            match cfg_src {
               Some(src) => {
                  output = match lua.load(&src).exec() {
                     Err(e) => app_err!("failed to load lua {}", e),
                     _ => {
                        cfg = Some(lua);
                        AppOutput::<()>::void()
                     }
                  };
               }
               None => {}
            }
         }
         None => {}
      }

      let mut terminal = ratatui::init();
      output = match TUI::<A>::init(cfg) {
         AppOutput::Ok(mut tui) => {
            tui.reload_lua();
            tui.run_loop(&mut terminal);
            AppOutput::<()>::void()
         }
         AppOutput::Err(e) => AppOutput::Err(e),
         AppOutput::Nil => AppOutput::<()>::void(),
      };
      ratatui::restore();
      output.out()
   }

   pub(crate) fn init(mut cfg: Cfg) -> AppOutput<TUI<A>> {
      let mut runtime = Runtime::new();
      let mut debug = Debug::new();
      let mut args = env::args().skip(1).collect();

      let tui_ref_mut = TUIMutRef {
         runtime: &mut runtime,
         debug: &mut debug,
         cfg: &mut cfg,
         args: &mut args,
      };

      let app = A::init(tui_ref_mut);
      let mut tui = Self {
         runtime,
         app,
         debug,
         args,
         cfg,
      };
      tui.lua_fn_call("init");
      tui.debug.current_fn.set_info_msg("init");
      AppOutput::ok(tui)
   }

   pub(crate) fn reload_lua(&mut self) -> AppOutput<()> {
      let _cfg_dir = match install_cfg::<A>() {
         AppOutput::Ok(p) => p,
         AppOutput::Err(e) => return AppOutput::<()>::Err(e),
         AppOutput::Nil => return AppOutput::<()>::void(),
      };

      let cfg_src = match read_cfg::<A>() {
         AppOutput::Ok(s) => s,
         AppOutput::Err(e) => return AppOutput::Err(e),
         AppOutput::Nil => return AppOutput::void(),
      };

      self.runtime.set_reload(false);
      self.runtime.set_just_reloaded(true);
      self.debug.current_log.set_event_msg("reloaded cfg!");
      self.load_lua(cfg_src)
   }

   pub(crate) fn load_lua(&mut self, src: CfgSrc) -> AppOutput<()> {
      match &self.cfg {
         Some(lua) => match lua.load(src.unwrap_or("".to_string())).exec() {
            Err(e) => app_err!("failed to load lua {}", e),
            _ => AppOutput::<()>::void(),
         },
         None => AppOutput::<()>::void(),
      }
   }

   pub(crate) fn lua_fn_call(&mut self, func: &str) -> AppOutput<()> {
      match &self.cfg {
         Some(lua) => match lua.globals().get::<mlua::Function>(func) {
            Ok(f) => {
               let _ = f.call::<()>(());
               AppOutput::void()
            }
            Err(e) => app_err!("failed to run cfg fn {func} {}", e),
         },
         None => AppOutput::<()>::void(),
      }
   }

   pub(crate) fn run_loop(&mut self, terminal: &mut DefaultTerminal) {
      let mut last_update = Instant::now();
      let mut last_render = Instant::now();

      let mut last_tps_check = Instant::now();
      let mut last_fps_check = Instant::now();

      let mut logic_counter = 0;
      let mut frame_counter = 0;

      while self.runtime.is_running() {
         let now = Instant::now();

         // recompute steps every loop so changes to t_tps / t_fps take effect
         let logic_step = Duration::from_secs_f64(1.0 / self.runtime.t_tps as f64);
         let render_step = Duration::from_secs_f64(1.0 / self.runtime.t_fps as f64);

         // --- Logic Tick ---
         if now.duration_since(last_update) >= logic_step {
            let tick_start = Instant::now();
            self.logic();

            self.runtime.t_ms = tick_start.elapsed().as_micros();
            self.runtime.tick = self.runtime.tick.wrapping_add(1);

            last_update += logic_step;
            logic_counter += 1;
         }

         // --- Render Frame ---
         if now.duration_since(last_render) >= render_step {
            let frame_start = Instant::now();

            self.render_to(terminal);
            self.runtime.frame = self.runtime.frame.wrapping_add(1);

            self.runtime.f_ms = frame_start.elapsed().as_micros();
            last_render += render_step;
            frame_counter += 1;
         }

         // --- Update TPS ---
         if last_tps_check.elapsed() >= Duration::from_secs(1) {
            self.runtime.tps = logic_counter as f32;
            logic_counter = 0;
            last_tps_check = Instant::now();
         }

         // --- Update FPS ---
         if last_fps_check.elapsed() >= Duration::from_secs(1) {
            self.runtime.fps = frame_counter as f32;
            frame_counter = 0;
            last_fps_check = Instant::now();
         }
      }
   }

   pub(crate) fn logic(&mut self) {
      let eve = match event::poll(Duration::ZERO) {
         Ok(ev) => ev,
         Err(_) => {
            return;
         }
      };
      self.lua_fn_call("tick");
      self.debug.current_log.set_info_msg("");
      self.debug.current_fn.set_info_msg("tick");
      if eve {
         if let Ok(e) = event::read() {
            let tui_mut = TUIMutRef::from(
               &mut self.runtime,
               &mut self.debug,
               &mut self.cfg,
               &mut self.args,
            );
            self.app.logic(tui_mut, Some(e));
         }
      } else {
         let tui_mut = TUIMutRef::from(
            &mut self.runtime,
            &mut self.debug,
            &mut self.cfg,
            &mut self.args,
         );
         self.app.logic(tui_mut, None);
      }

      self.runtime.set_just_reloaded(false);
      if self.runtime.is_reloading() {
         self.reload_lua();
      }
   }

   pub(crate) fn render_to(&self, terminal: &mut DefaultTerminal) {
      match terminal.draw(|frame: &mut Frame| {
         frame.render_widget(&*self, frame.area());
      }) {
         Err(_) => {}
         _ => {}
      };
   }
}

impl<A: App> Widget for &TUI<A> {
   fn render(self, area: Rect, buf: &mut Buffer) {
      self.app.render(
         TUIRef::from(&self.runtime, &self.debug, &self.cfg, &self.args),
         buf,
      );

      if !self.runtime.is_debug() {
         return;
      }
      let dbg_line = Rect::new(0, area.height - 1, area.width, 1);
      let t_fps = self.runtime.target_fps();
      let t_tps = self.runtime.target_tps();
      let fps_digits = t_fps.to_string().len();
      let tps_digits = t_tps.to_string().len();
      let frame_mod = self.runtime.frame() % t_fps;
      let tick_mod = self.runtime.tick() % t_tps;

      let info_txt = format!(
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
      let app_txt = format!(" {}: ", A::APP_NAME);
      let cfg_txt = match A::CONFIG_FILE {
         Some(c) => format!(" {c} -> "),
         None => " no cfg ".to_string(),
      };

      let fn_total_width: usize = 8;
      let fn_name_str = self.debug.current_fn.msg();

      let mut fn_total_pad = 0;
      let mut fn_left_pad = 0;
      let mut fn_right_pad = 0;

      let fn_txt = match A::CONFIG_FILE {
         Some(f) => {
            fn_total_pad = fn_total_width.saturating_sub(fn_name_str.len());
            fn_left_pad = fn_total_pad / 2;
            fn_right_pad = fn_total_pad - fn_left_pad + 2;
            format!(
               " fn {}{}{}()",
               " ".repeat(fn_left_pad),
               fn_name_str,
               " ".repeat(fn_right_pad)
            )
         }
         None => "".to_string(),
      };

      let pad = (dbg_line.width as usize)
         .saturating_sub(info_txt.width() + app_txt.width() + cfg_txt.width() + fn_txt.width());

      let log_total_width: usize = pad;
      let log_name_str = self.debug.current_log.msg();

      let log_title = " log: ";
      let log_total_pad = log_total_width.saturating_sub(log_name_str.len() + log_title.len());
      let log_left_pad = log_total_pad / 2;
      let log_right_pad = log_total_pad - log_left_pad;

      let log_txt = format!(
         "{log_title}{}{}{}",
         " ".repeat(log_left_pad),
         log_name_str,
         " ".repeat(log_right_pad)
      );

      let dbg_style = Style::default()
         .bg(Color::LightMagenta)
         .fg(Color::Black)
         .add_modifier(Modifier::BOLD);
      let app_style = dbg_style
         .clone()
         .bg(Color::Magenta)
         .add_modifier(Modifier::BOLD);
      let cfg_style = dbg_style.clone().bg(Color::White);
      let fn_style = dbg_style.clone().bg(Color::Magenta);
      let log_style = dbg_style
         .clone()
         .bg(self.debug.current_log.msg_type().color());

      let dbg_text = Line::from(vec![
         Span::styled(app_txt, app_style),
         Span::styled(info_txt, cfg_style),
         Span::styled(log_txt, log_style),
         Span::styled(cfg_txt, cfg_style),
         Span::styled(fn_txt, fn_style),
      ]);

      Paragraph::new(dbg_text)
         .block(Block::default())
         .render(dbg_line, buf);
   }
}
