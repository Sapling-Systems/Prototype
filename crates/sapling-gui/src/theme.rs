use raylib::{RaylibHandle, RaylibThread, color::Color, math::Vector2};

use crate::{base::DropShadowStyle, font::Font};

pub struct Theme {
  pub font_primary: Option<Font>,
  pub font_primary_bold: Option<Font>,
  pub color_primary: Color,
  pub color_background: Color,
  pub color_background_contrast: Color,
  pub color_background_highlight: Color,
  pub radius_default: f32,
  pub radius_large: f32,
  pub spacing_tiny: f32,
  pub spacing_small: f32,
  pub spacing_default: f32,
  pub spacing_large: f32,
  pub spacing_xlarge: f32,
  pub drop_shadow_default: DropShadowStyle,
}

#[derive(Debug, Clone, Copy)]
pub enum FontVariant {
  Primary,
  DefaultForeground,
  DefaultForegroundBold,
  Custom { color: Color, size: f32 },
}

impl Theme {
  pub fn new(rl: &mut RaylibHandle, thread: &RaylibThread) -> Self {
    let primary_font =
      Font::new(rl, thread, "./apps/ide/assets/fonts/FiraMono-Medium.ttf").unwrap();

    let primary_font_bold =
      Font::new(rl, thread, "./apps/ide/assets/fonts/FiraMono-Bold.ttf").unwrap();

    Self {
      font_primary: Some(primary_font),
      font_primary_bold: Some(primary_font_bold),
      ..Self::no_fonts()
    }
  }

  pub fn no_fonts() -> Self {
    Self {
      font_primary: None,
      font_primary_bold: None,
      color_primary: Color::from_hex("16A085").unwrap(),
      color_background: Color::from_hex("212121").unwrap(),
      color_background_contrast: Color::from_hex("F4F4F4").unwrap(),
      color_background_highlight: Color::from_hex("E1E3E5").unwrap(),
      spacing_tiny: 2.0,
      spacing_small: 4.0,
      spacing_default: 8.0,
      spacing_large: 16.0,
      spacing_xlarge: 32.0,
      radius_default: 8.0,
      radius_large: 16.0,
      drop_shadow_default: DropShadowStyle {
        color: Color::BLACK.alpha(0.70),
        offset: Vector2::new(2.0, 4.0),
        blur_radius: 4.0,
      },
    }
  }

  pub fn text_config<'a>(&'a mut self, variant: FontVariant) -> FontConfig<'a> {
    match variant {
      FontVariant::Primary => FontConfig {
        font: self.font_primary.as_mut().unwrap(),
        size: 14.0,
        color: Color::from_hex("2C3E50").unwrap(),
      },
      FontVariant::DefaultForeground => FontConfig {
        font: self.font_primary.as_mut().unwrap(),
        size: 14.0,
        color: self.color_background_contrast,
      },
      FontVariant::DefaultForegroundBold => FontConfig {
        font: self.font_primary_bold.as_mut().unwrap(),
        size: 14.0,
        color: self.color_background_contrast,
      },
      FontVariant::Custom { color, size } => FontConfig {
        font: self.font_primary.as_mut().unwrap(),
        size,
        color,
      },
    }
  }
}

pub struct FontConfig<'a> {
  pub font: &'a mut Font,
  pub size: f32,
  pub color: Color,
}
