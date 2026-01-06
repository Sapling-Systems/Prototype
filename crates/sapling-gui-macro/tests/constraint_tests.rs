use sapling_gui::prelude::*;
use sapling_gui_macro::{constraint, constraint1};

#[test]
fn test_simple_equality() {
  // Test: parent_x == self_x
  // Expected: parent_x - self_x == 0
  let result = constraint!(parent_x == self_x);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert!(matches!(c.operator, ElementConstraintOperator::Equal));
  assert_eq!(c.expression.constant, 0.0);
  assert_eq!(c.expression.terms.len(), 2);
  assert_eq!(c.strength, ElementConstraints::REQUIRED);

  // Check terms
  assert!(matches!(
    c.expression.terms[0].variable,
    ElementConstraintVariable::ParentX
  ));
  assert_eq!(c.expression.terms[0].coefficient, 1.0);

  assert!(matches!(
    c.expression.terms[1].variable,
    ElementConstraintVariable::SelfX
  ));
  assert_eq!(c.expression.terms[1].coefficient, -1.0);
}

#[test]
fn test_inequality_with_constant() {
  // Test: self_width >= parent_width - 10.0
  // Expected: self_width - parent_width + 10.0 >= 0
  let result = constraint!(self_width >= parent_width - 10.0);

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
    ElementConstraintVariable::SelfWidth
  ));
  assert_eq!(c.expression.terms[0].coefficient, 1.0);

  assert!(matches!(
    c.expression.terms[1].variable,
    ElementConstraintVariable::ParentWidth
  ));
  assert_eq!(c.expression.terms[1].coefficient, -1.0);
}

#[test]
fn test_less_or_equal() {
  // Test: self_y + 5.0 <= parent_height
  // Expected: self_y - parent_height + 5.0 <= 0
  let result = constraint!(self_y + 5.0 <= parent_height);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert!(matches!(c.operator, ElementConstraintOperator::LessOrEqual));
  assert_eq!(c.expression.constant, 5.0);
  assert_eq!(c.expression.terms.len(), 2);

  assert!(matches!(
    c.expression.terms[0].variable,
    ElementConstraintVariable::SelfY
  ));
  assert_eq!(c.expression.terms[0].coefficient, 1.0);

  assert!(matches!(
    c.expression.terms[1].variable,
    ElementConstraintVariable::ParentHeight
  ));
  assert_eq!(c.expression.terms[1].coefficient, -1.0);
}

#[test]
fn test_multiplication_by_constant() {
  // Test: self_x * 2.0 == parent_x
  // Expected: 2.0 * self_x - parent_x == 0
  let result = constraint!(self_x * 2.0 == parent_x);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert!(matches!(c.operator, ElementConstraintOperator::Equal));
  assert_eq!(c.expression.constant, 0.0);
  assert_eq!(c.expression.terms.len(), 2);

  assert!(matches!(
    c.expression.terms[0].variable,
    ElementConstraintVariable::SelfX
  ));
  assert_eq!(c.expression.terms[0].coefficient, 2.0);

  assert!(matches!(
    c.expression.terms[1].variable,
    ElementConstraintVariable::ParentX
  ));
  assert_eq!(c.expression.terms[1].coefficient, -1.0);
}

#[test]
fn test_constant_multiplication_left_side() {
  // Test: 3.0 * parent_y == self_y
  // Expected: 3.0 * parent_y - self_y == 0
  let result = constraint!(3.0 * parent_y == self_y);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert_eq!(c.expression.constant, 0.0);
  assert_eq!(c.expression.terms.len(), 2);

  assert!(matches!(
    c.expression.terms[0].variable,
    ElementConstraintVariable::ParentY
  ));
  assert_eq!(c.expression.terms[0].coefficient, 3.0);

  assert!(matches!(
    c.expression.terms[1].variable,
    ElementConstraintVariable::SelfY
  ));
  assert_eq!(c.expression.terms[1].coefficient, -1.0);
}

#[test]
fn test_division_by_constant() {
  // Test: self_width / 2.0 == parent_width
  // Expected: 0.5 * self_width - parent_width == 0
  let result = constraint!(self_width / 2.0 == parent_width);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert_eq!(c.expression.constant, 0.0);
  assert_eq!(c.expression.terms.len(), 2);

  assert!(matches!(
    c.expression.terms[0].variable,
    ElementConstraintVariable::SelfWidth
  ));
  assert_eq!(c.expression.terms[0].coefficient, 0.5);

  assert!(matches!(
    c.expression.terms[1].variable,
    ElementConstraintVariable::ParentWidth
  ));
  assert_eq!(c.expression.terms[1].coefficient, -1.0);
}

#[test]
fn test_complex_expression_with_parentheses() {
  // Test: (self_x + self_width) * 0.5 == parent_x
  // Expected: 0.5 * self_x + 0.5 * self_width - parent_x == 0
  let result = constraint!((self_x + self_width) * 0.5 == parent_x);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert_eq!(c.expression.constant, 0.0);
  assert_eq!(c.expression.terms.len(), 3);

  // Find terms by variable type
  let self_x_term = c
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::SelfX))
    .unwrap();
  assert_eq!(self_x_term.coefficient, 0.5);

  let self_width_term = c
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::SelfWidth))
    .unwrap();
  assert_eq!(self_width_term.coefficient, 0.5);

  let parent_x_term = c
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::ParentX))
    .unwrap();
  assert_eq!(parent_x_term.coefficient, -1.0);
}

#[test]
fn test_negation() {
  // Test: -self_height == parent_height
  // Expected: -self_height - parent_height == 0
  let result = constraint!(-self_height == parent_height);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert_eq!(c.expression.constant, 0.0);
  assert_eq!(c.expression.terms.len(), 2);

  assert!(matches!(
    c.expression.terms[0].variable,
    ElementConstraintVariable::SelfHeight
  ));
  assert_eq!(c.expression.terms[0].coefficient, -1.0);

  assert!(matches!(
    c.expression.terms[1].variable,
    ElementConstraintVariable::ParentHeight
  ));
  assert_eq!(c.expression.terms[1].coefficient, -1.0);
}

#[test]
fn test_multiple_terms() {
  // Test: self_x + parent_x >= self_width - 10.0
  // Expected: self_x + parent_x - self_width + 10.0 >= 0
  let result = constraint!(self_x + parent_x >= self_width - 10.0);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert!(matches!(
    c.operator,
    ElementConstraintOperator::GreaterOrEqual
  ));
  assert_eq!(c.expression.constant, 10.0);
  assert_eq!(c.expression.terms.len(), 3);

  let self_x_term = c
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::SelfX))
    .unwrap();
  assert_eq!(self_x_term.coefficient, 1.0);

  let parent_x_term = c
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::ParentX))
    .unwrap();
  assert_eq!(parent_x_term.coefficient, 1.0);

  let self_width_term = c
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::SelfWidth))
    .unwrap();
  assert_eq!(self_width_term.coefficient, -1.0);
}

#[test]
fn test_mixed_constraint_types() {
  // Test a constraint with:
  // 1. Known constraint variables (parent_x, self_x)
  // 2. Expression variables (element.x())
  // 3. Constants (10.0, 2.0)
  //
  // Test: self_x == element.x() + element.width() + parent_x * 2.0 + 10.0

  // Create a mock element with id 42
  let element = Element { id: 42 };

  let spacing = 10.0;
  let multiplier = 2.0;

  // This should compile and work correctly
  // Normalized: self_x - element.x() - element.width() - parent_x * 2.0 - 10.0 == 0
  let constraint =
    constraint1!(self_x == element.x() + element.width() + parent_x * multiplier + spacing);

  // Verify the constraint operator
  assert!(matches!(
    constraint.operator,
    ElementConstraintOperator::Equal
  ));

  // Verify strength
  assert_eq!(constraint.strength, ElementConstraints::REQUIRED);

  // Verify constant term: -spacing
  assert_eq!(constraint.expression.constant, -spacing);

  // Should have 4 terms: self_x, parent_x, element.x(), and element.width()
  assert_eq!(constraint.expression.terms.len(), 4);

  // Find and verify each term
  let self_x_term = constraint
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::SelfX))
    .expect("Should have self_x term");
  assert_eq!(self_x_term.coefficient, 1.0);

  let parent_x_term = constraint
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::ParentX))
    .expect("Should have parent_x term");
  assert_eq!(parent_x_term.coefficient, -multiplier);

  let element_x_term = constraint
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::ElementX(_)))
    .expect("Should have element.x() term");
  assert_eq!(element_x_term.coefficient, -1.0);

  let element_width_term = constraint
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::ElementWidth(_)))
    .expect("Should have element.width() term");
  assert_eq!(element_width_term.coefficient, -1.0);

  // Verify the element ID in the ElementX variant
  if let ElementConstraintVariable::ElementX(elem) = &element_x_term.variable {
    assert_eq!(elem.id, 42);
  } else {
    panic!("Expected ElementX variant");
  }
}

#[test]
fn test_complex_mixed_expression() {
  // Test with multiple runtime variable expressions
  let element1 = Element { id: 1 };
  let element2 = Element { id: 2 };

  let padding = 5.0;

  // Test: self_x == element1.x() + element1.width() + element2.x() + parent_y - padding
  // Normalized: self_x - element1.x() - element1.width() - element2.x() - parent_y + padding == 0
  let constraint =
    constraint1!(self_x == element1.x() + element1.width() + element2.x() + parent_y - padding);

  // Verify operator
  assert!(matches!(
    constraint.operator,
    ElementConstraintOperator::Equal
  ));

  // Verify strength
  assert_eq!(constraint.strength, ElementConstraints::REQUIRED);

  // Verify constant: +padding
  assert_eq!(constraint.expression.constant, padding);

  // Should have 5 terms: self_x, element1.x(), element1.width(), element2.x(), parent_y
  assert_eq!(constraint.expression.terms.len(), 5);

  // Verify self_x
  let self_x_term = constraint
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::SelfX))
    .expect("Should have self_x term");
  assert_eq!(self_x_term.coefficient, 1.0);

  // Verify parent_y
  let parent_y_term = constraint
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::ParentY))
    .expect("Should have parent_y term");
  assert_eq!(parent_y_term.coefficient, -1.0);

  // Verify element1.x()
  let element1_x_count = constraint
    .expression
    .terms
    .iter()
    .filter(|t| matches!(t.variable, ElementConstraintVariable::ElementX(ref e) if e.id == 1))
    .count();
  assert_eq!(element1_x_count, 1);

  // Verify element1.width()
  let element1_width_count = constraint
    .expression
    .terms
    .iter()
    .filter(|t| matches!(t.variable, ElementConstraintVariable::ElementWidth(ref e) if e.id == 1))
    .count();
  assert_eq!(element1_width_count, 1);

  // Verify element2.x()
  let element2_x_count = constraint
    .expression
    .terms
    .iter()
    .filter(|t| matches!(t.variable, ElementConstraintVariable::ElementX(ref e) if e.id == 2))
    .count();
  assert_eq!(element2_x_count, 1);
}

#[test]
fn test_runtime_variable_with_coefficient() {
  // Test runtime variable with a coefficient
  // Test: self_x == element.x() * 2.0 + 10.0
  // Normalized: self_x - element.x() * 2.0 - 10.0 == 0
  let element = Element { id: 99 };

  let constraint = constraint1!(self_x == element.x() * 2.0 + 10.0);

  // Verify operator
  assert!(matches!(
    constraint.operator,
    ElementConstraintOperator::Equal
  ));

  // Verify strength
  assert_eq!(constraint.strength, ElementConstraints::REQUIRED);

  // Verify constant: -10.0
  assert_eq!(constraint.expression.constant, -10.0);

  // Should have 2 terms: self_x and element.x()
  assert_eq!(constraint.expression.terms.len(), 2);

  // Verify self_x
  let self_x_term = constraint
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::SelfX))
    .expect("Should have self_x term");
  assert_eq!(self_x_term.coefficient, 1.0);

  // Verify element.x() with coefficient -2.0
  let element_right_term = constraint
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::ElementX(ref e) if e.id == 99))
    .expect("Should have element.x() term");
  assert_eq!(element_right_term.coefficient, -2.0);
}

#[test]
fn test_runtime_variable_in_subtraction() {
  // Test runtime variable in subtraction
  // Test: self_width == parent_width - element.x() - spacing
  // Normalized: self_width - parent_width + element.x() + spacing == 0
  let element = Element { id: 7 };

  let spacing = 8.0;

  let constraint = constraint1!(self_width == parent_width - element.x() - spacing);

  // Verify operator
  assert!(matches!(
    constraint.operator,
    ElementConstraintOperator::Equal
  ));

  // Verify strength
  assert_eq!(constraint.strength, ElementConstraints::REQUIRED);

  // Verify constant: +spacing
  assert_eq!(constraint.expression.constant, spacing);

  // Should have 3 terms: self_width, parent_width, element.x()
  assert_eq!(constraint.expression.terms.len(), 3);

  // Verify self_width
  let self_width_term = constraint
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::SelfWidth))
    .expect("Should have self_width term");
  assert_eq!(self_width_term.coefficient, 1.0);

  // Verify parent_width
  let parent_width_term = constraint
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::ParentWidth))
    .expect("Should have parent_width term");
  assert_eq!(parent_width_term.coefficient, -1.0);

  // Verify element.x() with coefficient +1.0 (double negation)
  let element_left_term = constraint
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::ElementX(ref e) if e.id == 7))
    .expect("Should have element.x() term");
  assert_eq!(element_left_term.coefficient, 1.0);
}

#[test]
fn test_runtime_variable_in_addition() {
  let element = Element { id: 7 };

  let spacing = 8.0;

  // becomes: self_x - spacing - element.x()
  let constraint = constraint1!(self_x == element.x() + spacing);

  assert!(matches!(
    constraint.operator,
    ElementConstraintOperator::Equal
  ));

  assert_eq!(constraint.strength, ElementConstraints::REQUIRED);
  assert_eq!(constraint.expression.constant, -spacing);
  assert_eq!(constraint.expression.terms.len(), 2);

  assert_eq!(
    constraint.expression.terms[0].variable,
    ElementConstraintVariable::SelfX
  );
  assert_eq!(constraint.expression.terms[0].coefficient, 1.0);

  assert_eq!(
    constraint.expression.terms[1].variable,
    ElementConstraintVariable::ElementX(Element { id: 7 })
  );
  assert_eq!(constraint.expression.terms[1].coefficient, -1.0);
}

#[test]
fn test_all_three_types_combined() {
  // Ultimate test: all three types in one constraint
  // Test: (self_x + self_width) * 0.5 == element.x() + parent_x * scale + offset + 10.0
  // Normalized: 0.5*self_x + 0.5*self_width - element.x() - parent_x*scale - offset - 10.0 == 0

  let element = Element { id: 123 };

  let offset = 15.0;
  let scale = 0.5;

  let constraint =
    constraint1!((self_x + self_width) * 0.5 == element.x() + parent_x * scale + offset + 10.0);

  // Verify operator
  assert!(matches!(
    constraint.operator,
    ElementConstraintOperator::Equal
  ));

  // Verify strength
  assert_eq!(constraint.strength, ElementConstraints::REQUIRED);

  // Verify constant: -offset - 10.0
  assert_eq!(constraint.expression.constant, -offset - 10.0);

  // Should have 4 terms: self_x, self_width, element.x(), parent_x
  assert_eq!(constraint.expression.terms.len(), 4);

  // Verify self_x with coefficient 0.5
  let self_x_term = constraint
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::SelfX))
    .expect("Should have self_x term");
  assert_eq!(self_x_term.coefficient, 0.5);

  // Verify self_width with coefficient 0.5
  let self_width_term = constraint
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::SelfWidth))
    .expect("Should have self_width term");
  assert_eq!(self_width_term.coefficient, 0.5);

  // Verify parent_x with coefficient -scale
  let parent_x_term = constraint
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::ParentX))
    .expect("Should have parent_x term");
  assert_eq!(parent_x_term.coefficient, -scale);

  // Verify element.x() with coefficient -1.0
  let element_right_term = constraint
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::ElementX(ref e) if e.id == 123))
    .expect("Should have element.x() term");
  assert_eq!(element_right_term.coefficient, -1.0);
}

// ============================================================================
// constraint1! macro tests
// ============================================================================

#[test]
fn test_constraint1_returns_single_constraint() {
  // Test that constraint1! returns a single ElementConstraint
  // self_x == parent_x + 10.0
  // Normalized: self_x - parent_x - 10.0 == 0
  let single_constraint = constraint1!(self_x == parent_x + 10.0);

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
      constraint1!(self_y == parent_y),
      constraint1!(
        self_height == parent_height,
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
  let constraint = constraint1!(self_x == parent_x + offset_x);
  let constraint2 = constraint1!(self_y == parent_y + offset_y * 2.0);

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

  let constraint = constraint1!(self_x == parent_x + base_padding * multiplier);

  assert!(matches!(
    constraint.operator,
    ElementConstraintOperator::Equal
  ));
  assert_eq!(constraint.expression.terms.len(), 2);
}

#[test]
fn test_constraint1_with_all_operators() {
  let offset = 5.0;

  let c1 = constraint1!(self_x == parent_x + offset);
  let c2 = constraint1!(self_width >= parent_width - offset);
  let c3 = constraint1!(self_y <= parent_y + offset);

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
  // Test: self_y == parent_y - (5.0 + 10.0)
  // Expected: self_y - parent_y + 15.0 == 0
  let result = constraint!(self_y == parent_y - (5.0 + 10.0));

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert_eq!(c.expression.constant, 15.0);
  assert_eq!(c.expression.terms.len(), 2);
}

#[test]
fn test_integer_literals() {
  // Test: self_x + 10 == parent_x
  // Expected: self_x - parent_x + 10.0 == 0
  let result = constraint!(self_x + 10 == parent_x);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert_eq!(c.expression.constant, 10.0);
  assert_eq!(c.expression.terms.len(), 2);
}

#[test]
fn test_custom_strength_literal() {
  // Test: self_x == parent_x, strength = 1.0
  let result = constraint!(self_x == parent_x, strength = 1.0);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert_eq!(c.strength, 1.0);
}

#[test]
fn test_custom_strength_constant() {
  // Test: self_width >= parent_width, strength = ElementConstraints::WEAK
  let result = constraint!(
    self_width >= parent_width,
    strength = ElementConstraints::WEAK
  );

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert_eq!(c.strength, ElementConstraints::WEAK);
}

#[test]
fn test_custom_strength_expression() {
  // Test: self_y <= parent_y, strength = 5.0 * 2.0
  let result = constraint!(self_y <= parent_y, strength = 5.0 * 2.0);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert_eq!(c.strength, 10.0);
}

#[test]
fn test_all_variable_types() {
  // Test each variable type can be used
  let _r1 = constraint!(parent_x == self_x);
  let _r2 = constraint!(parent_width == self_width);
  let _r3 = constraint!(parent_y == self_y);
  let _r4 = constraint!(parent_height == self_height);
}

#[test]
fn test_right_to_left_movement() {
  // Test: 0.0 == self_x - parent_x
  // Expected: -self_x + parent_x == 0 (or self_x - parent_x == 0 after simplification)
  let result = constraint!(0.0 == self_x - parent_x);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert_eq!(c.expression.constant, 0.0);
  assert_eq!(c.expression.terms.len(), 2);

  // The terms should be negated since we move them from right to left
  let self_x_term = c
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::SelfX))
    .unwrap();
  assert_eq!(self_x_term.coefficient, -1.0);

  let parent_x_term = c
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::ParentX))
    .unwrap();
  assert_eq!(parent_x_term.coefficient, 1.0);
}

#[test]
fn test_nested_parentheses() {
  // Test: ((self_x + 10.0) - 5.0) == parent_x
  // Expected: self_x - parent_x + 5.0 == 0
  let result = constraint!(((self_x + 10.0) - 5.0) == parent_x);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert_eq!(c.expression.constant, 5.0);
  assert_eq!(c.expression.terms.len(), 2);
}

#[test]
fn test_multiplication_distributive() {
  // Test: 2.0 * (self_x + parent_x) == self_width
  // Expected: 2.0 * self_x + 2.0 * parent_x - self_width == 0
  let result = constraint!(2.0 * (self_x + parent_x) == self_width);

  assert_eq!(result.constraints.len(), 1);
  let c = &result.constraints[0];

  assert_eq!(c.expression.constant, 0.0);
  assert_eq!(c.expression.terms.len(), 3);

  let self_x_term = c
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::SelfX))
    .unwrap();
  assert_eq!(self_x_term.coefficient, 2.0);

  let parent_x_term = c
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::ParentX))
    .unwrap();
  assert_eq!(parent_x_term.coefficient, 2.0);

  let self_width_term = c
    .expression
    .terms
    .iter()
    .find(|t| matches!(t.variable, ElementConstraintVariable::SelfWidth))
    .unwrap();
  assert_eq!(self_width_term.coefficient, -1.0);
}
