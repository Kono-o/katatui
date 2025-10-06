use kolor::RGB;
use ratatui::prelude::{Color, Modifier};
use ratatui::style::Style;

pub trait Stylify {
   fn bold(&mut self);
   fn italic(&mut self);
   fn strike(&mut self);
   fn underline(&mut self);
   fn fore(&mut self, rgb: RGB);
   fn back(&mut self, rgb: RGB);
   fn from_fg(fg: RGB) -> Self;
   fn from_bg(bg: RGB) -> Self;
   fn from_fg_bg(fg: RGB, bg: RGB) -> Self;
   fn from_hex(fg: &str) -> Self;
   fn from_hex_bg(bg: &str) -> Self;
   fn from_hex_fg_bg(fg: &str, bg: &str) -> Self;
}

pub fn rgb_to_color(rgb: RGB) -> Color {
   Color::Rgb(
      (rgb.r() * 255.0) as u8,
      (rgb.g() * 255.0) as u8,
      (rgb.b() * 255.0) as u8,
   )
}

impl Stylify for Style {
   fn bold(&mut self) {
      *self = self.add_modifier(Modifier::BOLD)
   }

   fn italic(&mut self) {
      *self = self.add_modifier(Modifier::ITALIC)
   }
   fn strike(&mut self) {
      *self = self.add_modifier(Modifier::CROSSED_OUT)
   }
   fn underline(&mut self) {
      *self = self.add_modifier(Modifier::UNDERLINED)
   }

   fn fore(&mut self, rgb: RGB) {
      *self = self.fg(rgb_to_color(rgb))
   }

   fn back(&mut self, rgb: RGB) {
      *self = self.bg(rgb_to_color(rgb))
   }

   fn from_fg(fg: RGB) -> Self {
      let mut s = Style::default();
      s.fore(fg);
      s
   }

   fn from_bg(bg: RGB) -> Self {
      let mut s = Style::default();
      s.fore(bg.contrasty());
      s.back(bg);
      s
   }

   fn from_fg_bg(fg: RGB, bg: RGB) -> Self {
      let mut s = Style::default();
      s.fore(fg);
      s.back(bg);
      s
   }

   fn from_hex(fg: &str) -> Self {
      let mut s = Style::default();
      s.fore(RGB::from_hex(fg));
      s
   }

   fn from_hex_bg(bg: &str) -> Self {
      let mut s = Style::default();
      let bg = RGB::from_hex(bg);
      s.fore(bg.contrasty());
      s.back(bg);
      s
   }

   fn from_hex_fg_bg(fg: &str, bg: &str) -> Self {
      let mut s = Style::default();
      s.fore(RGB::from_hex(fg));
      s.back(RGB::from_hex(bg));
      s
   }
}
