use crate::{LuaResult, LuaTable};
use std::time::Instant;

#[derive(Debug)]
pub struct GLoop {
   pub(crate) start: Instant,
   pub(crate) frame: u32,
   pub(crate) tick: u32,
   pub(crate) t_fps: u32,
   pub(crate) t_tps: u32,
   pub(crate) fps: f32,
   pub(crate) tps: f32,
   pub(crate) budget: u128,
   pub(crate) f_ms: u128,
   pub(crate) t_ms: u128,
   pub(crate) last: Instant,
}

impl GLoop {
   pub(crate) fn new() -> Self {
      let now = Instant::now();
      Self {
         start: now,
         last: now,
         frame: 0,
         tick: 0,
         t_fps: 16,
         t_tps: 8,
         fps: 0.0,
         tps: 0.0,
         budget: 0,
         f_ms: 0,
         t_ms: 0,
      }
   }

   pub fn to_lua(&self, lua: &mlua::Lua) -> LuaResult<LuaTable> {
      let table = lua.create_table()?;
      table.set("fps", self.fps)?;
      table.set("tps", self.tps)?;
      table.set("frame", self.frame)?;
      table.set("tick", self.tick)?;
      table.set("fps", self.fps)?;
      let elapsed = self.start.elapsed().as_secs_f32();
      table.set("elapsed", elapsed)?;
      Ok(table)
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
