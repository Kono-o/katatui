use ratatui::prelude::Color;

pub fn hex_to_rgb(hex: &str) -> Color {
   let hex = hex.trim_start_matches('#');
   if hex.len() != 6 {
      return Color::Rgb(255, 255, 255);
   }
   let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
   let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
   let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
   Color::Rgb(r, g, b)
}
