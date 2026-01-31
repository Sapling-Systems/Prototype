#[derive(Debug, Clone)]
pub enum CompiledConstraint {
  /// Forces an variable to equal a constant value.
  /// var = constant
  ForcedConstAssignment {
    variable: ConstraintVariable,
    constant: f32,
  },
  /// Forces an variable to equal another variable plus a constant offset.
  /// var = other_var + offset
  ForcedVariableAssignment {
    target_variable: ConstraintVariable,
    source_variable: ConstraintVariable,
    constant_offset: f32,
  },
  /// Forces an variable to equal the maximum value of a list of variables plus a constant offset.
  /// var = max(source_variables) + offset
  ForcedVariableAssignmentMaxOf {
    target_variable: ConstraintVariable,
    source_variables: Vec<ConstraintVariable>,
    constant_offset: f32,
  },
  /// Forces an variable to equal the sum of a list of terms (variable and constant multiplicator)
  /// var = sum(source_variables * multiplicator) + offset
  ForcedVariableAssignmentTerms {
    target_variable: ConstraintVariable,
    source_variables: Vec<(ConstraintVariable, f32)>,
    constant_offset: f32,
  },
  /// Tries to assume variable from child variable, this is only resolved if cycles can be prevented
  /// self.dimension = max(children.dimension)
  TryAssumeMaxChildSize {
    dimension: Dimension,
    constant_offset: f32,
  },
  /// Tries to assume variable from child variable, this is only resolved if cycles can be prevented
  /// self.dimension = max(parent.dimension)
  TryAssumeParentSize {
    dimension: Dimension,
    constant_offset: f32,
  },
}

impl CompiledConstraint {
  pub(crate) fn is_constant(&self) -> bool {
    match self {
      CompiledConstraint::ForcedConstAssignment { .. } => true,
      _ => false,
    }
  }

  pub(crate) fn get_explicit_target(&self) -> Option<ConstraintVariable> {
    match self {
      CompiledConstraint::ForcedVariableAssignment {
        target_variable, ..
      } => Some(*target_variable),
      CompiledConstraint::ForcedConstAssignment { variable, .. } => Some(*variable),
      CompiledConstraint::ForcedVariableAssignmentMaxOf {
        target_variable, ..
      } => Some(*target_variable),
      CompiledConstraint::ForcedVariableAssignmentTerms {
        target_variable, ..
      } => Some(*target_variable),
      _ => None,
    }
  }

  pub(crate) fn get_explicit_mentioned_variables(&self) -> Vec<ConstraintVariable> {
    match self {
      CompiledConstraint::ForcedConstAssignment { variable, .. } => vec![*variable],
      CompiledConstraint::ForcedVariableAssignment {
        source_variable,
        target_variable,
        ..
      } => vec![*source_variable, *target_variable],
      CompiledConstraint::ForcedVariableAssignmentMaxOf {
        target_variable,
        source_variables,
        ..
      } => {
        let mut dependencies = vec![*target_variable];
        dependencies.extend(source_variables.iter().map(|v| *v));
        dependencies
      }
      CompiledConstraint::ForcedVariableAssignmentTerms {
        target_variable,
        source_variables,
        ..
      } => {
        let mut dependencies = vec![*target_variable];
        dependencies.extend(source_variables.iter().map(|(v, _)| *v));
        dependencies
      }
      _ => vec![],
    }
  }

  pub fn get_formular(&self) -> String {
    let mut formular = String::new();

    match self {
      CompiledConstraint::ForcedConstAssignment { variable, constant } => {
        formular.push_str(&format!("{} = {}", variable.formular_name(), constant));
      }
      CompiledConstraint::ForcedVariableAssignmentMaxOf {
        target_variable,
        source_variables,
        ..
      } => {
        formular.push_str(&format!("{} = max(", target_variable.formular_name()));
        formular.push_str(
          &source_variables
            .iter()
            .map(|v| format!("{}", v.formular_name()))
            .collect::<Vec<String>>()
            .join(", "),
        );
        formular.push(')');
      }
      CompiledConstraint::ForcedVariableAssignmentTerms {
        target_variable,
        source_variables,
        ..
      } => {
        formular.push_str(&format!("{} = ", target_variable.formular_name()));
        formular.push_str(
          &source_variables
            .iter()
            .map(|(v, term)| format!("({} * {})", term, v.formular_name()))
            .collect::<Vec<String>>()
            .join(" + "),
        );
      }
      CompiledConstraint::TryAssumeMaxChildSize {
        dimension,
        constant_offset,
      } => {
        formular.push_str(&format!(
          "{} = max(child[i]:{}){}",
          match dimension {
            Dimension::Width => "self_width",
            Dimension::Height => "self_height",
          },
          match dimension {
            Dimension::Width => "self_width",
            Dimension::Height => "self_height",
          },
          if *constant_offset != 0.0f32 {
            format!(" + {}", constant_offset)
          } else {
            "".to_string()
          },
        ));
      }
      CompiledConstraint::ForcedVariableAssignment {
        target_variable,
        source_variable,
        constant_offset,
      } => {
        formular.push_str(&format!(
          "{} = {}{}",
          target_variable.formular_name(),
          source_variable.formular_name(),
          if *constant_offset != 0.0f32 {
            format!(" + {}", constant_offset)
          } else {
            "".to_string()
          },
        ));
      }
      _ => {}
    }

    formular
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConstraintVariable {
  WindowWidth,
  WindowHeight,
  SelfWidth,
  SelfHeight,
  SelfX,
  SelfY,
  ParentWidth,
  ParentHeight,
  ParentX,
  ParentY,
  ElementWidth { id: usize },
  ElementHeight { id: usize },
  ElementX { id: usize },
  ElementY { id: usize },
}

impl ConstraintVariable {
  pub fn formular_name(&self) -> String {
    match self {
      ConstraintVariable::WindowWidth => "window_width".to_string(),
      ConstraintVariable::WindowHeight => "window_height".to_string(),
      ConstraintVariable::SelfWidth => "self_width".to_string(),
      ConstraintVariable::SelfHeight => "self_height".to_string(),
      ConstraintVariable::SelfX => "self_x".to_string(),
      ConstraintVariable::SelfY => "self_y".to_string(),
      ConstraintVariable::ParentWidth => "parent_width".to_string(),
      ConstraintVariable::ParentHeight => "parent_height".to_string(),
      ConstraintVariable::ParentX => "parent_x".to_string(),
      ConstraintVariable::ParentY => "parent_y".to_string(),
      ConstraintVariable::ElementWidth { id } => format!("${}:width", id),
      ConstraintVariable::ElementHeight { id } => format!("${}:height", id),
      ConstraintVariable::ElementX { id } => format!("${}:x", id),
      ConstraintVariable::ElementY { id } => format!("${}:y", id),
    }
  }
}

pub enum ElementVariable {
  Width,
  Height,
  X,
  Y,
}

#[derive(Clone)]
pub struct UserElementConstraints {
  pub constraints: Vec<CompiledConstraint>,
}

impl UserElementConstraints {
  pub fn merged(&self, other: &UserElementConstraints) -> UserElementConstraints {
    UserElementConstraints {
      constraints: self
        .constraints
        .iter()
        .chain(other.constraints.iter())
        .cloned()
        .collect(),
    }
  }
}

#[derive(Debug, Clone)]
pub struct UserElementConstraint {
  pub operator: UserElementConstraintOperator,
  pub expression: UserElementConstraintExpression,
  pub strength: f32,
}

#[derive(Debug, Clone)]
pub struct UserElementConstraintExpression {
  pub constant: f32,
  pub terms: Vec<UserElementConstraintTerm>,
}

#[derive(Debug, Clone)]
pub struct UserElementConstraintTerm {
  pub variable: ConstraintVariable,
  pub coefficient: f32,
}

#[derive(Debug, Clone)]
pub enum UserElementConstraintOperator {
  Equal,
  GreaterOrEqual,
  LessOrEqual,
}

#[derive(Copy, Clone, Debug)]
pub enum Dimension {
  Width,
  Height,
}
