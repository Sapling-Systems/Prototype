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
      constraints: vec![constraint1!(self_left == x), constraint1!(self_top == y)],
    }
  }

  pub fn absolute_top(spacing: f32) -> Self {
    ElementConstraints {
      constraints: vec![constraint1!(self_top == spacing)],
    }
  }

  pub fn absolute_right(spacing: f32) -> Self {
    ElementConstraints {
      constraints: vec![constraint1!(self_right == screen_width - spacing)],
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
        constraint1!(self_right == parent_right - padding_right),
        constraint1!(self_bottom == parent_bottom - padding_bottom),
        constraint1!(self_left == parent_left + padding_left),
        constraint1!(self_top == parent_top + padding_top),
      ],
    }
  }

  pub fn cover_parent_horizontal(padding: f32) -> Self {
    ElementConstraints {
      constraints: vec![
        constraint1!(self_right == parent_right - padding),
        constraint1!(self_left == parent_left + padding),
      ],
    }
  }

  pub fn cover_parent_vertical(padding: f32) -> Self {
    ElementConstraints {
      constraints: vec![
        constraint1!(self_bottom == parent_bottom - padding),
        constraint1!(self_top == parent_top + padding),
      ],
    }
  }

  pub fn cover_parent_even_padding(padding: f32) -> Self {
    ElementConstraints {
      constraints: vec![
        constraint1!(self_right == parent_right - padding),
        constraint1!(self_bottom == parent_bottom - padding),
        constraint1!(self_left == parent_left + padding),
        constraint1!(self_top == parent_top + padding),
      ],
    }
  }

  pub fn relative_position() -> Self {
    ElementConstraints {
      constraints: vec![
        constraint1!(self_left == parent_left),
        constraint1!(self_top == parent_top),
      ],
    }
  }

  pub fn relative_top(spacing: f32) -> Self {
    ElementConstraints {
      constraints: vec![constraint1!(self_top == parent_top + spacing)],
    }
  }

  pub fn relative_left(spacing: f32) -> Self {
    ElementConstraints {
      constraints: vec![constraint1!(self_left == parent_left + spacing)],
    }
  }

  pub fn center() -> Self {
    ElementConstraints {
      constraints: vec![
        constraint1!(self_left + self_right == parent_left + parent_right),
        constraint1!(self_top + self_bottom == parent_top + parent_bottom),
      ],
    }
  }

  pub fn fixed_size(width: f32, height: f32) -> Self {
    ElementConstraints {
      constraints: vec![
        constraint1!(self_right - self_left == width),
        constraint1!(self_bottom - self_top == height),
      ],
    }
  }

  pub fn cover_parent() -> Self {
    ElementConstraints {
      constraints: vec![
        constraint1!(self_right == parent_right),
        constraint1!(self_bottom == parent_bottom),
        constraint1!(self_left == parent_left),
        constraint1!(self_top == parent_top),
      ],
    }
  }

  pub fn anchor_to_right_of(element: Element, spacing: f32) -> Self {
    ElementConstraints {
      constraints: vec![constraint1!(self_left == element.right() + spacing)],
    }
  }

  pub fn anchor_to_top_of(element: Element, spacing: f32) -> Self {
    ElementConstraints {
      constraints: vec![constraint1!(self_top == element.top() + spacing)],
    }
  }

  pub fn anchor_to_bottom_of(element: Element, spacing: f32) -> Self {
    ElementConstraints {
      constraints: vec![constraint1!(self_bottom == element.bottom() + spacing)],
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
  ParentLeft,
  ParentRight,
  ParentTop,
  ParentBottom,
  SelfLeft,
  SelfRight,
  SelfTop,
  SelfBottom,
  ElementLeft(Element),
  ElementRight(Element),
  ElementTop(Element),
  ElementBottom(Element),
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
