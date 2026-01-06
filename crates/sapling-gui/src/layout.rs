use sapling_gui_macro::constraint1;

use crate::prelude::Element;

#[derive(Debug, Clone)]
pub struct ResolvedLayout {
  pub width: f32,
  pub height: f32,
  pub x: f32,
  pub y: f32,
}

#[derive(Debug, Clone)]
pub struct ElementConstraints {
  pub constraints: Vec<ElementConstraint>,
}

impl ElementConstraints {
  pub const REQUIRED: f32 = 1_001_001_000.0;
  pub const WEAK: f32 = 1.0;

  pub fn weak(mut self) -> Self {
    for constraint in &mut self.constraints {
      constraint.strength = Self::WEAK
    }
    self
  }

  pub fn absolute_position(x: f32, y: f32) -> Self {
    ElementConstraints {
      constraints: vec![constraint1!(self_x == x), constraint1!(self_y == y)],
    }
  }

  pub fn absolute_top(spacing: f32) -> Self {
    ElementConstraints {
      constraints: vec![constraint1!(self_y == spacing)],
    }
  }

  pub fn absolute_right(spacing: f32) -> Self {
    ElementConstraints {
      constraints: vec![constraint1!(self_x + self_width == screen_width - spacing)],
    }
  }

  pub fn cover_parent_padding(
    padding_left: f32,
    padding_top: f32,
    padding_right: f32,
    padding_bottom: f32,
  ) -> Self {
    ElementConstraints {
      constraints: vec![
        constraint1!(self_x == parent_x + padding_left),
        constraint1!(self_y == parent_y + padding_top),
        constraint1!(self_width == parent_width - padding_left - padding_right),
        constraint1!(self_height == parent_height - padding_top - padding_bottom),
      ],
    }
  }

  pub fn cover_parent_horizontal(padding: f32) -> Self {
    ElementConstraints {
      constraints: vec![
        constraint1!(self_x == parent_x + padding),
        constraint1!(self_width == parent_width - padding - padding),
      ],
    }
  }

  pub fn cover_parent_vertical(padding: f32) -> Self {
    ElementConstraints {
      constraints: vec![
        constraint1!(self_y == parent_y + padding),
        constraint1!(self_height == parent_height - padding - padding),
      ],
    }
  }

  pub fn cover_parent_even_padding(padding: f32) -> Self {
    ElementConstraints {
      constraints: vec![
        constraint1!(self_x == parent_x + padding),
        constraint1!(self_y == parent_y + padding),
        constraint1!(self_width == parent_width - padding - padding),
        constraint1!(self_height == parent_height - padding - padding),
      ],
    }
  }

  pub fn relative_position() -> Self {
    ElementConstraints {
      constraints: vec![
        constraint1!(self_x == parent_x),
        constraint1!(self_y == parent_y),
      ],
    }
  }

  pub fn relative_top(spacing: f32) -> Self {
    ElementConstraints {
      constraints: vec![constraint1!(self_y == parent_y + spacing)],
    }
  }

  pub fn relative_left(spacing: f32) -> Self {
    ElementConstraints {
      constraints: vec![constraint1!(self_x == parent_x + spacing)],
    }
  }

  pub fn center() -> Self {
    ElementConstraints {
      constraints: vec![
        constraint1!(self_x + self_width / 2.0 == parent_x + parent_width / 2.0),
        constraint1!(self_y + self_height / 2.0 == parent_y + parent_height / 2.0),
      ],
    }
  }

  pub fn fixed_size(width: f32, height: f32) -> Self {
    ElementConstraints {
      constraints: vec![
        constraint1!(self_width == width),
        constraint1!(self_height == height),
      ],
    }
  }

  pub fn fixed_width(width: f32) -> Self {
    ElementConstraints {
      constraints: vec![constraint1!(self_width == width)],
    }
  }

  pub fn fixed_height(height: f32) -> Self {
    ElementConstraints {
      constraints: vec![constraint1!(self_height == height)],
    }
  }

  pub fn cover_parent() -> Self {
    ElementConstraints {
      constraints: vec![
        constraint1!(self_x == parent_x),
        constraint1!(self_y == parent_y),
        constraint1!(self_width == parent_width),
        constraint1!(self_height == parent_height),
      ],
    }
  }

  pub fn anchor_to_right_of(element: Element, spacing: f32) -> Self {
    ElementConstraints {
      constraints: vec![constraint1!(
        self_x == element.x() + element.width() + spacing
      )],
    }
  }

  pub fn anchor_to_top_of(element: Element, spacing: f32) -> Self {
    ElementConstraints {
      constraints: vec![constraint1!(self_y == element.y() + spacing)],
    }
  }

  pub fn anchor_to_bottom_of(element: Element, spacing: f32) -> Self {
    ElementConstraints {
      constraints: vec![constraint1!(
        self_y == element.y() + element.height() + spacing
      )],
    }
  }
}

#[derive(Debug, Clone)]
pub struct ElementConstraint {
  pub operator: ElementConstraintOperator,
  pub expression: ElementConstraintExpression,
  pub strength: f32,
}

#[derive(Debug, Clone)]
pub struct ElementConstraintExpression {
  pub constant: f32,
  pub terms: Vec<ElementConstraintTerm>,
}

#[derive(Debug, Clone)]
pub struct ElementConstraintTerm {
  pub variable: ElementConstraintVariable,
  pub coefficient: f32,
}

#[derive(Debug, Clone)]
pub enum ElementConstraintOperator {
  Equal,
  GreaterOrEqual,
  LessOrEqual,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum ElementConstraintVariable {
  ScreenWidth,
  ScreenHeight,
  ParentX,
  ParentY,
  ParentWidth,
  ParentHeight,
  SelfX,
  SelfY,
  SelfWidth,
  SelfHeight,
  ElementX(Element),
  ElementY(Element),
  ElementWidth(Element),
  ElementHeight(Element),
}

/// Trait for types that can be converted to constraint terms.
/// This allows both constants (f32) and variables (ElementConstraintVariable)
/// to be used in constraint expressions.
pub trait IntoConstraintTerm {
  fn into_constraint_term(self) -> ConstraintTermValue;
}

pub enum ConstraintTermValue {
  Constant(f32),
  Variable(ElementConstraintVariable),
}

impl IntoConstraintTerm for f32 {
  fn into_constraint_term(self) -> ConstraintTermValue {
    ConstraintTermValue::Constant(self)
  }
}

impl IntoConstraintTerm for ElementConstraintVariable {
  fn into_constraint_term(self) -> ConstraintTermValue {
    ConstraintTermValue::Variable(self)
  }
}
