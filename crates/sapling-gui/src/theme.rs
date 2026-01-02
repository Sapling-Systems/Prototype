use raylib::{RaylibHandle, RaylibThread, color::Color};

use crate::font::Font;

pub struct Theme {
  pub primary_font: Option<Font>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontVariant {
  Primary,
}

impl Theme {
  pub fn new(rl: &mut RaylibHandle, thread: &RaylibThread) -> Self {
    let primary_font =
      Font::new(rl, thread, "./apps/ide/assets/fonts/FiraCode-Medium.ttf").unwrap();

    Self {
      primary_font: Some(primary_font),
    }
  }

  pub fn mock() -> Self {
    Self { primary_font: None }
  }

  pub fn text_config<'a>(&'a mut self, variant: FontVariant) -> FontConfig<'a> {
    match variant {
      FontVariant::Primary => FontConfig {
        font: self.primary_font.as_mut().unwrap(),
        size: 20.0,
        color: Color::WHITE,
      },
    }
  }
}

pub struct FontConfig<'a> {
  pub font: &'a mut Font,
  pub size: f32,
  pub color: Color,
}
