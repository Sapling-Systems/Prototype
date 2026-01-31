use std::{
  collections::{HashMap, HashSet, VecDeque},
  usize,
};

use petgraph::{
  Directed, Graph, algo::toposort, dot::Dot, graph::NodeIndex, prelude::StableGraph, visit::EdgeRef,
};

use crate::layout::{
  Dimension,
  constraint::{CompiledConstraint, ConstraintVariable, ElementVariable},
};

pub struct ConstraintResolver {
  constraints: Vec<(usize, CompiledConstraint)>,
  relationships: Vec<RelationshipMeta>,
  resolved_variables: Vec<f32>,
}

impl ConstraintResolver {
  const MAX_VARIABLES_PER_ELEMENT: usize = 4;
  const ROOT_VARIABLES: usize = 2;

  pub fn new(
    constraints: Vec<(usize, CompiledConstraint)>,
    relationships: Vec<RelationshipMeta>,
    window: (f32, f32),
  ) -> Self {
    let mut resolver = ConstraintResolver {
      constraints,
      resolved_variables: vec![
        0.0;
        relationships.len() * Self::MAX_VARIABLES_PER_ELEMENT
          + Self::ROOT_VARIABLES
      ],
      relationships,
    };
    resolver.resolved_variables[0] = window.0;
    resolver.resolved_variables[1] = window.1;
    resolver
  }

  fn build_dependency_graph(&mut self) -> Graph<usize, usize, Directed> {
    let mut graph = StableGraph::<usize, usize>::with_capacity(
      self.constraints.len(),
      self.constraints.len() * 10,
    );

    // 1. Pre-create nodes for each constraint and memoize their assignment relation.
    let mut variable_assign_map = HashMap::<usize, Vec<NodeIndex>>::new();
    let mut post_child_parent_queue: HashSet<(usize, usize, usize, NodeIndex)> = HashSet::new();
    let mut post_parent_child_queue: HashSet<(usize, usize, usize, NodeIndex)> = HashSet::new();

    for (constraint_id, (element_id, constraint)) in self.constraints.iter().enumerate() {
      match constraint {
        CompiledConstraint::TryAssumeMaxChildSize { dimension, .. } => {
          let variable = match dimension {
            Dimension::Width => ConstraintVariable::SelfWidth,
            Dimension::Height => ConstraintVariable::SelfHeight,
          };
          let variable_index = self.map_element_variable_to_index(*element_id, variable);
          let relationship = &self.relationships[*element_id];
          let node_index = graph.add_node(constraint_id);
          variable_assign_map
            .entry(variable_index)
            .or_default()
            .push(node_index);
          post_child_parent_queue.insert((
            variable_index,
            *element_id,
            relationship.depth,
            node_index,
          ));
        }
        CompiledConstraint::TryAssumeParentSize { .. } => {}
        CompiledConstraint::ForcedConstAssignment { variable, .. } => {
          let variable_index = self.map_element_variable_to_index(*element_id, *variable);
          let node_index = graph.add_node(constraint_id);
          variable_assign_map
            .entry(variable_index)
            .or_default()
            .push(node_index);
        }
        CompiledConstraint::ForcedVariableAssignment {
          target_variable, ..
        } => {
          let variable_index = self.map_element_variable_to_index(*element_id, *target_variable);
          let node_index = graph.add_node(constraint_id);
          variable_assign_map
            .entry(variable_index)
            .or_default()
            .push(node_index);
        }
        CompiledConstraint::ForcedVariableAssignmentMaxOf {
          target_variable, ..
        } => {
          let variable_index = self.map_element_variable_to_index(*element_id, *target_variable);
          let node_index = graph.add_node(constraint_id);
          variable_assign_map
            .entry(variable_index)
            .or_default()
            .push(node_index);
        }
        CompiledConstraint::ForcedVariableAssignmentTerms {
          target_variable, ..
        } => {
          let variable_index = self.map_element_variable_to_index(*element_id, *target_variable);
          let node_index = graph.add_node(constraint_id);
          variable_assign_map
            .entry(variable_index)
            .or_default()
            .push(node_index);
        }
      }
    }

    // 2. Add direct edges between constraints
    fn add_variable_assignment_edge(
      graph: &mut StableGraph<usize, usize>,
      variable_assign_map: &HashMap<usize, Vec<NodeIndex>>,
      target_variable_index: usize,
      source_variable_index: usize,
    ) {
      let target_node_index = variable_assign_map.get(&target_variable_index);
      let source_node_index = variable_assign_map.get(&source_variable_index);

      for source_node in source_node_index.iter().flat_map(|i| *i) {
        for target_node in target_node_index.iter().flat_map(|i| *i) {
          graph.add_edge(*source_node, *target_node, source_variable_index);
        }
      }
    }

    for (_constraint_id, (element_id, constraint)) in self.constraints.iter().enumerate() {
      match constraint {
        // No op as everything is constant
        CompiledConstraint::TryAssumeMaxChildSize { .. } => {}
        CompiledConstraint::TryAssumeParentSize { .. } => {}
        CompiledConstraint::ForcedConstAssignment { .. } => {}
        CompiledConstraint::ForcedVariableAssignment {
          target_variable,
          source_variable,
          ..
        } => {
          let target_variable_index =
            self.map_element_variable_to_index(*element_id, *target_variable);
          let source_variable_index =
            self.map_element_variable_to_index(*element_id, *source_variable);

          add_variable_assignment_edge(
            &mut graph,
            &variable_assign_map,
            target_variable_index,
            source_variable_index,
          );
        }
        CompiledConstraint::ForcedVariableAssignmentTerms {
          target_variable,
          source_variables,
          ..
        } => {
          let target_variable_index =
            self.map_element_variable_to_index(*element_id, *target_variable);

          for (source_variable, _) in source_variables {
            let source_variable_index =
              self.map_element_variable_to_index(*element_id, *source_variable);

            add_variable_assignment_edge(
              &mut graph,
              &variable_assign_map,
              target_variable_index,
              source_variable_index,
            );
          }
        }
        CompiledConstraint::ForcedVariableAssignmentMaxOf {
          target_variable,
          source_variables,
          ..
        } => {
          let target_variable_index =
            self.map_element_variable_to_index(*element_id, *target_variable);

          for source_variable in source_variables {
            let source_variable_index =
              self.map_element_variable_to_index(*element_id, *source_variable);

            add_variable_assignment_edge(
              &mut graph,
              &variable_assign_map,
              target_variable_index,
              source_variable_index,
            );
          }
        }
      }
    }

    // 3. Resolve edges that require to go from child to parent order
    //      Ordering by depth allows us to only look at direct children instead of recursively looking for some child with size because previous iterations have already resolved the parent size.
    let mut post_child_parent_queue = post_child_parent_queue.into_iter().collect::<Vec<_>>();
    post_child_parent_queue.sort_unstable_by_key(|(_, _, depth, _)| *depth);
    post_child_parent_queue.reverse();

    fn check_if_node_reaches_other_node(
      graph: &StableGraph<usize, usize>,
      node: NodeIndex,
      other: NodeIndex,
    ) -> bool {
      if node == other {
        return true;
      }
      let mut visited = HashSet::new();
      let mut queue = VecDeque::new();
      queue.push_back(node);
      while let Some(current) = queue.pop_front() {
        if current == other {
          return true;
        }
        visited.insert(current);
        for neighbor in graph.neighbors(current) {
          if !visited.contains(&neighbor) {
            queue.push_back(neighbor);
          }
        }
      }
      false
    }

    for (variable_index, element_id, _, node_id) in post_child_parent_queue {
      let (_, constraint) = self
        .constraints
        .get(*graph.node_weight(node_id).unwrap())
        .unwrap();
      let relationship = self.relationships.get(element_id).unwrap();

      match constraint {
        CompiledConstraint::TryAssumeMaxChildSize { dimension, .. } => {
          let dimension_variable = match dimension {
            Dimension::Width => ConstraintVariable::SelfWidth,
            Dimension::Height => ConstraintVariable::SelfHeight,
          };
          let coordinate_variable = match dimension {
            Dimension::Width => ConstraintVariable::SelfX,
            Dimension::Height => ConstraintVariable::SelfY,
          };
          let mut found_child = false;
          for child_id in &relationship.children {
            let variable_index = self.map_element_variable_to_index(*child_id, dimension_variable);
            if let Some(child_dimension_assign_node) = variable_assign_map.get(&variable_index) {
              for child_assign_node in child_dimension_assign_node {
                if !check_if_node_reaches_other_node(&graph, node_id, *child_assign_node) {
                  graph.add_edge(*child_assign_node, node_id, *child_id);

                  let variable_index =
                    self.map_element_variable_to_index(*child_id, coordinate_variable);
                  if let Some(child_dimension_assign_node) =
                    variable_assign_map.get(&variable_index)
                  {
                    for child_coordinate_assign_node in child_dimension_assign_node {
                      graph.add_edge(*child_coordinate_assign_node, node_id, *child_id);
                    }
                  }
                }
              }
              found_child = true;
            }
          }
          // Removing the node is important if no child was found to allow
          // other constraint resolved in this case.
          if !found_child {
            graph.remove_node(node_id);
            if let Some(assignments) = variable_assign_map.get_mut(&variable_index) {
              assignments.retain(|assignment| assignment != &node_id);
            }
            continue;
          }

          // Because we need consider it's own position to calculate bottom/right value we need to also add
          // an edge to the coordinate variable of itself.
          let self_coordinate_assignments = variable_assign_map
            .get(&self.map_element_variable_to_index(element_id, coordinate_variable));
          if let Some(assignments) = self_coordinate_assignments {
            for assignment in assignments {
              // Small hack: usize::MAX is reserved for self coordinate
              graph.add_edge(*assignment, node_id, usize::MAX);
            }
          }
        }
        CompiledConstraint::TryAssumeParentSize { .. } => {}
        _ => {}
      }
    }

    // 4. Resolve edges that require to go from parent to child order
    //      Ordering by depth allows us to only look at direct children instead of recursively looking for some child with size because previous iterations have already resolved the parent size.

    graph.into()
  }

  pub fn get_element_variable_resolution(
    &self,
    element_id: usize,
    variable: ElementVariable,
  ) -> f32 {
    let index = self.map_element_variable_to_index(
      element_id,
      match variable {
        ElementVariable::X => ConstraintVariable::ElementX { id: element_id },
        ElementVariable::Y => ConstraintVariable::ElementY { id: element_id },
        ElementVariable::Width => ConstraintVariable::ElementWidth { id: element_id },
        ElementVariable::Height => ConstraintVariable::ElementHeight { id: element_id },
      },
    );
    self.resolved_variables[index]
  }

  pub fn resolve(&mut self) {
    let graph = self.build_dependency_graph();

    let topology = match toposort(&graph, None) {
      Ok(mut topology) => {
        //topology.reverse();
        topology
      }
      Err(err) => {
        let constraint_id = graph.node_weight(err.node_id()).unwrap();
        let constraint = &self.constraints[*constraint_id];

        eprintln!(
          "DOT: \n{:?}\n\n",
          Dot::with_attr_getters(
            &graph,
            &[],
            &|_graph, edge| "".to_string(),
            &|graph, (node, _)| {
              let constraint_id = graph.node_weight(node).copied().unwrap();
              let (element_id, constraint) = &self.constraints[constraint_id];
              format!("label=\"{:?} on {}\"", constraint, element_id)
            },
          )
        );
        panic!(
          "Cycle found in layout constraints, not supported yet! ID: {} Constraint: {:?}",
          constraint_id, constraint
        );
      }
    };
    //println!("START EXECUTION OF CONSTRAINTS =========");

    for node_index in topology {
      let constraint_id = *graph.node_weight(node_index).unwrap();
      let (element_id, constraint) = &self.constraints[constraint_id];

      match constraint {
        CompiledConstraint::TryAssumeMaxChildSize {
          constant_offset,
          dimension,
        } => {
          let dimension_variable = match dimension {
            Dimension::Width => ConstraintVariable::SelfWidth,
            Dimension::Height => ConstraintVariable::SelfHeight,
          };
          let coordinate_variable = match dimension {
            Dimension::Width => ConstraintVariable::SelfX,
            Dimension::Height => ConstraintVariable::SelfY,
          };

          let dimension_variable_index =
            self.map_element_variable_to_index(*element_id, dimension_variable);
          let current_dimension_value = self.resolved_variables[dimension_variable_index];

          let coordinate_variable_index =
            self.map_element_variable_to_index(*element_id, coordinate_variable);
          let current_coordinate_value = self.resolved_variables[coordinate_variable_index];

          let incoming_edges = graph
            .edges_directed(node_index, petgraph::Direction::Incoming)
            .collect::<Vec<_>>();
          if incoming_edges.is_empty() || current_dimension_value != 0.0 {
            continue;
          }

          let mut absolute_end = 0.0f32;

          for incoming_edge in incoming_edges {
            let source_element_id = *incoming_edge.weight();

            // usize::MAX is reserved for its' own coordinate, which is only added for correct
            // execution order
            if source_element_id == usize::MAX {
              continue;
            }

            let coordinate_index =
              self.map_element_variable_to_index(source_element_id, coordinate_variable);
            let dimension_index =
              self.map_element_variable_to_index(source_element_id, dimension_variable);

            let coordinate_value = self.resolved_variables[coordinate_index];
            let dimension_value = self.resolved_variables[dimension_index];

            absolute_end = absolute_end.max(dimension_value + coordinate_value);
          }
          self.resolved_variables[dimension_variable_index] =
            (absolute_end - current_coordinate_value).max(0.0) + constant_offset;
        }
        CompiledConstraint::TryAssumeParentSize { .. } => {}
        CompiledConstraint::ForcedConstAssignment { variable, constant } => {
          let target_variable_index = self.map_element_variable_to_index(*element_id, *variable);
          self.resolved_variables[target_variable_index] = *constant;
        }
        CompiledConstraint::ForcedVariableAssignment {
          target_variable,
          source_variable,
          constant_offset,
        } => {
          let source_index = self.map_element_variable_to_index(*element_id, *source_variable);
          let target_index = self.map_element_variable_to_index(*element_id, *target_variable);
          self.resolved_variables[target_index] =
            self.resolved_variables[source_index] + *constant_offset;
        }
        CompiledConstraint::ForcedVariableAssignmentMaxOf {
          target_variable,
          source_variables,
          constant_offset,
        } => {
          let target_index = self.map_element_variable_to_index(*element_id, *target_variable);
          let max_value = source_variables.iter().fold(0.0f32, |a, next| {
            let b = self.resolved_variables[self.map_element_variable_to_index(*element_id, *next)];
            a.max(b)
          });
          self.resolved_variables[target_index] = max_value + *constant_offset;
        }
        CompiledConstraint::ForcedVariableAssignmentTerms {
          target_variable,
          source_variables,
          constant_offset,
        } => {
          let target_index = self.map_element_variable_to_index(*element_id, *target_variable);
          let sum_value = source_variables
            .iter()
            .fold(0.0f32, |a, (next, multiplicator)| {
              let b =
                self.resolved_variables[self.map_element_variable_to_index(*element_id, *next)];
              a + b * multiplicator
            });
          self.resolved_variables[target_index] = sum_value + *constant_offset;
        }
      }
    }
  }

  fn map_element_variable_to_index(&self, self_id: usize, variable: ConstraintVariable) -> usize {
    let total_static_variables = Self::ROOT_VARIABLES;
    let total_variables_per_element = Self::MAX_VARIABLES_PER_ELEMENT;
    let self_offset = total_variables_per_element * self_id + total_static_variables;
    let relationship = &self.relationships[self_id];
    let parent_id = relationship.parent_id;
    let parent_offset =
      parent_id.map(|id| total_variables_per_element * id + total_static_variables);

    match variable {
      ConstraintVariable::WindowWidth => 0,
      ConstraintVariable::WindowHeight => 1,
      ConstraintVariable::SelfWidth => self_offset,
      ConstraintVariable::SelfHeight => self_offset + 1,
      ConstraintVariable::SelfX => self_offset + 2,
      ConstraintVariable::SelfY => self_offset + 3,
      ConstraintVariable::ParentWidth => parent_offset.unwrap_or(self_offset),
      ConstraintVariable::ParentHeight => parent_offset.unwrap_or(self_offset) + 1,
      ConstraintVariable::ParentX => parent_offset.unwrap_or(self_offset) + 2,
      ConstraintVariable::ParentY => parent_offset.unwrap_or(self_offset) + 3,
      ConstraintVariable::ElementWidth { id } => {
        let element_offset = total_variables_per_element * id + total_static_variables;
        element_offset
      }
      ConstraintVariable::ElementHeight { id } => {
        let element_offset = total_variables_per_element * id + total_static_variables;
        element_offset + 1
      }
      ConstraintVariable::ElementX { id } => {
        let element_offset = total_variables_per_element * id + total_static_variables;
        element_offset + 2
      }
      ConstraintVariable::ElementY { id } => {
        let element_offset = total_variables_per_element * id + total_static_variables;
        element_offset + 3
      }
    }
  }
}

pub struct RelationshipMeta {
  pub parent_id: Option<usize>,
  pub children: Vec<usize>,
  pub depth: usize,
}
