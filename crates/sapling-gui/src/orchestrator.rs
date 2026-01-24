use std::{
  any::{Any, TypeId},
  collections::HashMap,
  time::{Duration, Instant},
};

use kasuari::{
  Constraint as KasuariConstraint, Expression, RelationalOperator as KasuariRelationalOperator,
  Solver as KasuariSolver, Strength, Term as KasuariTerm, Variable as KasuariVariable,
  WeightedRelation::*,
};
use sapling_app::App;
use sapling_gui_macro::constraint1;

use crate::{
  component::Component,
  input::InputState,
  layout::{
    ElementConstraint, ElementConstraintExpression, ElementConstraintOperator,
    ElementConstraintVariable, ElementConstraints, ResolvedLayout,
  },
  prelude::Renderer,
  theme::Theme,
};

pub struct Orchestrator {
  elements: Vec<AllocatedElement>,
  root_vars: RootVars,
  debug_enabled: bool,
  debug_tree: Option<Vec<DebugAllocatedElement>>,
  mutable_state: HashMap<ComponentStateKey, Box<dyn Any>>,
}

impl Orchestrator {
  pub fn new(debug_enabled: bool) -> Self {
    Orchestrator {
      root_vars: RootVars {
        render_x: KasuariVariable::new(),
        render_y: KasuariVariable::new(),
        render_width: KasuariVariable::new(),
        render_height: KasuariVariable::new(),
      },
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

    use kasuari::Strength;
    use kasuari::WeightedRelation::*;

    // construction phase
    self.elements.push(AllocatedElement {
      parent_element: None,
      key: "root".to_string(),
      component: Some(Box::new(root)),
      direct_child_component_occurrences: HashMap::new(),
      constraints: vec![],
      debug_constraints: None,
      layout_vars: AllocatedElementLayoutVars {
        self_x: self.root_vars.render_x,
        self_y: self.root_vars.render_y,
        self_width: self.root_vars.render_width,
        self_height: self.root_vars.render_height,
        has_height_constraints: true,
        has_width_constraints: true,
      },
    });
    let element = self.elements.last_mut().unwrap();
    let mut component = element.component.take().unwrap();

    component.construct(&mut ElementContext {
      parent_element: Some(0),
      elements: &mut self.elements,
      mutable_state: &mut self.mutable_state,
      root_vars: &self.root_vars,
      debug_enabled: self.debug_enabled,
      render_width: width,
      render_height: height,
      prev_debug_nodes: &self.debug_tree,
      input_state,
      theme,
      app,
    });

    let element = &mut self.elements[0];
    element.component = Some(component);

    let construction_end = std::time::Instant::now();
    let layouting_start = std::time::Instant::now();
    let mut solver = KasuariSolver::new();
    let const_constraints = solver.add_constraints([
      self.root_vars.render_x | EQ(Strength::REQUIRED) | 0.0,
      self.root_vars.render_y | EQ(Strength::REQUIRED) | 0.0,
      self.root_vars.render_width | EQ(Strength::REQUIRED) | width,
      self.root_vars.render_height | EQ(Strength::REQUIRED) | height,
    ]);
    if let Err(err) = const_constraints {
      eprintln!("Solver error on root constraints: {}", err);
    }

    // Create tree info
    let mut parent_children_relationship: HashMap<usize, Vec<usize>> = HashMap::new();
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

    // Set parent coverage layout as a weak constraint for defaulting
    for element_id in 0..self.elements.len() {
      let mut additional_constraints = vec![];
      additional_constraints.extend(ElementConstraints::relative_position().weak().constraints);

      let element = &self.elements[element_id];
      let (has_width_constraint, has_height_constraint) = (
        element.layout_vars.has_width_constraints,
        element.layout_vars.has_height_constraints,
      );

      // Automatically grow parent to be children's size if not further restrictred
      if !has_width_constraint {
        for child_id in parent_children_relationship.get(&element_id).unwrap() {
          let child_x = ElementConstraintVariable::ElementX(Element { id: *child_id });
          let child_width = ElementConstraintVariable::ElementWidth(Element { id: *child_id });
          additional_constraints.push(constraint1!(
            self_x + self_width >= child_x + child_width,
            strength = Strength::STRONG.value() as f32
          ));
          additional_constraints.push(constraint1!(
            self_width == 0,
            strength = Strength::WEAK.value() as f32
          ));
        }
      }
      if !has_height_constraint {
        for child_id in parent_children_relationship.get(&element_id).unwrap() {
          let child_y = ElementConstraintVariable::ElementY(Element { id: *child_id });
          let child_height = ElementConstraintVariable::ElementHeight(Element { id: *child_id });
          additional_constraints.push(constraint1!(
            self_y + self_height >= child_y + child_height,
            strength = Strength::STRONG.value() as f32
          ));
        }
      }

      let mut element_context = ElementContext {
        app,
        theme,
        parent_element: element.parent_element,
        mutable_state: &mut self.mutable_state,
        render_height: height,
        render_width: width,
        debug_enabled: self.debug_enabled,
        root_vars: &self.root_vars,
        elements: &mut self.elements,
        prev_debug_nodes: &self.debug_tree,
        input_state,
      };
      element_context.set_element_constraints(&Element { id: element_id }, additional_constraints);
    }

    let element_constraints = self
      .elements
      .iter()
      .flat_map(|element| element.constraints.iter().cloned())
      .collect::<Vec<_>>();

    let element_constraints = solver.add_constraints(element_constraints);
    if let Err(err) = element_constraints {
      eprintln!("Solver error on element constraints: {:?}", err);
    }

    let layouting_end = std::time::Instant::now();
    let rendering_start = std::time::Instant::now();

    let mut total_constraints = 0;
    for (id, element) in self.elements.iter().enumerate() {
      let x = solver.get_value(element.layout_vars.self_x);
      let y = solver.get_value(element.layout_vars.self_y);
      let width = solver.get_value(element.layout_vars.self_width);
      let height = solver.get_value(element.layout_vars.self_height);
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
    }

    OrchestratorStats {
      constrain_count: total_constraints,
      element_count: self.elements.len(),
      construction_duration: construction_end - construction_start,
      layout_duration: layouting_end - layouting_start,
      render_duration: rendering_end - rendering_start,
    }
  }

  fn create_debug_element(
    &self,
    solver: &KasuariSolver,
    relationships: &HashMap<usize, Vec<usize>>,
    element: &AllocatedElement,
    element_id: usize,
  ) -> DebugAllocatedElement {
    let children = relationships.get(&element_id).unwrap();

    let layout = {
      let x = solver.get_value(element.layout_vars.self_x) as f32;
      let y = solver.get_value(element.layout_vars.self_y) as f32;
      let width = solver.get_value(element.layout_vars.self_width) as f32;
      let height = solver.get_value(element.layout_vars.self_height) as f32;
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
      layout_constraints: element.debug_constraints.clone().unwrap(),
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
  pub fn x(&self) -> ElementConstraintVariable {
    ElementConstraintVariable::ElementX(*self)
  }

  pub fn y(&self) -> ElementConstraintVariable {
    ElementConstraintVariable::ElementY(*self)
  }

  pub fn width(&self) -> ElementConstraintVariable {
    ElementConstraintVariable::ElementWidth(*self)
  }

  pub fn height(&self) -> ElementConstraintVariable {
    ElementConstraintVariable::ElementHeight(*self)
  }
}

struct AllocatedElement {
  parent_element: Option<usize>,
  component: Option<Box<dyn Component>>,
  constraints: Vec<KasuariConstraint>,
  debug_constraints: Option<ElementConstraints>,
  layout_vars: AllocatedElementLayoutVars,
  direct_child_component_occurrences: HashMap<TypeId, usize>,
  key: String,
}

struct AllocatedElementLayoutVars {
  self_x: KasuariVariable,
  self_y: KasuariVariable,
  self_width: KasuariVariable,
  self_height: KasuariVariable,
  has_width_constraints: bool,
  has_height_constraints: bool,
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
  pub layout_constraints: ElementConstraints,
  pub children: Vec<usize>,
}

struct RootVars {
  render_x: KasuariVariable,
  render_y: KasuariVariable,
  render_width: KasuariVariable,
  render_height: KasuariVariable,
}

pub struct ElementContext<'a> {
  elements: &'a mut Vec<AllocatedElement>,
  parent_element: Option<usize>,
  root_vars: &'a RootVars,
  render_width: f32,
  render_height: f32,
  debug_enabled: bool,
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
      debug_constraints: None,
      constraints: Vec::new(),
      key,
      direct_child_component_occurrences: HashMap::new(),
      layout_vars: AllocatedElementLayoutVars {
        self_x: KasuariVariable::new(),
        self_y: KasuariVariable::new(),
        self_width: KasuariVariable::new(),
        self_height: KasuariVariable::new(),
        has_height_constraints: false,
        has_width_constraints: false,
      },
    });
    Element { id }
  }

  pub fn construct_element(&mut self, element: &Element) {
    let mut component = self.elements[element.id].component.take().unwrap();
    component.construct(&mut ElementContext {
      elements: self.elements,
      debug_enabled: self.debug_enabled,
      parent_element: Some(element.id),
      render_height: self.render_height,
      render_width: self.render_width,
      root_vars: self.root_vars,
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
      parent_element: Some(parent_element.id),
      render_height: self.render_height,
      render_width: self.render_width,
      debug_enabled: self.debug_enabled,
      root_vars: self.root_vars,
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

  pub fn set_parent_element_constraints(&mut self, constraints: Vec<ElementConstraint>) {
    if let Some(parent_id) = self.parent_element {
      self.set_element_constraints(&Element { id: parent_id }, constraints);
    }
  }

  pub fn set_element_constraints(
    &mut self,
    element: &Element,
    constraints: Vec<ElementConstraint>,
  ) {
    if self.debug_enabled {
      let element = self.elements.get_mut(element.id).unwrap();
      if element.debug_constraints.is_none() {
        element.debug_constraints = Some(ElementConstraints {
          constraints: vec![],
        });
      }
      let debug_constraints = element.debug_constraints.as_mut().unwrap();
      debug_constraints.constraints.extend(constraints.clone());
    }

    let (has_width_constraints, has_height_constraints) = {
      constraints
        .iter()
        .fold((false, false), |(has_width, has_height), constraint| {
          constraint.expression.terms.iter().fold(
            (has_width, has_height),
            |(has_width, has_height), term| match term.variable {
              ElementConstraintVariable::SelfWidth => (true, has_height),
              ElementConstraintVariable::SelfHeight => (has_width, true),
              _ => (has_width, has_height),
            },
          )
        })
    };

    let id = element.id;
    let element = self.elements.get(id).unwrap();

    let constraints = constraints
      .into_iter()
      .map(|constraint| self.map_constraint(constraint, element))
      .collect::<Vec<_>>();

    let element = self.elements.get_mut(id).unwrap();

    // Insert default system constraints
    if element.constraints.is_empty() {
      // Ensure that elements are always positive width & height or 0
      element
        .constraints
        .push(element.layout_vars.self_width | GE(Strength::REQUIRED) | 0.0);
      element
        .constraints
        .push(element.layout_vars.self_height | GE(Strength::REQUIRED) | 0.0);
    }

    element.constraints.extend(constraints);
    if has_height_constraints {
      element.layout_vars.has_height_constraints = true;
    }
    if has_width_constraints {
      element.layout_vars.has_width_constraints = true;
    }
  }

  fn map_constraint(
    &self,
    constraint: ElementConstraint,
    element: &AllocatedElement,
  ) -> KasuariConstraint {
    let expression = Expression::new(
      constraint
        .expression
        .terms
        .into_iter()
        .map(|term| {
          KasuariTerm::new(
            self.map_constraint_variable(&term.variable, element),
            term.coefficient as f64,
          )
        })
        .collect(),
      constraint.expression.constant as f64,
    );

    let operator = match constraint.operator {
      ElementConstraintOperator::Equal => KasuariRelationalOperator::Equal,
      ElementConstraintOperator::GreaterOrEqual => KasuariRelationalOperator::GreaterOrEqual,
      ElementConstraintOperator::LessOrEqual => KasuariRelationalOperator::LessOrEqual,
    };

    KasuariConstraint::new(
      expression,
      operator,
      Strength::new(constraint.strength as f64),
    )
  }

  fn map_constraint_variable(
    &self,
    variable: &ElementConstraintVariable,
    element: &AllocatedElement,
  ) -> KasuariVariable {
    let parent_element = element.parent_element.and_then(|id| self.elements.get(id));
    match variable {
      // Screen
      ElementConstraintVariable::ScreenWidth => self.root_vars.render_width,
      ElementConstraintVariable::ScreenHeight => self.root_vars.render_height,
      // Self
      ElementConstraintVariable::SelfX => element.layout_vars.self_x,
      ElementConstraintVariable::SelfY => element.layout_vars.self_y,
      ElementConstraintVariable::SelfWidth => element.layout_vars.self_width,
      ElementConstraintVariable::SelfHeight => element.layout_vars.self_height,
      // Parent
      ElementConstraintVariable::ParentX => parent_element
        .map(|parent| parent.layout_vars.self_x)
        .unwrap_or_else(|| self.root_vars.render_x),
      ElementConstraintVariable::ParentY => parent_element
        .map(|parent| parent.layout_vars.self_y)
        .unwrap_or_else(|| self.root_vars.render_y),
      ElementConstraintVariable::ParentWidth => parent_element
        .map(|parent| parent.layout_vars.self_width)
        .unwrap_or_else(|| self.root_vars.render_width),
      ElementConstraintVariable::ParentHeight => parent_element
        .map(|parent| parent.layout_vars.self_height)
        .unwrap_or_else(|| self.root_vars.render_height),
      // Other Elements
      ElementConstraintVariable::ElementX(element) => self
        .elements
        .get(element.id)
        .map(|element| element.layout_vars.self_x)
        .or_else(|| parent_element.map(|parent| parent.layout_vars.self_x))
        .unwrap_or(self.root_vars.render_x),
      ElementConstraintVariable::ElementY(element) => self
        .elements
        .get(element.id)
        .map(|element| element.layout_vars.self_y)
        .or_else(|| parent_element.map(|parent| parent.layout_vars.self_y))
        .unwrap_or(self.root_vars.render_y),
      ElementConstraintVariable::ElementWidth(element) => self
        .elements
        .get(element.id)
        .map(|element| element.layout_vars.self_width)
        .or_else(|| parent_element.map(|parent| parent.layout_vars.self_width))
        .unwrap_or(self.root_vars.render_width),
      ElementConstraintVariable::ElementHeight(element) => self
        .elements
        .get(element.id)
        .map(|element| element.layout_vars.self_height)
        .or_else(|| parent_element.map(|parent| parent.layout_vars.self_height))
        .unwrap_or(self.root_vars.render_height),
    }
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
