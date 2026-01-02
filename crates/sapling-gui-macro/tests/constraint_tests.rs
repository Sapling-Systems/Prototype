use sapling_gui::prelude::*;
use sapling_gui_macro::{constraint, constraint1};

#[test]
fn test_simple_equality() {
  // Test: parent_left == self_left
  // Expected: parent_left - self_left == 0
  let result = constraint!(parent_left == self_left);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert!(matches!(c.operator, ElementConstraintOperator::Equal));
  assert_eq!(c.expression.constant, 0.0);
  assert_eq!(c.expression.terms.len(), 2);
  assert_eq!(c.strength, ElementConstraints::REQUIRED);

  // Check terms
  assert!(matches!(
    c.expression.terms[0].variable,
    ElementConstraintVariable::ParentLeft
  ));
  assert_eq!(c.expression.terms[0].coefficient, 1.0);

  assert!(matches!(
    c.expression.terms[1].variable,
    ElementConstraintVariable::SelfLeft
  ));
  assert_eq!(c.expression.terms[1].coefficient, -1.0);
}

#[test]
fn test_inequality_with_constant() {
  // Test: self_right >= parent_right - 10.0
  // Expected: self_right - parent_right + 10.0 >= 0
  let result = constraint!(self_right >= parent_right - 10.0);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert!(matches!(
    c.operator,
    ElementConstraintOperator::GreaterOrEqual
  ));
  assert_eq!(c.expression.constant, 10.0);
  assert_eq!(c.expression.terms.len(), 2);

  assert!(matches!(
    c.expression.terms[0].variable,
    ElementConstraintVariable::SelfRight
  ));
  assert_eq!(c.expression.terms[0].coefficient, 1.0);

  assert!(matches!(
    c.expression.terms[1].variable,
    ElementConstraintVariable::ParentRight
  ));
  assert_eq!(c.expression.terms[1].coefficient, -1.0);
}

#[test]
fn test_less_or_equal() {
  // Test: self_top + 5.0 <= parent_bottom
  // Expected: self_top - parent_bottom + 5.0 <= 0
  let result = constraint!(self_top + 5.0 <= parent_bottom);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert!(matches!(c.operator, ElementConstraintOperator::LessOrEqual));
  assert_eq!(c.expression.constant, 5.0);
  assert_eq!(c.expression.terms.len(), 2);

  assert!(matches!(
    c.expression.terms[0].variable,
    ElementConstraintVariable::SelfTop
  ));
  assert_eq!(c.expression.terms[0].coefficient, 1.0);

  assert!(matches!(
    c.expression.terms[1].variable,
    ElementConstraintVariable::ParentBottom
  ));
  assert_eq!(c.expression.terms[1].coefficient, -1.0);
}

#[test]
fn test_multiplication_by_constant() {
  // Test: self_left * 2.0 == parent_left
  // Expected: 2.0 * self_left - parent_left == 0
  let result = constraint!(self_left * 2.0 == parent_left);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert!(matches!(c.operator, ElementConstraintOperator::Equal));
  assert_eq!(c.expression.constant, 0.0);
  assert_eq!(c.expression.terms.len(), 2);

  assert!(matches!(
    c.expression.terms[0].variable,
    ElementConstraintVariable::SelfLeft
  ));
  assert_eq!(c.expression.terms[0].coefficient, 2.0);

  assert!(matches!(
    c.expression.terms[1].variable,
    ElementConstraintVariable::ParentLeft
  ));
  assert_eq!(c.expression.terms[1].coefficient, -1.0);
}

#[test]
fn test_constant_multiplication_left_side() {
  // Test: 3.0 * parent_top == self_top
  // Expected: 3.0 * parent_top - self_top == 0
  let result = constraint!(3.0 * parent_top == self_top);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert_eq!(c.expression.constant, 0.0);
  assert_eq!(c.expression.terms.len(), 2);

  assert!(matches!(
    c.expression.terms[0].variable,
    ElementConstraintVariable::ParentTop
  ));
  assert_eq!(c.expression.terms[0].coefficient, 3.0);

  assert!(matches!(
    c.expression.terms[1].variable,
    ElementConstraintVariable::SelfTop
  ));
  assert_eq!(c.expression.terms[1].coefficient, -1.0);
}

#[test]
fn test_division_by_constant() {
  // Test: self_right / 2.0 == parent_right
  // Expected: 0.5 * self_right - parent_right == 0
  let result = constraint!(self_right / 2.0 == parent_right);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert_eq!(c.expression.constant, 0.0);
  assert_eq!(c.expression.terms.len(), 2);

  assert!(matches!(
    c.expression.terms[0].variable,
    ElementConstraintVariable::SelfRight
  ));
  assert_eq!(c.expression.terms[0].coefficient, 0.5);

  assert!(matches!(
    c.expression.terms[1].variable,
    ElementConstraintVariable::ParentRight
  ));
  assert_eq!(c.expression.terms[1].coefficient, -1.0);
}

#[test]
fn test_complex_expression_with_parentheses() {
  // Test: (self_left + self_right) * 0.5 == parent_left
  // Expected: 0.5 * self_left + 0.5 * self_right - parent_left == 0
  let result = constraint!((self_left + self_right) * 0.5 == parent_left);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert_eq!(c.expression.constant, 0.0);
  assert_eq!(c.expression.terms.len(), 3);

  // Find terms by variable type
  let self_left_term = c
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::SelfLeft))
    .unwrap();
  assert_eq!(self_left_term.coefficient, 0.5);

  let self_right_term = c
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::SelfRight))
    .unwrap();
  assert_eq!(self_right_term.coefficient, 0.5);

  let parent_left_term = c
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::ParentLeft))
    .unwrap();
  assert_eq!(parent_left_term.coefficient, -1.0);
}

#[test]
fn test_negation() {
  // Test: -self_bottom == parent_bottom
  // Expected: -self_bottom - parent_bottom == 0
  let result = constraint!(-self_bottom == parent_bottom);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert_eq!(c.expression.constant, 0.0);
  assert_eq!(c.expression.terms.len(), 2);

  assert!(matches!(
    c.expression.terms[0].variable,
    ElementConstraintVariable::SelfBottom
  ));
  assert_eq!(c.expression.terms[0].coefficient, -1.0);

  assert!(matches!(
    c.expression.terms[1].variable,
    ElementConstraintVariable::ParentBottom
  ));
  assert_eq!(c.expression.terms[1].coefficient, -1.0);
}

#[test]
fn test_multiple_terms() {
  // Test: self_left + parent_left >= self_right - 10.0
  // Expected: self_left + parent_left - self_right + 10.0 >= 0
  let result = constraint!(self_left + parent_left >= self_right - 10.0);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert!(matches!(
    c.operator,
    ElementConstraintOperator::GreaterOrEqual
  ));
  assert_eq!(c.expression.constant, 10.0);
  assert_eq!(c.expression.terms.len(), 3);

  let self_left_term = c
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::SelfLeft))
    .unwrap();
  assert_eq!(self_left_term.coefficient, 1.0);

  let parent_left_term = c
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::ParentLeft))
    .unwrap();
  assert_eq!(parent_left_term.coefficient, 1.0);

  let self_right_term = c
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::SelfRight))
    .unwrap();
  assert_eq!(self_right_term.coefficient, -1.0);
}

// ============================================================================
// constraint1! macro tests
// ============================================================================

#[test]
fn test_constraint1_returns_single_constraint() {
  // Test that constraint1! returns a single ElementConstraint
  // self_left == parent_left + 10.0
  // Normalized: self_left - parent_left - 10.0 == 0
  let single_constraint = constraint1!(self_left == parent_left + 10.0);

  // Should be able to use it directly
  assert!(matches!(
    single_constraint.operator,
    ElementConstraintOperator::Equal
  ));
  assert_eq!(single_constraint.expression.constant, -10.0);
  assert_eq!(single_constraint.expression.terms.len(), 2);
}

#[test]
fn test_constraint1_in_vec() {
  // Test that we can use constraint1! directly in a vec
  let constraints = ElementConstraints {
    constraints: vec![
      constraint1!(self_top == parent_top),
      constraint1!(
        self_bottom == parent_bottom,
        strength = ElementConstraints::WEAK
      ),
    ],
  };

  assert_eq!(constraints.constraints.len(), 2);
  assert_eq!(
    constraints.constraints[0].strength,
    ElementConstraints::REQUIRED
  );
  assert_eq!(
    constraints.constraints[1].strength,
    ElementConstraints::WEAK
  );
}

#[test]
fn test_constraint1_with_runtime_expressions() {
  // Test that runtime expressions work
  let padding = 16.0;
  let constraints = ElementConstraints::cover_parent_even_padding(padding);

  assert_eq!(constraints.constraints.len(), 4);

  // Test with variables
  let offset_x = 10.0;
  let offset_y = 20.0;
  let constraint = constraint1!(self_left == parent_left + offset_x);
  let constraint2 = constraint1!(self_top == parent_top + offset_y * 2.0);

  // These should compile and be valid constraints
  assert!(matches!(
    constraint.operator,
    ElementConstraintOperator::Equal
  ));
  assert!(matches!(
    constraint2.operator,
    ElementConstraintOperator::Equal
  ));
}

#[test]
fn test_constraint1_complex_runtime_expression() {
  let base_padding = 8.0;
  let multiplier = 2.0;

  let constraint = constraint1!(self_left == parent_left + base_padding * multiplier);

  assert!(matches!(
    constraint.operator,
    ElementConstraintOperator::Equal
  ));
  assert_eq!(constraint.expression.terms.len(), 2);
}

#[test]
fn test_constraint1_with_all_operators() {
  let offset = 5.0;

  let c1 = constraint1!(self_left == parent_left + offset);
  let c2 = constraint1!(self_right >= parent_right - offset);
  let c3 = constraint1!(self_top <= parent_top + offset);

  assert!(matches!(c1.operator, ElementConstraintOperator::Equal));
  assert!(matches!(
    c2.operator,
    ElementConstraintOperator::GreaterOrEqual
  ));
  assert!(matches!(
    c3.operator,
    ElementConstraintOperator::LessOrEqual
  ));
}

#[test]
fn test_complex_constant_expression() {
  // Test: self_top == parent_top - (5.0 + 10.0)
  // Expected: self_top - parent_top + 15.0 == 0
  let result = constraint!(self_top == parent_top - (5.0 + 10.0));

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert_eq!(c.expression.constant, 15.0);
  assert_eq!(c.expression.terms.len(), 2);
}

#[test]
fn test_integer_literals() {
  // Test: self_left + 10 == parent_left
  // Expected: self_left - parent_left + 10.0 == 0
  let result = constraint!(self_left + 10 == parent_left);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert_eq!(c.expression.constant, 10.0);
  assert_eq!(c.expression.terms.len(), 2);
}

#[test]
fn test_custom_strength_literal() {
  // Test: self_left == parent_left, strength = 1.0
  let result = constraint!(self_left == parent_left, strength = 1.0);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert_eq!(c.strength, 1.0);
}

#[test]
fn test_custom_strength_constant() {
  // Test: self_right >= parent_right, strength = ElementConstraints::WEAK
  let result = constraint!(
    self_right >= parent_right,
    strength = ElementConstraints::WEAK
  );

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert_eq!(c.strength, ElementConstraints::WEAK);
}

#[test]
fn test_custom_strength_expression() {
  // Test: self_top <= parent_top, strength = 5.0 * 2.0
  let result = constraint!(self_top <= parent_top, strength = 5.0 * 2.0);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert_eq!(c.strength, 10.0);
}

#[test]
fn test_all_variable_types() {
  // Test each variable type can be used
  let _r1 = constraint!(parent_left == self_left);
  let _r2 = constraint!(parent_right == self_right);
  let _r3 = constraint!(parent_top == self_top);
  let _r4 = constraint!(parent_bottom == self_bottom);
}

#[test]
fn test_right_to_left_movement() {
  // Test: 0.0 == self_left - parent_left
  // Expected: -self_left + parent_left == 0 (or self_left - parent_left == 0 after simplification)
  let result = constraint!(0.0 == self_left - parent_left);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert_eq!(c.expression.constant, 0.0);
  assert_eq!(c.expression.terms.len(), 2);

  // The terms should be negated since we move them from right to left
  let self_left_term = c
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::SelfLeft))
    .unwrap();
  assert_eq!(self_left_term.coefficient, -1.0);

  let parent_left_term = c
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::ParentLeft))
    .unwrap();
  assert_eq!(parent_left_term.coefficient, 1.0);
}

#[test]
fn test_nested_parentheses() {
  // Test: ((self_left + 10.0) - 5.0) == parent_left
  // Expected: self_left - parent_left + 5.0 == 0
  let result = constraint!(((self_left + 10.0) - 5.0) == parent_left);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert_eq!(c.expression.constant, 5.0);
  assert_eq!(c.expression.terms.len(), 2);
}

#[test]
fn test_multiplication_distributive() {
  // Test: 2.0 * (self_left + parent_left) == self_right
  // Expected: 2.0 * self_left + 2.0 * parent_left - self_right == 0
  let result = constraint!(2.0 * (self_left + parent_left) == self_right);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert_eq!(c.expression.constant, 0.0);
  assert_eq!(c.expression.terms.len(), 3);

  let self_left_term = c
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::SelfLeft))
    .unwrap();
  assert_eq!(self_left_term.coefficient, 2.0);

  let parent_left_term = c
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::ParentLeft))
    .unwrap();
  assert_eq!(parent_left_term.coefficient, 2.0);

  let self_right_term = c
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::SelfRight))
    .unwrap();
  assert_eq!(self_right_term.coefficient, -1.0);
}
