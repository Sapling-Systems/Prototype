use raylib::{
  color::Color,
  math::{Rectangle, Vector2, Vector4},
};

pub trait Renderer {
  fn draw_text(
    &mut self,
    _font: &mut Font,
    _text: &str,
    _position: Vector2,
    _font_size: f32,
    _color: Color,
  ) {
  }
  fn draw_rectangle(&mut self, _rect: Rectangle, _radii: Vector4, _fill: Color) {}
  fn draw_rectangle_border(
    &mut self,
    _rect: Rectangle,
    _radii: Vector4,
    _border: Color,
    _thickness: f32,
  ) {
  }
}

pub struct NoopRenderer;

impl Renderer for NoopRenderer {}

mod raylib_renderer;
mod raylib_util;

pub use raylib_renderer::RaylibRenderer;

use crate::font::Font;
