use crate::{TUIMutRef, TUIRef};
use ratatui::crossterm::event::Event;
use ratatui::prelude::Buffer;

pub trait App {
   const APP_NAME: &'static str;
   const CONFIG_FILE: &'static str;
   const DEFAULT_CONFIG_SRC: &'static str;
   fn init(tui: TUIMutRef) -> AppOutput<Self>
   where
      Self: Sized;
   fn logic(&mut self, tui: TUIMutRef, event: Option<Event>)
   where
      Self: Sized;
   fn render(&self, tui: TUIRef, buf: &mut Buffer)
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
   pub fn ok(t: T) -> Self {
      AppOutput::Ok(t)
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
#[macro_export]
macro_rules! app_nil {
   ($($arg:tt)*) => {
      AppOutput::Nil
   };
}
