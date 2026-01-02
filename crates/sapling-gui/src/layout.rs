use sapling_gui_macro::constraint1;

#[derive(Debug)]
pub struct ResolvedLayout {
  pub width: f32,
  pub height: f32,
  pub x: f32,
  pub y: f32,
}

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
        constraint1!(self_left == parent_top),
        constraint1!(self_top == parent_top),
      ],
    }
  }

  pub fn fixed_size(width: f32, height: f32) -> Self {
    ElementConstraints {
      constraints: vec![
        constraint1!(self_right == parent_left + width),
        constraint1!(self_bottom == parent_top + height),
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
}

pub struct ElementConstraint {
  pub operator: ElementConstraintOperator,
  pub expression: ElementConstraintExpression,
  pub strength: f32,
}

pub struct ElementConstraintExpression {
  pub constant: f32,
  pub terms: Vec<ElementConstraintTerm>,
}

pub struct ElementConstraintTerm {
  pub variable: ElementConstraintVariable,
  pub coefficient: f32,
}

pub enum ElementConstraintOperator {
  Equal,
  GreaterOrEqual,
  LessOrEqual,
}

pub enum ElementConstraintVariable {
  ParentLeft,
  ParentRight,
  ParentTop,
  ParentBottom,
  SelfLeft,
  SelfRight,
  SelfTop,
  SelfBottom,
}
