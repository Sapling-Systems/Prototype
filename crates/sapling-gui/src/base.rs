use std::{
  any::Any,
  arch::x86_64::_MM_EXCEPT_INEXACT,
  hash::{DefaultHasher, Hash, Hasher},
  marker::PhantomData,
};

use kasuari::Strength;
use raylib::{
  color::Color,
  ffi::KeyboardKey,
  math::{Rectangle, Vector2, Vector4},
};
use sapling_gui_macro::constraint1;

use crate::{
  component::Component,
  prelude::{ElementContext, RenderContext, RenderFilter, StatefulContext},
  theme::FontVariant,
};

/// Utility view that serves as a container for other views, does not render anything itself.
#[derive(Debug, Default)]
pub struct LayoutView;

impl Component for LayoutView {}

/// General purpose view that renders a stylable element supporting basic shapes.
#[derive(Debug)]
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
    self.style.background_color = Some(color);
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

  pub fn with_drop_shadow(mut self, shadow: DropShadowStyle) -> Self {
    self.style.drop_shadow = Some(shadow);
    self
  }

  pub fn with_border(mut self, width: f32, color: Color) -> Self {
    self.style.border_width = width;
    self.style.border_color = color;
    self
  }
}

impl Component for StyledView {
  fn render(&self, context: &mut RenderContext) {
    let radii = Vector4::new(
      self.style.border_radius.0,
      self.style.border_radius.1,
      self.style.border_radius.2,
      self.style.border_radius.3,
    );

    if let Some(shadow) = &self.style.drop_shadow {
      let layout = context.layout.clone();
      let shadow = shadow.clone();
      context.renderer.draw_with_filter(
        RenderFilter::Blur {
          amount: shadow.blur_radius,
        },
        Box::new(move |mut renderer| {
          renderer.draw_rectangle(
            Rectangle {
              x: layout.x + shadow.offset.x,
              y: layout.y + shadow.offset.y,
              width: layout.width,
              height: layout.height,
            },
            radii,
            shadow.color,
          );
        }),
      );
    }

    if self.style.border_width > 0.0 {
      context.renderer.draw_rectangle_border(
        Rectangle {
          x: context.layout.x,
          y: context.layout.y,
          height: context.layout.height,
          width: context.layout.width,
        },
        radii,
        self.style.border_color,
        self.style.border_width,
      );
    }

    if let Some(background_color) = self.style.background_color {
      context.renderer.draw_rectangle(
        Rectangle {
          x: context.layout.x + self.style.border_width / 2.0,
          y: context.layout.y + self.style.border_width / 2.0,
          height: context.layout.height - self.style.border_width,
          width: context.layout.width - self.style.border_width,
        },
        inner_radii(
          radii,
          self.style.border_width,
          self.style.border_width <= 0.0,
        ),
        background_color,
      );
    }
  }
}

#[derive(Debug)]
pub struct ViewStyle {
  background_color: Option<Color>,
  border_radius: (f32, f32, f32, f32),
  border_width: f32,
  border_color: Color,
  drop_shadow: Option<DropShadowStyle>,
}

#[derive(Clone, Debug)]
pub struct DropShadowStyle {
  pub offset: Vector2,
  pub color: Color,
  pub blur_radius: f32,
}

impl Default for ViewStyle {
  fn default() -> Self {
    ViewStyle {
      background_color: None,
      border_radius: (0.0, 0.0, 0.0, 0.0),
      border_width: 0.0,
      border_color: Color::BLACK,
      drop_shadow: None,
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

#[derive(Debug)]
pub struct TextView {
  variant: FontVariant,
  text: String,
  horizontal_alignment: TextHorizontalAlignment,
  vertical_alignment: TextVerticalAlignment,
  auto_size: bool,
  line_height: f32,
}

impl TextView {
  pub fn new(variant: FontVariant, text: String) -> Self {
    TextView {
      variant,
      text,
      horizontal_alignment: TextHorizontalAlignment::Left,
      vertical_alignment: TextVerticalAlignment::Top,
      auto_size: true,
      line_height: 1.0,
    }
  }

  pub fn with_line_height(mut self, line_height: f32) -> Self {
    self.line_height = line_height;
    self
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
  fn construct(&mut self, context: &mut ElementContext) {
    let font_config = context.theme.text_config(self.variant);
    let mut expected_size = font_config
      .font
      .calculate_text_size(&self.text, font_config.size);
    expected_size.y *= self.line_height;

    let grow_width = self.horizontal_alignment == TextHorizontalAlignment::Left && self.auto_size;
    let grow_height = self.vertical_alignment == TextVerticalAlignment::Top && self.auto_size;

    let mut constraints = vec![];
    if grow_width {
      constraints.push(constraint1!(
        self_width == expected_size.x,
        strength = Strength::STRONG.value() as f32
      ));
    }
    if grow_height {
      constraints.push(constraint1!(
        self_height == expected_size.y,
        strength = Strength::STRONG.value() as f32
      ));
    }

    context.set_parent_element_constraints(constraints);
  }

  fn render(&self, context: &mut RenderContext) {
    let font_config = context.theme.text_config(self.variant);
    let expected_size = font_config
      .font
      .calculate_text_size(&self.text, font_config.size);

    let x = match self.horizontal_alignment {
      TextHorizontalAlignment::Left => context.layout.x,
      TextHorizontalAlignment::Center => {
        (context.layout.width - expected_size.x) / 2.0 + context.layout.x
      }
      TextHorizontalAlignment::Right => context.layout.width - expected_size.x + context.layout.x,
    };

    let y = match self.vertical_alignment {
      TextVerticalAlignment::Top => context.layout.y,
      TextVerticalAlignment::Center => {
        (context.layout.height - expected_size.y) / 2.0 + context.layout.y
      }
      TextVerticalAlignment::Bottom => context.layout.height - expected_size.y + context.layout.y,
    };

    context.renderer.draw_text(
      font_config.font,
      &self.text,
      Vector2::new(x, y),
      font_config.size,
      font_config.color,
    );
  }
}

#[derive(Debug, PartialEq, Eq)]
pub enum TextHorizontalAlignment {
  Left,
  Center,
  Right,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TextVerticalAlignment {
  Top,
  Center,
  Bottom,
}

#[derive(Debug)]
pub struct FormattedTextView {
  texts: Vec<TextFormat>,
  auto_size: bool,
  vertical_alignment: TextVerticalAlignment,
  horizontal_alignment: TextHorizontalAlignment,
}

impl FormattedTextView {
  pub fn new() -> Self {
    Self {
      texts: Vec::new(),
      vertical_alignment: TextVerticalAlignment::Top,
      horizontal_alignment: TextHorizontalAlignment::Left,
      auto_size: true,
    }
  }

  pub fn add_text(mut self, variant: FontVariant, text: String) -> Self {
    self.texts.push(TextFormat::new(variant, text));
    self
  }

  pub fn with_vertical_alignment(mut self, alignment: TextVerticalAlignment) -> Self {
    self.vertical_alignment = alignment;
    self
  }

  pub fn with_horizontal_alignment(mut self, alignment: TextHorizontalAlignment) -> Self {
    self.horizontal_alignment = alignment;
    self
  }
}

impl Component for FormattedTextView {
  fn construct(&mut self, context: &mut ElementContext) {
    let expected_sizes = self
      .texts
      .iter()
      .enumerate()
      .map(|(index, text)| {
        let font_config = context.theme.text_config(text.variant);
        let with_space = index > 0;
        font_config.font.calculate_text_size(
          &format!("{}{}", if with_space { " " } else { "" }, text.text),
          font_config.size,
        )
      })
      .collect::<Vec<_>>();

    let expected_size = expected_sizes
      .iter()
      .cloned()
      .reduce(|a, b| Vector2::new(a.x + b.x, a.y.max(b.y)))
      .unwrap_or_default();

    let grow_width = self.horizontal_alignment == TextHorizontalAlignment::Left && self.auto_size;
    let grow_height = self.vertical_alignment == TextVerticalAlignment::Top && self.auto_size;

    let mut constraints = vec![];
    if grow_width {
      constraints.push(constraint1!(
        self_width == expected_size.x,
        strength = Strength::STRONG.value() as f32
      ));
    }
    if grow_height {
      constraints.push(constraint1!(
        self_height == expected_size.y,
        strength = Strength::STRONG.value() as f32
      ));
    }
    context.set_parent_element_constraints(constraints);
  }

  fn render(&self, context: &mut RenderContext) {
    let expected_sizes = self
      .texts
      .iter()
      .enumerate()
      .map(|(index, text)| {
        let font_config = context.theme.text_config(text.variant);
        let with_space = index > 0;
        font_config.font.calculate_text_size(
          &format!("{}{}", if with_space { " " } else { "" }, text.text),
          font_config.size,
        )
      })
      .collect::<Vec<_>>();

    let expected_size = expected_sizes
      .iter()
      .cloned()
      .reduce(|a, b| Vector2::new(a.x + b.x, a.y.max(b.y)))
      .unwrap_or_default();

    let mut base_x = match self.horizontal_alignment {
      TextHorizontalAlignment::Left => context.layout.x,
      TextHorizontalAlignment::Center => {
        (context.layout.width - expected_size.x) / 2.0 + context.layout.x
      }
      TextHorizontalAlignment::Right => context.layout.width - expected_size.x + context.layout.x,
    };

    let base_y = match self.vertical_alignment {
      TextVerticalAlignment::Top => context.layout.y,
      TextVerticalAlignment::Center => {
        (context.layout.height - expected_size.y) / 2.0 + context.layout.y
      }
      TextVerticalAlignment::Bottom => context.layout.height - expected_size.y + context.layout.y,
    };

    for (index, (text, size)) in self.texts.iter().zip(expected_sizes).enumerate() {
      let with_space = index > 0;
      let font_config = context.theme.text_config(text.variant);
      let draw_text = &format!("{}{}", if with_space { " " } else { "" }, text.text);
      context.renderer.draw_text(
        font_config.font,
        draw_text,
        Vector2::new(base_x, base_y),
        font_config.size,
        font_config.color,
      );
      base_x += size.x;
    }
  }
}

#[derive(Debug)]
pub struct TextFormat {
  variant: FontVariant,
  text: String,
}

impl TextFormat {
  pub fn new(variant: FontVariant, text: String) -> Self {
    Self { variant, text }
  }
}

pub struct Pressable {
  on_press: Box<dyn Fn(&mut RenderContext) + 'static>,
}

impl Pressable {
  pub fn new<F: Fn(&mut RenderContext) + 'static>(on_press: F) -> Self {
    Self {
      on_press: Box::new(on_press),
    }
  }
}

impl std::fmt::Debug for Pressable {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Pressable").finish()
  }
}

impl Component for Pressable {
  fn render(&self, context: &mut RenderContext) {
    if let Some(pressed_location) = context.input_state.mouse_press {
      if pressed_location.x >= context.layout.x
        && pressed_location.y >= context.layout.y
        && pressed_location.x < context.layout.x + context.layout.width
        && pressed_location.y < context.layout.y + context.layout.height
      {
        (*self.on_press)(context);
      }
    }
  }
}

pub struct MutableState<T: Any + Clone + 'static> {
  name: &'static str,
  element_id: usize,
  _p: PhantomData<T>,
}

// Manually implement Clone and Copy for all T, since MutableState doesn't actually store T
impl<T: Any + Clone + 'static> Clone for MutableState<T> {
  fn clone(&self) -> Self {
    *self
  }
}

impl<T: Any + Clone + 'static> Copy for MutableState<T> {}

impl<T: Any + Clone + 'static> MutableState<T> {
  pub fn new<FInit: FnOnce() -> T>(
    context: &mut ElementContext,
    initializer: FInit,
    name: &'static str,
  ) -> (T, Self) {
    let value = context.prepare_and_get_state(initializer, name);
    (
      value,
      Self {
        name,
        element_id: context.current_element_id(),
        _p: PhantomData,
      },
    )
  }

  pub fn set_direct(&self, context: &mut impl StatefulContext, new_value: T) {
    context.set_state(self.element_id, self.name, new_value);
  }
}

pub struct FocusableInteractiveView {
  action_handlers: Vec<(u64, Box<dyn FnOnce(&mut ElementContext)>)>,
}

impl std::fmt::Debug for FocusableInteractiveView {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("FocusableInteractiveView").finish()
  }
}

impl FocusableInteractiveView {
  pub fn new() -> Self {
    Self {
      action_handlers: Vec::new(),
    }
  }

  pub fn with_action_handler<F: FnOnce(&mut ElementContext) + 'static>(
    mut self,
    action: impl Hash,
    handler: F,
  ) -> Self {
    let hash = {
      let mut hasher = DefaultHasher::new();
      action.hash(&mut hasher);
      hasher.finish()
    };
    self.action_handlers.push((hash, Box::new(handler)));
    self
  }
}

impl Component for FocusableInteractiveView {
  fn construct(&mut self, context: &mut ElementContext) {
    for (hash, handler) in self.action_handlers.drain(..) {
      if context.input_state.is_action_pressed(hash) {
        handler(context);
      }
    }
  }
}
