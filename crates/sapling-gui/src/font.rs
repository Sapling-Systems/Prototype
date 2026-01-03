use std::ffi::CString;

use anyhow::{Result, anyhow};
use raylib::{
  RaylibHandle, RaylibThread,
  color::Color,
  ffi::{self, Font as RaylibFFIFont, FontType, MeasureTextEx, Rectangle, Texture},
  math::Vector2,
  prelude::{RaylibDraw, RaylibDrawHandle, RaylibShaderModeExt},
  shaders::Shader,
  text::{RSliceGlyphInfo, gen_image_font_atlas},
  texture::{RaylibTexture2D, Texture2D},
};

// TODO: This is hardcoded to raylib renderer
pub struct Font {
  shader: Shader,
  glyph_info: RSliceGlyphInfo,
  rectangles: Vec<Rectangle>,
  texture: Texture2D,
}

impl Font {
  const FONT_SIZE: i32 = 32;
  const FONT_GLYPHS: &'static str = "!\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHI\nJKLMNOPQRSTUVWXYZ[]^_`abcdefghijklmn\nopqrstuvwxyz{|}~";

  pub fn new(raylib: &mut RaylibHandle, thread: &RaylibThread, font_path: &str) -> Result<Self> {
    let base_font_size = Self::FONT_SIZE;
    let base_glyphs = Self::FONT_GLYPHS;

    let data = std::fs::read(font_path)?;
    let mut glyph_info = raylib
      .load_font_data(
        &data,
        base_font_size,
        Some(base_glyphs),
        FontType::FONT_SDF as i32,
      )
      .ok_or_else(|| anyhow!("Failed to generate sdf font"))?;

    let glyph_info_slice_ffi =
      unsafe { std::mem::transmute::<_, &mut [ffi::GlyphInfo]>(glyph_info.as_mut().as_mut()) };

    //let glyph_info_raw = glyph_info[0].clone();
    let (image, rectangles) = gen_image_font_atlas(
      thread,
      glyph_info_slice_ffi,
      Self::FONT_GLYPHS.len() as i32,
      0,
      1,
    );

    let texture = raylib.load_texture_from_image(thread, &image)?;
    texture.set_texture_filter(thread, ffi::TextureFilter::TEXTURE_FILTER_BILINEAR);

    let shader = raylib.load_shader_from_memory(
      thread,
      None,
      Some(include_str!("../../../apps/ide/resources/shaders/sdf.fs")),
    );

    Ok(Self {
      shader,
      glyph_info,
      rectangles,
      texture,
    })
  }

  pub fn calculate_text_size(&mut self, text: &str, font_size: f32) -> Vector2 {
    let glyph_info_slice_ffi = unsafe {
      std::mem::transmute::<_, &mut ffi::GlyphInfo>(&mut self.glyph_info.as_mut().as_mut()[0])
    };

    let font = RaylibFFIFont {
      baseSize: Self::FONT_SIZE,
      glyphCount: Self::FONT_GLYPHS.len() as i32,
      glyphPadding: 0,
      glyphs: glyph_info_slice_ffi as _,
      recs: self.rectangles.as_mut_ptr(),
      texture: Texture {
        format: self.texture.format,
        height: self.texture.height,
        width: self.texture.width,
        mipmaps: self.texture.mipmaps,
        id: self.texture.id,
      },
    };

    let c_text = CString::new(text).unwrap();
    let output = unsafe { MeasureTextEx(font, c_text.as_ptr(), font_size, 0.0) };
    Vector2 {
      x: output.x,
      y: output.y,
    }
  }

  pub fn draw_text<T: RaylibDraw>(
    &mut self,
    draw: &mut T,
    text: &str,
    position: Vector2,
    font_size: f32,
    color: Color,
  ) {
    // FFI is horrible in raylib-rs, is this correct?
    let glyph_info_slice_ffi = unsafe {
      std::mem::transmute::<_, &mut ffi::GlyphInfo>(&mut self.glyph_info.as_mut().as_mut()[0])
    };

    let font = RaylibFFIFont {
      baseSize: Self::FONT_SIZE,
      glyphCount: Self::FONT_GLYPHS.len() as i32,
      glyphPadding: 0,
      glyphs: glyph_info_slice_ffi as _,
      recs: self.rectangles.as_mut_ptr(),
      texture: Texture {
        format: self.texture.format,
        height: self.texture.height,
        width: self.texture.width,
        mipmaps: self.texture.mipmaps,
        id: self.texture.id,
      },
    };

    let mut shader_mode = draw.begin_shader_mode(&mut self.shader);
    shader_mode.draw_text_ex(FontWrapper(font), text, position, font_size, 0.0, color);
  }
}

struct FontWrapper(RaylibFFIFont);

impl AsRef<RaylibFFIFont> for FontWrapper {
  fn as_ref(&self) -> &RaylibFFIFont {
    &self.0
  }
}
