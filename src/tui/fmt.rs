use kolor::RGB;
use ratatui::prelude::{Color, Modifier};
use ratatui::style::Style;
use ratatui::text::Line;

pub trait Stylify {
   fn bolden(&mut self);
   fn italize(&mut self);
   fn striked(&mut self);
   fn underlined(&mut self);
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
   fn bolden(&mut self) {
      *self = self.add_modifier(Modifier::BOLD)
   }

   fn italize(&mut self) {
      *self = self.add_modifier(Modifier::ITALIC)
   }
   fn striked(&mut self) {
      *self = self.add_modifier(Modifier::CROSSED_OUT)
   }
   fn underlined(&mut self) {
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
