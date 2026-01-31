use crate::{
  layout::constraint::{CompiledConstraint, ConstraintVariable, UserElementConstraints},
  prelude::Element,
};

impl UserElementConstraints {
  pub fn absolute_position(x: f32, y: f32) -> Self {
    Self {
      constraints: vec![
        CompiledConstraint::ForcedConstAssignment {
          variable: ConstraintVariable::SelfX,
          constant: x,
        },
        CompiledConstraint::ForcedConstAssignment {
          variable: ConstraintVariable::SelfY,
          constant: y,
        },
      ],
    }
  }

  pub fn floating_top_right(spacing_x: f32, spacing_y: f32, width: f32, height: f32) -> Self {
    Self {
      constraints: vec![
        CompiledConstraint::ForcedVariableAssignment {
          target_variable: ConstraintVariable::SelfX,
          source_variable: ConstraintVariable::WindowWidth,
          constant_offset: -width - spacing_x,
        },
        CompiledConstraint::ForcedConstAssignment {
          variable: ConstraintVariable::SelfY,
          constant: spacing_y,
        },
        CompiledConstraint::ForcedConstAssignment {
          variable: ConstraintVariable::SelfWidth,
          constant: width,
        },
        CompiledConstraint::ForcedConstAssignment {
          variable: ConstraintVariable::SelfHeight,
          constant: height,
        },
      ],
    }
  }

  pub fn relative_to_parent_horizontal(x: f32) -> Self {
    Self {
      constraints: vec![CompiledConstraint::ForcedVariableAssignment {
        target_variable: ConstraintVariable::SelfX,
        source_variable: ConstraintVariable::ParentX,
        constant_offset: x,
      }],
    }
  }

  pub fn relative_to_parent_vertical(y: f32) -> Self {
    Self {
      constraints: vec![CompiledConstraint::ForcedVariableAssignment {
        target_variable: ConstraintVariable::SelfY,
        source_variable: ConstraintVariable::ParentY,
        constant_offset: y,
      }],
    }
  }

  pub fn relative_to_parent(x: f32, y: f32) -> Self {
    Self::relative_to_parent_horizontal(x).merged(&Self::relative_to_parent_vertical(y))
  }

  pub fn cover_parent_horizontal(spacing_x: f32) -> Self {
    Self {
      constraints: vec![CompiledConstraint::ForcedVariableAssignment {
        target_variable: ConstraintVariable::SelfWidth,
        source_variable: ConstraintVariable::ParentWidth,
        constant_offset: spacing_x,
      }],
    }
  }

  pub fn cover_parent_vertical(spacing_y: f32) -> Self {
    Self {
      constraints: vec![CompiledConstraint::ForcedVariableAssignment {
        target_variable: ConstraintVariable::SelfHeight,
        source_variable: ConstraintVariable::ParentHeight,
        constant_offset: spacing_y,
      }],
    }
  }

  pub fn width_of_element(element: Element) -> Self {
    Self {
      constraints: vec![CompiledConstraint::ForcedVariableAssignment {
        target_variable: ConstraintVariable::SelfWidth,
        source_variable: ConstraintVariable::ElementWidth { id: element.id },
        constant_offset: 0.0,
      }],
    }
  }

  pub fn height_of_element(element: Element) -> Self {
    Self {
      constraints: vec![CompiledConstraint::ForcedVariableAssignment {
        target_variable: ConstraintVariable::SelfHeight,
        source_variable: ConstraintVariable::ElementHeight { id: element.id },
        constant_offset: 0.0,
      }],
    }
  }

  pub fn cover_parent(spacing_x: f32, spacing_y: f32) -> Self {
    Self::cover_parent_horizontal(spacing_x).merged(&Self::cover_parent_vertical(spacing_y))
  }

  pub fn cover_element_horizontal(element: Element, spacing_x: f32) -> Self {
    Self {
      constraints: vec![
        CompiledConstraint::ForcedVariableAssignment {
          target_variable: ConstraintVariable::SelfX,
          source_variable: ConstraintVariable::ElementX { id: element.id },
          constant_offset: spacing_x,
        },
        CompiledConstraint::ForcedVariableAssignment {
          target_variable: ConstraintVariable::SelfWidth,
          source_variable: ConstraintVariable::ElementWidth { id: element.id },
          constant_offset: spacing_x.abs() * 2.0,
        },
      ],
    }
  }

  pub fn cover_element_vertical(element: Element, spacing_y: f32) -> Self {
    Self {
      constraints: vec![
        CompiledConstraint::ForcedVariableAssignment {
          target_variable: ConstraintVariable::SelfY,
          source_variable: ConstraintVariable::ElementY { id: element.id },
          constant_offset: spacing_y,
        },
        CompiledConstraint::ForcedVariableAssignment {
          target_variable: ConstraintVariable::SelfHeight,
          source_variable: ConstraintVariable::ElementHeight { id: element.id },
          constant_offset: spacing_y.abs() * 2.0,
        },
      ],
    }
  }

  pub fn cover_element(element: Element, spacing_x: f32, spacing_y: f32) -> Self {
    Self::cover_element_horizontal(element, spacing_x)
      .merged(&Self::cover_element_vertical(element, spacing_y))
  }

  pub fn center_in_parent_horizontal() -> Self {
    Self {
      constraints: vec![CompiledConstraint::ForcedVariableAssignmentTerms {
        target_variable: ConstraintVariable::SelfX,
        source_variables: vec![
          (ConstraintVariable::ParentWidth, 0.5),
          (ConstraintVariable::SelfWidth, -0.5),
        ],
        constant_offset: 0.0,
      }],
    }
  }

  pub fn center_in_parent_vertical() -> Self {
    Self {
      constraints: vec![CompiledConstraint::ForcedVariableAssignmentTerms {
        target_variable: ConstraintVariable::SelfY,
        source_variables: vec![
          (ConstraintVariable::ParentHeight, 0.5),
          (ConstraintVariable::SelfHeight, -0.5),
        ],
        constant_offset: 0.0,
      }],
    }
  }

  pub fn center_in_parent() -> Self {
    Self::center_in_parent_horizontal().merged(&Self::center_in_parent_vertical())
  }

  pub fn fixed_width(width: f32) -> Self {
    Self {
      constraints: vec![CompiledConstraint::ForcedConstAssignment {
        variable: ConstraintVariable::SelfWidth,
        constant: width,
      }],
    }
  }

  pub fn fixed_height(height: f32) -> Self {
    Self {
      constraints: vec![CompiledConstraint::ForcedConstAssignment {
        variable: ConstraintVariable::SelfHeight,
        constant: height,
      }],
    }
  }

  pub fn fixed_size(width: f32, height: f32) -> Self {
    Self::fixed_height(height).merged(&Self::fixed_width(width))
  }

  pub fn anchor_to_right_of(element: Element, spacing: f32) -> Self {
    Self {
      constraints: vec![CompiledConstraint::ForcedVariableAssignmentTerms {
        target_variable: ConstraintVariable::SelfX,
        source_variables: vec![
          (ConstraintVariable::ElementX { id: element.id }, 1.0),
          (ConstraintVariable::ElementWidth { id: element.id }, 1.0),
        ],
        constant_offset: spacing,
      }],
    }
  }

  pub fn anchor_to_top_of(element: Element, spacing: f32) -> Self {
    Self {
      constraints: vec![CompiledConstraint::ForcedVariableAssignment {
        target_variable: ConstraintVariable::SelfY,
        source_variable: ConstraintVariable::ElementY { id: element.id },
        constant_offset: spacing,
      }],
    }
  }

  pub fn anchor_to_bottom_of(element: Element, spacing: f32) -> Self {
    Self {
      constraints: vec![CompiledConstraint::ForcedVariableAssignmentTerms {
        target_variable: ConstraintVariable::SelfY,
        source_variables: vec![
          (ConstraintVariable::ElementY { id: element.id }, 1.0),
          (ConstraintVariable::ElementHeight { id: element.id }, 1.0),
        ],
        constant_offset: spacing,
      }],
    }
  }

  pub fn scale_to_bottom_of(element: Element, spacing: f32) -> Self {
    Self {
      constraints: vec![CompiledConstraint::ForcedVariableAssignmentTerms {
        target_variable: ConstraintVariable::SelfHeight,
        source_variables: vec![
          (ConstraintVariable::ElementY { id: element.id }, 1.0),
          (ConstraintVariable::ElementHeight { id: element.id }, 1.0),
          (ConstraintVariable::SelfY, -1.0),
        ],
        constant_offset: spacing,
      }],
    }
  }
}
