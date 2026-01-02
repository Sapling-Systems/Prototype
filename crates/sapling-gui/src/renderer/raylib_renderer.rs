use raylib::{
  color::Color,
  math::{Rectangle, Vector4},
  prelude::RaylibDrawHandle,
};

use crate::renderer::{
  Renderer,
  raylib_util::{CornerRadii, draw_round_rect_per_corner},
};

pub struct RaylibRenderer<'runtime> {
  draw: RaylibDrawHandle<'runtime>,
}

impl<'runtime> RaylibRenderer<'runtime> {
  pub fn new(draw: RaylibDrawHandle<'runtime>) -> Self {
    Self { draw }
  }

  pub fn end(self) -> RaylibDrawHandle<'runtime> {
    self.draw
  }
}

impl<'runtime> Renderer for RaylibRenderer<'runtime> {
  fn draw_rectangle(&mut self, rect: Rectangle, radii: Vector4, fill: Color) {
    draw_round_rect_per_corner(
      &mut self.draw,
      rect,
      CornerRadii {
        tl: radii.x,
        tr: radii.y,
        br: radii.z,
        bl: radii.w,
      },
      12,
      true,
      0.0,
      fill,
    );
  }

  fn draw_rectangle_border(
    &mut self,
    rect: Rectangle,
    radii: Vector4,
    border: Color,
    thickness: f32,
  ) {
    draw_round_rect_per_corner(
      &mut self.draw,
      rect,
      CornerRadii {
        tl: radii.x,
        tr: radii.y,
        br: radii.z,
        bl: radii.w,
      },
      12,
      false,
      thickness,
      border,
    );
  }

  fn draw_text(
    &mut self,
    font: &mut crate::font::Font,
    text: &str,
    position: raylib::prelude::Vector2,
    font_size: f32,
    color: Color,
  ) {
    font.draw_text(&mut self.draw, text, position, font_size, color);
  }
}
