use crate::tui::{GLoop, GState};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event;
use ratatui::layout::Rect;

pub trait App {
   const APP_NAME: &'static str;
   const CONFIG_FILE: &'static str;
   const DEFAULT_CONFIG_SRC: &'static str;
   fn init(gloop: &mut GLoop, cfg_src: String) -> AppOutput<Self>
   where
      Self: Sized;
   fn reload(&mut self, gloop: &mut GLoop, cfg_src: String) -> AppOutput<()>
   where
      Self: Sized;
   fn logic(&mut self, gloop: &mut GLoop, gstate: &mut GState, event: Option<event::Event>)
   where
      Self: Sized;
   fn render(&self, gloop: &GLoop, gstate: &GState, area: Rect, buf: &mut Buffer)
   where
      Self: Sized;
}

pub enum AppOutput<T> {
   Ok(T),
   Nil,
   Err(String),
}

impl<T> AppOutput<T> {
   pub fn out(self) {
      match self {
         AppOutput::Err(e) => eprintln!("{}", e),
         _ => {}
      }
   }
   pub fn nil() -> Self {
      AppOutput::Nil
   }
}

#[macro_export]
macro_rules! app_err {
    ($($arg:tt)*) => {
        AppOutput::Err(format!($($arg)*))
    };
}
