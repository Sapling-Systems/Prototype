use raylib::{RaylibHandle, RaylibThread, color::Color, math::Vector2};

use crate::{base::DropShadowStyle, font::Font};

pub struct Theme {
  pub font_primary: Option<Font>,
  pub font_primary_bold: Option<Font>,
  pub color_primary: Color,
  pub color_background: Color,
  pub color_background_contrast: Color,
  pub radius_default: f32,
  pub spacing_tiny: f32,
  pub spacing_small: f32,
  pub spacing_default: f32,
  pub spacing_large: f32,
  pub spacing_xlarge: f32,
  pub drop_shadow_default: DropShadowStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontVariant {
  Primary,
  DefaultForeground,
  DefaultForegroundBold,
}

impl Theme {
  pub fn new(rl: &mut RaylibHandle, thread: &RaylibThread) -> Self {
    let primary_font =
      Font::new(rl, thread, "./apps/ide/assets/fonts/FiraCode-Medium.ttf").unwrap();

    let primary_font_bold =
      Font::new(rl, thread, "./apps/ide/assets/fonts/FiraCode-Bold.ttf").unwrap();

    Self {
      font_primary: Some(primary_font),
      font_primary_bold: Some(primary_font_bold),
      color_primary: Color::from_hex("16A085").unwrap(),
      color_background: Color::from_hex("212121").unwrap(),
      color_background_contrast: Color::from_hex("F4F4F4").unwrap(),
      spacing_tiny: 2.0,
      spacing_small: 4.0,
      spacing_default: 8.0,
      spacing_large: 16.0,
      spacing_xlarge: 32.0,
      radius_default: 8.0,
      drop_shadow_default: DropShadowStyle {
        color: Color::BLACK.alpha(0.55),
        offset: Vector2::new(2.0, 4.0),
        blur_radius: 8.0,
      },
    }
  }

  pub fn mock() -> Self {
    Self {
      font_primary: None,
      font_primary_bold: None,
      color_primary: Color::WHITE,
      color_background: Color::BLACK,
      color_background_contrast: Color::BLACK,
      spacing_tiny: 2.0,
      spacing_small: 4.0,
      spacing_default: 8.0,
      spacing_large: 16.0,
      spacing_xlarge: 32.0,
      radius_default: 8.0,
      drop_shadow_default: DropShadowStyle {
        color: Color::BLACK.alpha(0.55),
        offset: Vector2::new(2.0, 4.0),
        blur_radius: 8.0,
      },
    }
  }

  pub fn text_config<'a>(&'a mut self, variant: FontVariant) -> FontConfig<'a> {
    match variant {
      FontVariant::Primary => FontConfig {
        font: self.font_primary.as_mut().unwrap(),
        size: 20.0,
        color: Color::WHITE,
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
    }
  }
}

pub struct FontConfig<'a> {
  pub font: &'a mut Font,
  pub size: f32,
  pub color: Color,
}
