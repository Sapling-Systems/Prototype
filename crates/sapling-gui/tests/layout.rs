use std::collections::HashMap;

use sapling_gui::{
  ConstraintResolver, ElementVariable, RelationshipMeta,
  prelude::{CompiledConstraint, ConstraintVariable, Dimension},
};

fn create_relationship_meta(parent_map: Vec<Option<usize>>) -> Vec<RelationshipMeta> {
  let mut child_map = HashMap::<usize, Vec<usize>>::new();
  for (id, parent) in parent_map.iter().enumerate() {
    if let Some(parent) = parent {
      let entry = child_map.entry(*parent).or_default();
      entry.push(id)
    }
  }

  let mut depth_map = HashMap::<usize, usize>::new();
  for (id, parent) in parent_map.iter().enumerate() {
    if let Some(parent) = parent {
      let entry = depth_map.entry(*parent).or_default();
      *entry += 1;
    }
  }

  parent_map
    .iter()
    .enumerate()
    .map(|(id, parent)| RelationshipMeta {
      parent_id: *parent,
      children: child_map.get(&id).cloned().unwrap_or_default(),
      depth: depth_map.get(&id).copied().unwrap_or_default(),
    })
    .collect()
}

#[test]
fn test_simple_layout() {
  let root = 0;
  let child_a = 1;
  let child_b = 2;
  let parent_map = vec![None, Some(0), Some(0)];
  let constraints = vec![
    (
      root,
      CompiledConstraint::ForcedConstAssignment {
        variable: ConstraintVariable::SelfWidth,
        constant: 128.0,
      },
    ),
    (
      child_a,
      CompiledConstraint::ForcedVariableAssignment {
        target_variable: ConstraintVariable::SelfWidth,
        source_variable: ConstraintVariable::ParentWidth,
        constant_offset: -5.0,
      },
    ),
    (
      child_b,
      CompiledConstraint::ForcedVariableAssignment {
        target_variable: ConstraintVariable::SelfWidth,
        source_variable: ConstraintVariable::ParentWidth,
        constant_offset: -10.0,
      },
    ),
  ];
  let mut resolver = ConstraintResolver::new(
    constraints,
    create_relationship_meta(parent_map),
    (1.0, 1.0),
  );
  resolver.resolve();

  assert_eq!(
    resolver.get_element_variable_resolution(root, ElementVariable::Width),
    128.0
  );

  assert_eq!(
    resolver.get_element_variable_resolution(child_a, ElementVariable::Width),
    123.0
  );
  assert_eq!(
    resolver.get_element_variable_resolution(child_b, ElementVariable::Width),
    118.0
  );
}

#[test]
fn test_child_parent_sizing() {
  let root = 0;
  let child_a = 1;
  let child_b = 2;
  let child_a_a = 3;
  let parent_map = vec![None, Some(0), Some(0), Some(1)];
  let constraints = vec![
    (
      root,
      CompiledConstraint::TryAssumeMaxChildSize {
        dimension: Dimension::Height,
        constant_offset: 1.0,
      },
    ),
    (
      child_a,
      CompiledConstraint::TryAssumeMaxChildSize {
        dimension: Dimension::Height,
        constant_offset: 2.0,
      },
    ),
    (
      child_a_a,
      CompiledConstraint::ForcedConstAssignment {
        variable: ConstraintVariable::SelfHeight,
        constant: 32.0,
      },
    ),
    (
      child_b,
      CompiledConstraint::ForcedConstAssignment {
        variable: ConstraintVariable::SelfHeight,
        constant: 16.0,
      },
    ),
  ];
  let mut resolver = ConstraintResolver::new(
    constraints,
    create_relationship_meta(parent_map),
    (1.0, 1.0),
  );
  resolver.resolve();

  assert_eq!(
    resolver.get_element_variable_resolution(root, ElementVariable::Height),
    35.0
  );
  assert_eq!(
    resolver.get_element_variable_resolution(child_a, ElementVariable::Height),
    34.0
  );
  assert_eq!(
    resolver.get_element_variable_resolution(child_a_a, ElementVariable::Height),
    32.0
  );
  assert_eq!(
    resolver.get_element_variable_resolution(child_b, ElementVariable::Height),
    16.0
  );
}

#[test]
fn test_child_parent_only_if_needed() {
  let root = 0;
  let child_a = 1;
  let child_b = 2;
  let child_a_a = 3;
  let parent_map = vec![None, Some(0), Some(0), Some(1)];
  let constraints = vec![
    (
      root,
      CompiledConstraint::TryAssumeMaxChildSize {
        dimension: Dimension::Height,
        constant_offset: 1.0,
      },
    ),
    (
      child_a,
      CompiledConstraint::TryAssumeMaxChildSize {
        dimension: Dimension::Height,
        constant_offset: 2.0,
      },
    ),
    (
      child_a,
      CompiledConstraint::ForcedConstAssignment {
        variable: ConstraintVariable::SelfHeight,
        constant: 4.0,
      },
    ),
    (
      child_a_a,
      CompiledConstraint::ForcedConstAssignment {
        variable: ConstraintVariable::SelfHeight,
        constant: 32.0,
      },
    ),
    (
      child_b,
      CompiledConstraint::ForcedConstAssignment {
        variable: ConstraintVariable::SelfHeight,
        constant: 1.0,
      },
    ),
  ];
  let mut resolver = ConstraintResolver::new(
    constraints,
    create_relationship_meta(parent_map),
    (1.0, 1.0),
  );
  resolver.resolve();

  assert_eq!(
    resolver.get_element_variable_resolution(root, ElementVariable::Height),
    5.0
  );
  assert_eq!(
    resolver.get_element_variable_resolution(child_a, ElementVariable::Height),
    4.0
  );
  assert_eq!(
    resolver.get_element_variable_resolution(child_a_a, ElementVariable::Height),
    32.0
  );
  assert_eq!(
    resolver.get_element_variable_resolution(child_b, ElementVariable::Height),
    1.0
  );
}

#[test]
fn test_child_parent_only_if_needed_reverse_order() {
  let root = 0;
  let child_a = 1;
  let child_b = 2;
  let child_a_a = 3;
  let parent_map = vec![None, Some(0), Some(0), Some(1)];
  let constraints = vec![
    (
      root,
      CompiledConstraint::TryAssumeMaxChildSize {
        dimension: Dimension::Height,
        constant_offset: 1.0,
      },
    ),
    (
      child_a,
      CompiledConstraint::ForcedConstAssignment {
        variable: ConstraintVariable::SelfHeight,
        constant: 4.0,
      },
    ),
    (
      child_a,
      CompiledConstraint::TryAssumeMaxChildSize {
        dimension: Dimension::Height,
        constant_offset: 2.0,
      },
    ),
    (
      child_a_a,
      CompiledConstraint::ForcedConstAssignment {
        variable: ConstraintVariable::SelfHeight,
        constant: 32.0,
      },
    ),
    (
      child_b,
      CompiledConstraint::ForcedConstAssignment {
        variable: ConstraintVariable::SelfHeight,
        constant: 1.0,
      },
    ),
  ];
  let mut resolver = ConstraintResolver::new(
    constraints,
    create_relationship_meta(parent_map),
    (1.0, 1.0),
  );
  resolver.resolve();

  assert_eq!(
    resolver.get_element_variable_resolution(root, ElementVariable::Height),
    5.0
  );
  assert_eq!(
    resolver.get_element_variable_resolution(child_a, ElementVariable::Height),
    4.0
  );
  assert_eq!(
    resolver.get_element_variable_resolution(child_a_a, ElementVariable::Height),
    32.0
  );
  assert_eq!(
    resolver.get_element_variable_resolution(child_b, ElementVariable::Height),
    1.0
  );
}
