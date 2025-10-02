use ratatui::prelude::Color;

#[derive(Debug)]
pub struct Msg {
   msg: String,
   typ: MsgType,
}

#[derive(Debug)]
pub enum MsgType {
   Info,
   Event,
   Warn,
   Error,
}

impl MsgType {
   pub fn color(&self) -> Color {
      match self {
         MsgType::Info => Color::Blue,
         MsgType::Event => Color::Green,
         MsgType::Warn => Color::Yellow,
         MsgType::Error => Color::Red,
      }
   }
}

impl Msg {
   pub fn new(msg: &str, typ: MsgType) -> Self {
      Self {
         msg: msg.to_string(),
         typ,
      }
   }
   pub fn clear(&mut self) {
      self.set_msg("", MsgType::Info);
   }
   pub fn msg(&self) -> &str {
      &self.msg
   }
   pub fn set_msg(&mut self, msg: &str, typ: MsgType) {
      self.msg = msg.to_string();
      self.typ = typ;
   }

   pub fn set_info_msg(&mut self, msg: &str) {
      self.msg = msg.to_string();
      self.typ = MsgType::Info;
   }
   pub fn set_warn_msg(&mut self, msg: &str) {
      self.msg = msg.to_string();
      self.typ = MsgType::Warn;
   }
   pub fn set_error_msg(&mut self, msg: &str) {
      self.msg = msg.to_string();
      self.typ = MsgType::Error;
   }
   pub fn set_event_msg(&mut self, msg: &str) {
      self.msg = msg.to_string();
      self.typ = MsgType::Event;
   }
   pub fn msg_type(&self) -> &MsgType {
      &self.typ
   }
}

#[derive(Debug)]
pub struct Debug {
   pub current_log: Msg,
   pub(crate) current_fn: Msg,
}

impl Debug {
   pub(crate) fn new() -> Self {
      Self {
         current_log: Msg::new("???", MsgType::Info),
         current_fn: Msg::new("???", MsgType::Info),
      }
   }
}
