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
pub struct GState {
   pub(crate) reload: bool,
   pub(crate) just_reloaded: bool,
   pub(crate) debug: bool,
   pub(crate) exit: bool,
   pub msg: Msg,
}

impl GState {
   pub(crate) fn new() -> Self {
      Self {
         reload: false,
         just_reloaded: false,
         debug: false,
         exit: false,
         msg: Msg::new("???", MsgType::Info),
      }
   }

   pub fn request_reload(&mut self) {
      self.reload = true;
   }
   pub fn is_reloading(&self) -> bool {
      self.reload
   }
   pub fn just_reloaded(&self) -> bool {
      self.just_reloaded
   }
   pub fn request_exit(&mut self) {
      self.exit = true;
   }
   pub fn is_running(&self) -> bool {
      !self.exit
   }
   pub fn toggle_debug(&mut self) {
      self.debug = !self.debug;
   }
   pub fn is_debug(&self) -> bool {
      self.debug
   }

   pub(crate) fn set_reload(&mut self, req: bool) {
      self.reload = req;
   }
   pub(crate) fn set_just_reloaded(&mut self, req: bool) {
      self.just_reloaded = req;
   }
   pub(crate) fn set_debug(&mut self, dbg: bool) {
      self.debug = dbg;
   }
   pub(crate) fn set_exit(&mut self, exit: bool) {
      self.exit = exit;
   }
}
