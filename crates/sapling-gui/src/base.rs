use raylib::{
  color::Color,
  math::{Rectangle, Vector2, Vector4},
};

use crate::{
  component::Component,
  layout::ResolvedLayout,
  prelude::Renderer,
  theme::{FontVariant, Theme},
};

/// Utility view that serves as a container for other views, does not render anything itself.
pub struct LayoutView;

impl Component for LayoutView {}

/// General purpose view that renders a stylable element supporting basic shapes.
pub struct StyledView {
  style: ViewStyle,
}

impl StyledView {
  pub fn new() -> Self {
    StyledView {
      style: ViewStyle::default(),
    }
  }

  pub fn with_background_color(mut self, color: Color) -> Self {
    self.style.background_color = color;
    self
  }

  pub fn with_border_radius_even(mut self, radius: f32) -> Self {
    self.style.border_radius = (radius, radius, radius, radius);
    self
  }

  pub fn with_border_radius(
    mut self,
    top_left: f32,
    top_right: f32,
    bottom_right: f32,
    bottom_left: f32,
  ) -> Self {
    self.style.border_radius = (top_left, top_right, bottom_right, bottom_left);
    self
  }

  pub fn with_border(mut self, width: f32, color: Color) -> Self {
    self.style.border_width = width;
    self.style.border_color = color;
    self
  }
}

impl Component for StyledView {
  fn render(&self, layout: &ResolvedLayout, renderer: &mut dyn Renderer, _theme: &mut Theme) {
    if self.style.border_width > 0.0 {
      renderer.draw_rectangle_border(
        Rectangle {
          x: layout.x,
          y: layout.y,
          height: layout.height,
          width: layout.width,
        },
        Vector4::new(
          self.style.border_radius.0,
          self.style.border_radius.1,
          self.style.border_radius.2,
          self.style.border_radius.3,
        ),
        self.style.border_color,
        self.style.border_width,
      );
    }

    renderer.draw_rectangle(
      Rectangle {
        x: layout.x + self.style.border_width / 2.0,
        y: layout.y + self.style.border_width / 2.0,
        height: layout.height - self.style.border_width,
        width: layout.width - self.style.border_width,
      },
      inner_radii(
        Vector4::new(
          self.style.border_radius.0,
          self.style.border_radius.1,
          self.style.border_radius.2,
          self.style.border_radius.3,
        ),
        self.style.border_width,
        self.style.border_width <= 0.0,
      ),
      self.style.background_color,
    );
  }
}

pub struct ViewStyle {
  background_color: Color,
  border_radius: (f32, f32, f32, f32),
  border_width: f32,
  border_color: Color,
}

impl Default for ViewStyle {
  fn default() -> Self {
    ViewStyle {
      background_color: Color::WHITE,
      border_radius: (0.0, 0.0, 0.0, 0.0),
      border_width: 0.0,
      border_color: Color::BLACK,
    }
  }
}

fn inner_radii(outer: Vector4, border_thickness: f32, inside_only: bool) -> Vector4 {
  let k = if inside_only { 1.0 } else { 0.5 };
  let d = border_thickness * k;
  Vector4 {
    x: (outer.x - d).max(0.0),
    y: (outer.y - d).max(0.0),
    z: (outer.z - d).max(0.0),
    w: (outer.w - d).max(0.0),
  }
}

pub struct TextView {
  variant: FontVariant,
  text: String,
  horizontal_alignment: TextHorizontalAlignment,
  vertical_alignment: TextVerticalAlignment,
}

impl TextView {
  pub fn new(variant: FontVariant, text: String) -> Self {
    TextView {
      variant,
      text,
      horizontal_alignment: TextHorizontalAlignment::Left,
      vertical_alignment: TextVerticalAlignment::Top,
    }
  }

  pub fn with_horizontal_alignment(mut self, alignment: TextHorizontalAlignment) -> Self {
    self.horizontal_alignment = alignment;
    self
  }

  pub fn with_vertical_alignment(mut self, alignment: TextVerticalAlignment) -> Self {
    self.vertical_alignment = alignment;
    self
  }
}

impl Component for TextView {
  fn render(&self, layout: &ResolvedLayout, renderer: &mut dyn Renderer, theme: &mut Theme) {
    let font_config = theme.text_config(self.variant);
    let expected_size = font_config
      .font
      .calculate_text_size(&self.text, font_config.size);

    let x = match self.horizontal_alignment {
      TextHorizontalAlignment::Left => layout.x,
      TextHorizontalAlignment::Center => (layout.width - expected_size.x) / 2.0 + layout.x,
      TextHorizontalAlignment::Right => layout.width - expected_size.x + layout.x,
    };

    let y = match self.vertical_alignment {
      TextVerticalAlignment::Top => layout.y,
      TextVerticalAlignment::Center => (layout.height - expected_size.y) / 2.0 + layout.y,
      TextVerticalAlignment::Bottom => layout.height - expected_size.y + layout.y,
    };

    renderer.draw_text(
      font_config.font,
      &self.text,
      Vector2::new(x, y),
      font_config.size,
      font_config.color,
    );
  }
}

pub enum TextHorizontalAlignment {
  Left,
  Center,
  Right,
}

pub enum TextVerticalAlignment {
  Top,
  Center,
  Bottom,
}
