use std::{
  any::{Any, TypeId},
  collections::HashMap,
  time::Duration,
};

use sapling_app::App;

use crate::{
  component::Component,
  input::InputState,
  layout::{
    CompiledConstraint, ConstraintResolver, ConstraintVariable, Dimension, ElementVariable,
    RelationshipMeta, ResolvedLayout, UserElementConstraints,
  },
  prelude::Renderer,
  theme::Theme,
};

pub struct Orchestrator {
  elements: Vec<AllocatedElement>,
  debug_enabled: bool,
  debug_tree: Option<Vec<DebugAllocatedElement>>,
  mutable_state: HashMap<ComponentStateKey, Box<dyn Any>>,
}

impl Orchestrator {
  pub fn new(debug_enabled: bool) -> Self {
    Orchestrator {
      elements: Vec::new(),
      debug_enabled,
      debug_tree: None,
      mutable_state: HashMap::new(),
    }
  }

  pub fn construct_and_render<T: Component + 'static, TRenderer: Renderer>(
    &mut self,
    root: T,
    width: f32,
    height: f32,
    renderer: &mut TRenderer,
    theme: &mut Theme,
    app: &mut App,
    input_state: &InputState,
  ) -> OrchestratorStats {
    let construction_start = std::time::Instant::now();
    self.elements.clear();

    // construction phase
    self.elements.push(AllocatedElement {
      parent_element: None,
      depth: 0,
      key: "root".to_string(),
      component: Some(Box::new(root)),
      direct_child_component_occurrences: HashMap::new(),
      constraints: vec![],
    });

    let element = self.elements.last_mut().unwrap();
    let mut component = element.component.take().unwrap();

    component.construct(&mut ElementContext {
      parent_element: Some(0),
      depth: 1,
      elements: &mut self.elements,
      mutable_state: &mut self.mutable_state,
      debug_enabled: self.debug_enabled,
      render_width: width,
      render_height: height,
      prev_debug_nodes: &self.debug_tree,
      input_state,
      theme,
      app,
    });

    let mut parent_children_relationship: HashMap<usize, Vec<usize>> = HashMap::new();
    let construction_end = std::time::Instant::now();
    let layouting_start = std::time::Instant::now();
    {
      let element = &mut self.elements[0];
      element.constraints = UserElementConstraints::fixed_size(width, height)
        .merged(&UserElementConstraints::absolute_position(0.0, 0.0))
        .constraints;
      element.component = Some(component);

      // Create tree info
      for (index, _) in self.elements.iter().enumerate() {
        parent_children_relationship.insert(index, Vec::new());
      }
      for (id, element) in self.elements.iter().enumerate() {
        if let Some(parent) = element.parent_element {
          parent_children_relationship
            .get_mut(&parent)
            .unwrap()
            .push(id);
        }
      }
    }

    // Post processing layout to add default assignments if needed
    let elements = self.elements.len();
    for element_id in 0..elements {
      let mut has_explicit_width = false;
      let mut has_explicit_height = false;
      let mut has_explicit_x = false;
      let mut has_explicit_y = false;

      {
        let element = &mut self.elements[element_id];
        for constraint in element.constraints.iter() {
          match constraint.get_explicit_target() {
            Some(ConstraintVariable::SelfWidth { .. }) => {
              has_explicit_width = true;
            }
            Some(ConstraintVariable::SelfHeight { .. }) => {
              has_explicit_height = true;
            }
            Some(ConstraintVariable::SelfX { .. }) => {
              has_explicit_x = true;
            }
            Some(ConstraintVariable::SelfY { .. }) => {
              has_explicit_y = true;
            }
            _ => {}
          }
        }
      }

      // By default the position is relative to the parent
      if !has_explicit_x {
        let element = &mut self.elements[element_id];
        element
          .constraints
          .extend(UserElementConstraints::relative_to_parent_horizontal(0.0).constraints);
      }
      if !has_explicit_y {
        let element = &mut self.elements[element_id];
        element
          .constraints
          .extend(UserElementConstraints::relative_to_parent_vertical(0.0).constraints);
      }

      // By default the size of a element covers the size of it's children
      if !has_explicit_width {
        let element = &mut self.elements[element_id];
        element
          .constraints
          .push(CompiledConstraint::TryAssumeMaxChildSize {
            dimension: Dimension::Width,
            constant_offset: 0.0,
          });
      }
      if !has_explicit_height {
        let element = &mut self.elements[element_id];
        element
          .constraints
          .push(CompiledConstraint::TryAssumeMaxChildSize {
            dimension: Dimension::Height,
            constant_offset: 0.0,
          });
      }
    }

    // TODO: Large allocation ahead, should be re-used across frames
    let mut solver = ConstraintResolver::new(
      self
        .elements
        .iter()
        .enumerate()
        .flat_map(|(id, element)| {
          element
            .constraints
            .iter()
            .cloned()
            .map(move |constraint| (id, constraint))
        })
        .collect(),
      self
        .elements
        .iter()
        .enumerate()
        .map(|(id, element)| RelationshipMeta {
          parent_id: element.parent_element,
          depth: element.depth,
          children: parent_children_relationship
            .get(&id)
            .cloned()
            .unwrap_or_default(),
        })
        .collect(),
      (width, height),
    );
    solver.resolve();

    let layouting_end = std::time::Instant::now();
    let rendering_start = std::time::Instant::now();

    let mut total_constraints = 0;
    for (id, element) in self.elements.iter().enumerate() {
      let x = solver.get_element_variable_resolution(id, ElementVariable::X);
      let y = solver.get_element_variable_resolution(id, ElementVariable::Y);
      let width = solver.get_element_variable_resolution(id, ElementVariable::Width);
      let height = solver.get_element_variable_resolution(id, ElementVariable::Height);
      total_constraints += element.constraints.len();

      if let Some(component) = &element.component {
        component.render(&mut RenderContext {
          layout: &ResolvedLayout {
            x: x as f32,
            y: y as f32,
            width: width as f32,
            height: height as f32,
          },
          theme,
          renderer,
          input_state,
          element_id: id,
          elements: &self.elements,
          mutable_state: &mut self.mutable_state,
        });
      } else {
        eprintln!("Allocated element has no component")
      }
    }

    let rendering_end = std::time::Instant::now();

    if self.debug_enabled {
      let debug_elements = self
        .elements
        .iter()
        .enumerate()
        .map(|(id, element)| {
          self.create_debug_element(&solver, &parent_children_relationship, element, id)
        })
        .collect::<Vec<_>>();
      self.debug_tree = Some(debug_elements);
      //self.print_debug_tree(0, self.debug_tree.as_ref().map(|tree| &tree[0]).unwrap());
    }

    OrchestratorStats {
      constrain_count: total_constraints,
      element_count: self.elements.len(),
      construction_duration: construction_end - construction_start,
      layout_duration: layouting_end - layouting_start,
      render_duration: rendering_end - rendering_start,
    }
  }

  fn print_debug_tree(&self, depth: usize, element: &DebugAllocatedElement) {
    println!(
      "{}{} (X:{}, Y:{}, W:{}, H:{})",
      " ".repeat(depth * 2),
      if element.component_name.is_empty() {
        "Unknown"
      } else {
        &element.component_name
      },
      element.layout.x,
      element.layout.y,
      element.layout.width,
      element.layout.height
    );
    for child in &element.children {
      self.print_debug_tree(
        depth + 1,
        self.debug_tree.as_ref().map(|tree| &tree[*child]).unwrap(),
      );
    }
  }

  fn create_debug_element(
    &self,
    solver: &ConstraintResolver,
    relationships: &HashMap<usize, Vec<usize>>,
    element: &AllocatedElement,
    element_id: usize,
  ) -> DebugAllocatedElement {
    let children = relationships.get(&element_id).unwrap();

    let layout = {
      let x = solver.get_element_variable_resolution(element_id, ElementVariable::X);
      let y = solver.get_element_variable_resolution(element_id, ElementVariable::Y);
      let width = solver.get_element_variable_resolution(element_id, ElementVariable::Width);
      let height = solver.get_element_variable_resolution(element_id, ElementVariable::Height);
      ResolvedLayout {
        x,
        y,
        width,
        height,
      }
    };

    let debug_info = format!("{:#?}", element.component);
    let component_name = format!("{:?}", element.component)
      .split("{")
      .next()
      .map(|name| {
        name
          .trim()
          .replace("Some(", "")
          .replace(")", "")
          .to_string()
      })
      .unwrap_or_else(|| debug_info.clone());

    DebugAllocatedElement {
      key: element.key.clone(),
      parent_id: element.parent_element,
      layout_constraints: element.constraints.clone(),
      id: element_id,
      layout,
      children: children.clone(),
      component_name,
      debug_info,
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Element {
  pub id: usize,
}

impl Element {
  pub fn x(&self) -> ConstraintVariable {
    ConstraintVariable::ElementX { id: self.id }
  }

  pub fn y(&self) -> ConstraintVariable {
    ConstraintVariable::ElementY { id: self.id }
  }

  pub fn width(&self) -> ConstraintVariable {
    ConstraintVariable::ElementWidth { id: self.id }
  }

  pub fn height(&self) -> ConstraintVariable {
    ConstraintVariable::ElementHeight { id: self.id }
  }
}

struct AllocatedElement {
  parent_element: Option<usize>,
  depth: usize,
  component: Option<Box<dyn Component>>,
  constraints: Vec<CompiledConstraint>,
  direct_child_component_occurrences: HashMap<TypeId, usize>,
  key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ComponentStateKey {
  key: String,
  parent_key: String,
  type_id: TypeId,
  name: String,
}

impl ComponentStateKey {
  pub fn new<T: Any + Clone + 'static>(
    elements: &[AllocatedElement],
    id: usize,
    name: &str,
  ) -> Self {
    let element = &elements[id];
    let parent_element = element.parent_element.map(|parent_id| &elements[parent_id]);

    ComponentStateKey {
      key: element.key.clone(),
      name: name.to_string(),
      parent_key: parent_element
        .map(|parent| parent.key.clone())
        .unwrap_or_default(),
      type_id: std::any::TypeId::of::<T>(),
    }
  }
}

#[derive(Debug, Clone)]
pub struct DebugAllocatedElement {
  pub id: usize,
  pub parent_id: Option<usize>,
  pub key: String,
  pub debug_info: String,
  pub component_name: String,
  pub layout: ResolvedLayout,
  pub layout_constraints: Vec<CompiledConstraint>,
  pub children: Vec<usize>,
}

pub struct ElementContext<'a> {
  elements: &'a mut Vec<AllocatedElement>,
  parent_element: Option<usize>,
  render_width: f32,
  render_height: f32,
  debug_enabled: bool,
  depth: usize,
  mutable_state: &'a mut HashMap<ComponentStateKey, Box<dyn Any>>,
  pub input_state: &'a InputState,
  pub prev_debug_nodes: &'a Option<Vec<DebugAllocatedElement>>,
  pub theme: &'a mut Theme,
  pub app: &'a mut App,
}

impl<'a> ElementContext<'a> {
  pub fn allocate_element<T: Component + 'static>(&mut self, component: T) -> Element {
    let id = self.elements.len();

    let mut key = "root".to_string();
    if let Some(parent_id) = self.parent_element {
      let parent = &mut self.elements[parent_id];
      let type_id = component.type_id();
      let index = *parent
        .direct_child_component_occurrences
        .entry(type_id)
        .or_insert(0);
      *parent
        .direct_child_component_occurrences
        .get_mut(&type_id)
        .unwrap() += 1;
      key = format!("{:?}{}", type_id, index);
    }

    self.elements.push(AllocatedElement {
      parent_element: self.parent_element,
      component: Some(Box::new(component)),
      constraints: Vec::new(),
      depth: self.depth,
      key,
      direct_child_component_occurrences: HashMap::new(),
    });
    Element { id }
  }

  pub fn construct_element(&mut self, element: &Element) {
    let mut component = self.elements[element.id].component.take().unwrap();
    component.construct(&mut ElementContext {
      elements: self.elements,
      depth: self.depth,
      debug_enabled: self.debug_enabled,
      parent_element: Some(element.id),
      render_height: self.render_height,
      render_width: self.render_width,
      prev_debug_nodes: self.prev_debug_nodes,
      mutable_state: self.mutable_state,
      theme: self.theme,
      app: self.app,
      input_state: self.input_state,
    });
    self.elements[element.id].component = Some(component);
  }

  pub fn get_context_for_child(&mut self, parent_element: &Element) -> ElementContext<'_> {
    ElementContext {
      elements: self.elements,
      depth: self.depth + 1,
      parent_element: Some(parent_element.id),
      render_height: self.render_height,
      render_width: self.render_width,
      debug_enabled: self.debug_enabled,
      prev_debug_nodes: self.prev_debug_nodes,
      mutable_state: self.mutable_state,
      theme: self.theme,
      app: self.app,
      input_state: self.input_state,
    }
  }

  pub fn current_element_id(&self) -> usize {
    self.parent_element.unwrap_or_default()
  }

  pub fn set_parent_element_constraints(&mut self, constraints: Vec<CompiledConstraint>) {
    if let Some(parent_id) = self.parent_element {
      self.set_element_constraints(&Element { id: parent_id }, constraints);
    }
  }

  pub fn set_element_constraints(
    &mut self,
    element: &Element,
    constraints: Vec<CompiledConstraint>,
  ) {
    let id = element.id;
    let element = self.elements.get_mut(id).unwrap();
    element.constraints.extend(constraints);
  }
}

pub trait StatefulContext {
  fn prepare_and_get_state<T: Any + Clone + 'static, FInit: FnOnce() -> T>(
    &mut self,
    initializer: FInit,
    name: &str,
  ) -> T;

  fn set_state<T: Any + Clone + 'static>(&mut self, element_id: usize, name: &str, value: T);
}

impl<'a> StatefulContext for ElementContext<'a> {
  fn prepare_and_get_state<T: Any + Clone + 'static, FInit: FnOnce() -> T>(
    &mut self,
    initializer: FInit,
    name: &str,
  ) -> T {
    let state_key = ComponentStateKey::new::<T>(self.elements, self.parent_element.unwrap(), name);

    if !self.mutable_state.contains_key(&state_key) {
      self
        .mutable_state
        .insert(state_key.clone(), Box::new(initializer()));
    }

    let boxed_state = self.mutable_state.get(&state_key).unwrap();
    boxed_state.downcast_ref::<T>().unwrap().clone()
  }

  fn set_state<T: Any + Clone + 'static>(&mut self, element_id: usize, name: &str, value: T) {
    let state_key = ComponentStateKey::new::<T>(self.elements, element_id, name);
    self.mutable_state.insert(state_key, Box::new(value));
  }
}

pub struct RenderContext<'a> {
  pub layout: &'a ResolvedLayout,
  pub renderer: &'a mut dyn Renderer,
  pub theme: &'a mut Theme,
  pub input_state: &'a InputState,
  elements: &'a [AllocatedElement],
  element_id: usize,
  mutable_state: &'a mut HashMap<ComponentStateKey, Box<dyn Any>>,
}

impl<'a> StatefulContext for RenderContext<'a> {
  fn prepare_and_get_state<T: Any + Clone + 'static, FInit: FnOnce() -> T>(
    &mut self,
    initializer: FInit,
    name: &str,
  ) -> T {
    let state_key = ComponentStateKey::new::<T>(self.elements, self.element_id, name);

    if !self.mutable_state.contains_key(&state_key) {
      self
        .mutable_state
        .insert(state_key.clone(), Box::new(initializer()));
    }

    let boxed_state = self.mutable_state.get(&state_key).unwrap();
    boxed_state.downcast_ref::<T>().unwrap().clone()
  }

  fn set_state<T: Any + Clone + 'static>(&mut self, element_id: usize, name: &str, value: T) {
    let state_key = ComponentStateKey::new::<T>(self.elements, element_id, name);
    self.mutable_state.insert(state_key, Box::new(value));
  }
}

pub struct OrchestratorStats {
  pub construction_duration: Duration,
  pub layout_duration: Duration,
  pub render_duration: Duration,
  pub element_count: usize,
  pub constrain_count: usize,
}
