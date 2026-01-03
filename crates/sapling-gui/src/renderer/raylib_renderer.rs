use std::ops::DerefMut;

use raylib::{
  RaylibHandle, RaylibThread,
  color::Color,
  math::{Rectangle, Vector2, Vector4},
  prelude::{RaylibDraw, RaylibShaderModeExt, RaylibTextureModeExt},
  shaders::{RaylibShader, Shader},
  texture::RenderTexture2D,
};

use crate::{
  prelude::RenderFilter,
  renderer::{
    Renderer,
    raylib_util::{CornerRadii, draw_round_rect_per_corner},
  },
};

pub struct RaylibRendererState {
  render_texture: RenderTexture2D,
  blur_shader: Shader,
  blur_shader_location_radius: i32,
  blur_shader_location_render_width: i32,
}

impl RaylibRendererState {
  pub fn new(handle: &mut RaylibHandle, thread: &RaylibThread) -> Self {
    let width = handle.get_render_width();
    let height = handle.get_render_height();
    let render_texture = handle
      .load_render_texture(thread, width as u32, height as u32)
      .unwrap();

    let blur_shader = handle.load_shader_from_memory(
      thread,
      None,
      Some(include_str!(
        "../../../../apps/ide/resources/shaders/blur.fs"
      )),
    );
    let blur_shader_location_render_width = blur_shader.get_shader_location("renderWidth");
    let blur_shader_location_radius = blur_shader.get_shader_location("blurRadius");

    Self {
      render_texture,
      blur_shader,
      blur_shader_location_radius,
      blur_shader_location_render_width,
    }
  }
}

pub struct RaylibRenderer<'state, THandle: RaylibDraw> {
  draw: THandle,
  thread: RaylibThread,
  state: Option<&'state mut RaylibRendererState>,
}

impl<'state, THandle: RaylibDraw + DerefMut<Target = RaylibHandle>>
  RaylibRenderer<'state, THandle>
{
  pub fn new(draw: THandle, state: &'state mut RaylibRendererState, thread: RaylibThread) -> Self {
    Self {
      draw,
      state: Some(state),
      thread,
    }
  }

  pub fn end(self) -> THandle {
    self.draw
  }

  fn prepare_render_texture(&mut self) {
    let height = self.draw.get_render_height();
    let width = self.draw.get_render_width();
    if self.state.as_ref().unwrap().render_texture.texture.height != height
      || self.state.as_ref().unwrap().render_texture.texture.width != width
    {
      self.state.as_mut().unwrap().render_texture = self
        .draw
        .load_render_texture(&self.thread, width as u32, height as u32)
        .unwrap();
    }
  }
}

impl<'state, THandle: RaylibDraw + DerefMut<Target = RaylibHandle>> Renderer
  for RaylibRenderer<'state, THandle>
{
  fn draw_with_filter(
    &mut self,
    ty: RenderFilter,
    filter: Box<dyn for<'a> FnOnce(Box<dyn Renderer + 'a>)>,
  ) {
    self.prepare_render_texture();

    {
      let thread = self.thread.clone();
      let mut texture_mode = self.draw.begin_texture_mode(
        &self.thread,
        &mut self.state.as_mut().unwrap().render_texture,
      );

      texture_mode.clear_background(Color::WHITE.alpha(0.0));

      let renderer: Box<dyn Renderer> = Box::new(RaylibRenderer {
        draw: texture_mode,
        state: None,
        thread,
      });

      filter(renderer);
    }

    let width = self.draw.get_render_width();
    let height = self.draw.get_render_height();

    match ty {
      RenderFilter::Blur { amount } => {
        let state = self.state.as_mut().unwrap();

        state
          .blur_shader
          .set_shader_value_v(state.blur_shader_location_render_width, &[width as f32]);
        state
          .blur_shader
          .set_shader_value_v(state.blur_shader_location_radius, &[amount]);

        let mut shader_mode = self.draw.begin_shader_mode(&mut state.blur_shader);
        shader_mode.draw_texture_rec(
          &state.render_texture,
          Rectangle {
            x: 0.0,
            y: 0.0,
            width: width as f32,
            height: -(height as f32), // Flip vertically for render texture
          },
          Vector2::new(0.0, 0.0),
          Color::WHITE,
        );
      }
    }
  }

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
