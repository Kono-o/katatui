use crate::app::{App, AppOutput};
use crate::{app_err, entry};
use ratatui::crossterm::event;
use ratatui::prelude::*;
use ratatui::Frame;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct GState {
   reload: bool,
   exit: bool,
}

impl GState {
   pub fn request_reload(&mut self) {
      self.reload = true;
   }
   pub fn is_reloading(&self) -> bool {
      self.reload
   }
   pub fn request_exit(&mut self) {
      self.exit = true;
   }
   pub fn is_running(&self) -> bool {
      !self.exit
   }
}

#[derive(Debug)]
pub struct GLoop {
   frame: u32,
   tick: u32,
   t_fps: u32,
   t_tps: u32,
   fps: f32,
   tps: f32,
   budget: u128,
   f_ms: u128,
   t_ms: u128,
   last: Instant,
}

impl GLoop {
   pub(crate) fn new() -> Self {
      Self {
         frame: 0,
         tick: 0,
         t_fps: 24,
         t_tps: 24,
         fps: 0.0,
         tps: 0.0,
         budget: 0,
         f_ms: 0,
         t_ms: 0,
         last: Instant::now(),
      }
   }

   pub fn set_fps(&mut self, fps: u32) {
      self.t_fps = fps;
   }
   pub fn set_tps(&mut self, tps: u32) {
      self.t_tps = tps;
   }
   pub fn fps(&self) -> f32 {
      self.fps
   }
   pub fn tps(&self) -> f32 {
      self.tps
   }
   pub fn target_fps(&self) -> u32 {
      self.t_fps
   }
   pub fn target_tps(&self) -> u32 {
      self.t_tps
   }
   pub fn frame(&self) -> u32 {
      self.frame
   }
   pub fn tick(&self) -> u32 {
      self.tick
   }
   pub fn f_ms(&self) -> u128 {
      self.f_ms
   }
   pub fn t_ms(&self) -> u128 {
      self.t_ms
   }
   pub fn budget(&self) -> u128 {
      self.budget
   }
}

#[derive(Debug)]
pub struct TUI<A: App> {
   gloop: GLoop,
   gstate: GState,
   app: A,
}

impl<A: App> TUI<A> {
   pub(crate) fn new(cfg_src: String) -> AppOutput<TUI<A>> {
      let mut gloop = GLoop::new();
      match A::init(&mut gloop, cfg_src) {
         AppOutput::Ok(app) => AppOutput::Ok(Self {
            gloop,
            app,
            gstate: GState {
               reload: false,
               exit: false,
            },
         }),
         AppOutput::Nil => AppOutput::Nil,
         AppOutput::Err(e) => AppOutput::Err(e),
      }
   }
}

impl<A: App> TUI<A> {
   pub(crate) fn reload(&mut self) -> AppOutput<()> {
      let install_result = entry::install::<A>();
      if let AppOutput::Err(_) = install_result {
         return install_result;
      }
      let cfg_src = match entry::read_cfg_from_disk::<A>() {
         AppOutput::Ok(s) => s,
         AppOutput::Err(e) => return AppOutput::Err(e),
         AppOutput::Nil => return AppOutput::Nil,
      };
      self.gstate.reload = false;
      self.app.reload(&mut self.gloop, cfg_src)
   }

   pub(crate) fn run(&mut self, terminal: &mut ratatui::DefaultTerminal) -> AppOutput<()> {
      let mut last_update = Instant::now();
      let mut last_render = Instant::now();

      let logic_step = Duration::from_secs_f64(1.0 / self.gloop.t_tps as f64);
      let render_step = Duration::from_secs_f64(1.0 / self.gloop.t_fps as f64);

      let update_budget = std::cmp::max(logic_step, render_step);
      let mut logic_counter = 0;
      let mut last_tps_check = Instant::now();

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
            if eve {
               if let Ok(e) = event::read() {
                  self.app.logic(&mut self.gloop, &mut self.gstate, Some(e));
               }
            } else {
               self.app.logic(&mut self.gloop, &mut self.gstate, None);
            }

            if self.gstate.reload {
               self.reload();
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

impl<A: App> Widget for &TUI<A> {
   fn render(self, area: Rect, buf: &mut Buffer) {
      self.app.render(&self.gloop, &self.gstate, area, buf);
   }
}
