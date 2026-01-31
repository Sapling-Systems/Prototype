use super::constraint::{CompiledConstraint, UserElementConstraint, UserElementConstraintOperator};

/// Represents the result of compiling a constraint, with a maximum of 2 compiled constraints.
/// Most user constraints compile to 0 or 1 compiled constraints.
pub struct CompiledConstraintResult {
  pub constraints: [Option<CompiledConstraint>; 2],
  pub count: usize,
}

impl CompiledConstraintResult {
  const fn empty() -> Self {
    Self {
      constraints: [None, None],
      count: 0,
    }
  }

  const fn single(constraint: CompiledConstraint) -> Self {
    Self {
      constraints: [Some(constraint), None],
      count: 1,
    }
  }

  pub fn to_vec(self) -> Vec<CompiledConstraint> {
    let mut result = Vec::with_capacity(self.count);
    let mut i = 0;
    while i < self.count {
      if let Some(constraint) = &self.constraints[i] {
        result.push(constraint.clone());
      }
      i += 1;
    }
    result
  }
}

/// Compile-time optimizer that converts user-defined constraints into optimized compiled constraints.
///
/// This optimizer analyzes constraint expressions and transforms them into direct assignment
/// operations when possible, avoiding the overhead of the full cassowary constraint solver.
///
/// # Optimization Patterns
///
/// 1. **Constant Assignment**: `var == constant` → `ForcedConstAssignment`
///    - Example: `x + 10 == 0` → `x = -10`
///
/// 2. **Variable Assignment**: `var1 == var2 + constant` → `ForcedVariableAssignment`
///    - Example: `x - y - 5 == 0` → `x = y + 5`
///
/// # Compilation Rules
///
/// - Only `Equal` operator constraints can be compiled (for now)
/// - `GreaterOrEqual` and `LessOrEqual` cannot be compiled to forced assignments
/// - Constraints with more than 2 terms (excluding constant) cannot be compiled
/// - The constraint must be solvable for exactly one variable
pub const fn optimize_constraint(constraint: &UserElementConstraint) -> CompiledConstraintResult {
  // Only Equal operator can be compiled into forced assignments
  // Inequalities (>= and <=) cannot be directly compiled to assignments
  match constraint.operator {
    UserElementConstraintOperator::Equal => {}
    UserElementConstraintOperator::GreaterOrEqual => {
      panic!(
        "Cannot compile GreaterOrEqual constraint into CompiledConstraint. \
                Inequality constraints require a full constraint solver."
      );
    }
    UserElementConstraintOperator::LessOrEqual => {
      panic!(
        "Cannot compile LessOrEqual constraint into CompiledConstraint. \
                Inequality constraints require a full constraint solver."
      );
    }
  }

  let expr = &constraint.expression;

  // Access terms as a slice to get const-compatible length
  let terms_slice = expr.terms.as_slice();
  let num_terms = terms_slice.len();

  match num_terms {
    0 => {
      // Expression like: constant == 0
      // This is a tautology or contradiction
      if expr.constant > f32::EPSILON || expr.constant < -f32::EPSILON {
        panic!("Constraint is unsatisfiable: constant == 0 but constant != 0");
      }
      // Empty constraint (0 == 0), return empty result
      CompiledConstraintResult::empty()
    }
    1 => {
      // Expression like: coeff * var + constant == 0
      // Solve for var: var = -constant / coeff
      compile_single_term_constraint(terms_slice, expr.constant)
    }
    2 => {
      // Expression like: coeff1 * var1 + coeff2 * var2 + constant == 0
      // We can solve for one variable in terms of the other
      compile_two_term_constraint(terms_slice, expr.constant)
    }
    _ => {
      // Expression with 3+ terms cannot be compiled to simple assignments
      panic!(
        "Cannot compile constraint with 3+ terms into CompiledConstraint. \
                Only constraints with 1-2 variable terms can be compiled."
      );
    }
  }
}

/// Compile a constraint with a single variable term.
///
/// Pattern: `coeff * var + constant == 0`
/// Solves to: `var = -constant / coeff`
const fn compile_single_term_constraint(
  terms: &[super::constraint::UserElementConstraintTerm],
  constant: f32,
) -> CompiledConstraintResult {
  let term = &terms[0];

  if term.coefficient > -f32::EPSILON && term.coefficient < f32::EPSILON {
    panic!("Cannot compile constraint with zero coefficient");
  }

  // Solve: coeff * var + constant == 0
  // => var = -constant / coeff
  let value = -constant / term.coefficient;

  CompiledConstraintResult::single(CompiledConstraint::ForcedConstAssignment {
    variable: term.variable,
    constant: value,
  })
}

/// Compile a constraint with two variable terms.
///
/// Pattern: `coeff1 * var1 + coeff2 * var2 + constant == 0`
///
/// We choose which variable to solve for based on a heuristic:
/// - Prefer solving for variables with coefficient magnitude closest to 1.0
/// - This minimizes floating point error in the division
const fn compile_two_term_constraint(
  terms: &[super::constraint::UserElementConstraintTerm],
  constant: f32,
) -> CompiledConstraintResult {
  let term1 = &terms[0];
  let term2 = &terms[1];

  // Check for zero coefficients
  if term1.coefficient > -f32::EPSILON && term1.coefficient < f32::EPSILON {
    panic!("Cannot compile constraint with zero coefficient on first term");
  }
  if term2.coefficient > -f32::EPSILON && term2.coefficient < f32::EPSILON {
    panic!("Cannot compile constraint with zero coefficient on second term");
  }

  // Choose which variable to solve for based on coefficient magnitude
  // Prefer solving for the variable with coefficient closest to 1.0 or -1.0
  let coeff1_abs = if term1.coefficient < 0.0 {
    -term1.coefficient
  } else {
    term1.coefficient
  };
  let coeff2_abs = if term2.coefficient < 0.0 {
    -term2.coefficient
  } else {
    term2.coefficient
  };

  let coeff1_distance = if coeff1_abs > 1.0 {
    coeff1_abs - 1.0
  } else {
    1.0 - coeff1_abs
  };
  let coeff2_distance = if coeff2_abs > 1.0 {
    coeff2_abs - 1.0
  } else {
    1.0 - coeff2_abs
  };

  let (target_var, target_coeff, source_var, source_coeff) = if coeff1_distance <= coeff2_distance {
    // Solve for var1
    (
      term1.variable,
      term1.coefficient,
      term2.variable,
      term2.coefficient,
    )
  } else {
    // Solve for var2
    (
      term2.variable,
      term2.coefficient,
      term1.variable,
      term1.coefficient,
    )
  };

  // Solve: target_coeff * target_var + source_coeff * source_var + constant == 0
  // => target_var = (-source_coeff / target_coeff) * source_var + (-constant / target_coeff)
  let source_multiplier = -source_coeff / target_coeff;
  let constant_offset = -constant / target_coeff;

  // Check if source_multiplier is 1.0 or -1.0 (or very close to it)
  let is_one = source_multiplier > 0.999 && source_multiplier < 1.001;
  let is_neg_one = source_multiplier > -1.001 && source_multiplier < -0.999;

  if is_one || is_neg_one {
    // Simple case: target_var = ±source_var + constant_offset
    CompiledConstraintResult::single(CompiledConstraint::ForcedVariableAssignment {
      target_variable: target_var,
      source_variable: source_var,
      constant_offset,
    })
  } else {
    // General case: target_var = source_multiplier * source_var + constant_offset
    // This requires multiplication which ForcedVariableAssignment doesn't support
    panic!(
      "Cannot compile constraint where the source variable has a coefficient other than 1.0 or -1.0. \
            The current CompiledConstraint variants only support assignments of the form 'var = other_var + constant'."
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::layout::constraint::{
    ConstraintVariable, UserElementConstraintExpression, UserElementConstraintOperator,
    UserElementConstraintTerm,
  };

  #[test]
  fn test_single_term_constant_assignment() {
    // x + 10 == 0 => x = -10
    let constraint = UserElementConstraint {
      operator: UserElementConstraintOperator::Equal,
      expression: UserElementConstraintExpression {
        constant: 10.0,
        terms: vec![UserElementConstraintTerm {
          variable: ConstraintVariable::SelfX,
          coefficient: 1.0,
        }],
      },
      strength: 1.0,
    };

    let result = optimize_constraint(&constraint);
    assert_eq!(result.count, 1);
    match result.constraints[0] {
      Some(CompiledConstraint::ForcedConstAssignment { variable, constant }) => {
        assert_eq!(variable, ConstraintVariable::SelfX);
        assert!((constant + 10.0).abs() < f32::EPSILON);
      }
      _ => panic!("Expected ForcedConstAssignment"),
    }
  }

  #[test]
  fn test_two_term_variable_assignment() {
    // x - y - 5 == 0 => x = y + 5
    let constraint = UserElementConstraint {
      operator: UserElementConstraintOperator::Equal,
      expression: UserElementConstraintExpression {
        constant: -5.0,
        terms: vec![
          UserElementConstraintTerm {
            variable: ConstraintVariable::SelfX,
            coefficient: 1.0,
          },
          UserElementConstraintTerm {
            variable: ConstraintVariable::SelfY,
            coefficient: -1.0,
          },
        ],
      },
      strength: 1.0,
    };

    let result = optimize_constraint(&constraint);
    assert_eq!(result.count, 1);
    match result.constraints[0] {
      Some(CompiledConstraint::ForcedVariableAssignment {
        target_variable,
        source_variable,
        constant_offset,
      }) => {
        assert_eq!(target_variable, ConstraintVariable::SelfX);
        assert_eq!(source_variable, ConstraintVariable::SelfY);
        assert!((constant_offset - 5.0).abs() < f32::EPSILON);
      }
      _ => panic!("Expected ForcedVariableAssignment"),
    }
  }

  #[test]
  #[should_panic(expected = "Cannot compile constraint where the source variable")]
  fn test_scaled_coefficient() {
    // 2x - y == 0 => x = 0.5y (not supported)
    let constraint = UserElementConstraint {
      operator: UserElementConstraintOperator::Equal,
      expression: UserElementConstraintExpression {
        constant: 0.0,
        terms: vec![
          UserElementConstraintTerm {
            variable: ConstraintVariable::SelfX,
            coefficient: 2.0,
          },
          UserElementConstraintTerm {
            variable: ConstraintVariable::SelfY,
            coefficient: -1.0,
          },
        ],
      },
      strength: 1.0,
    };

    optimize_constraint(&constraint);
  }

  #[test]
  #[should_panic(expected = "Cannot compile GreaterOrEqual constraint")]
  fn test_inequality_panics() {
    let constraint = UserElementConstraint {
      operator: UserElementConstraintOperator::GreaterOrEqual,
      expression: UserElementConstraintExpression {
        constant: 0.0,
        terms: vec![UserElementConstraintTerm {
          variable: ConstraintVariable::SelfX,
          coefficient: 1.0,
        }],
      },
      strength: 1.0,
    };

    optimize_constraint(&constraint);
  }

  #[test]
  #[should_panic(expected = "Cannot compile constraint with 3+ terms")]
  fn test_too_many_terms_panics() {
    let constraint = UserElementConstraint {
      operator: UserElementConstraintOperator::Equal,
      expression: UserElementConstraintExpression {
        constant: 0.0,
        terms: vec![
          UserElementConstraintTerm {
            variable: ConstraintVariable::SelfX,
            coefficient: 1.0,
          },
          UserElementConstraintTerm {
            variable: ConstraintVariable::SelfY,
            coefficient: 1.0,
          },
          UserElementConstraintTerm {
            variable: ConstraintVariable::SelfWidth,
            coefficient: 1.0,
          },
        ],
      },
      strength: 1.0,
    };

    optimize_constraint(&constraint);
  }

  #[test]
  fn test_empty_constraint() {
    // 0 == 0 (tautology)
    let constraint = UserElementConstraint {
      operator: UserElementConstraintOperator::Equal,
      expression: UserElementConstraintExpression {
        constant: 0.0,
        terms: vec![],
      },
      strength: 1.0,
    };

    let result = optimize_constraint(&constraint);
    assert_eq!(result.count, 0);
  }

  #[test]
  fn test_negative_coefficient_assignment() {
    // -x + 5 == 0 => x = 5
    let constraint = UserElementConstraint {
      operator: UserElementConstraintOperator::Equal,
      expression: UserElementConstraintExpression {
        constant: 5.0,
        terms: vec![UserElementConstraintTerm {
          variable: ConstraintVariable::SelfWidth,
          coefficient: -1.0,
        }],
      },
      strength: 1.0,
    };

    let result = optimize_constraint(&constraint);
    assert_eq!(result.count, 1);
    match result.constraints[0] {
      Some(CompiledConstraint::ForcedConstAssignment { variable, constant }) => {
        assert_eq!(variable, ConstraintVariable::SelfWidth);
        assert!((constant - 5.0).abs() < f32::EPSILON);
      }
      _ => panic!("Expected ForcedConstAssignment"),
    }
  }
}
