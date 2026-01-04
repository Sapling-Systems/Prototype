use std::{
  any::{Any, TypeId},
  collections::HashMap,
};

use kasuari::{
  Constraint as KasuariConstraint, Expression, RelationalOperator as KasuariRelationalOperator,
  Solver as KasuariSolver, Strength, Term as KasuariTerm, Variable as KasuariVariable,
};
use sapling_app::App;

use crate::{
  component::Component,
  input::InputState,
  layout::{
    ElementConstraint, ElementConstraintOperator, ElementConstraintVariable, ElementConstraints,
    ResolvedLayout,
  },
  prelude::{RenderContext, Renderer},
  theme::Theme,
};

pub struct Orchestrator {
  elements: Vec<AllocatedElement>,
  root_vars: RootVars,
  debug_enabled: bool,
  debug_tree: Option<DebugAllocatedElement>,
  mutable_state: HashMap<ComponentStateKey, Box<dyn Any>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ComponentStateKey {
  key: String,
  parent_key: String,
  type_id: TypeId,
}

#[derive(Debug, Clone)]
pub struct DebugAllocatedElement {
  pub id: usize,
  pub debug_info: String,
  pub component_name: String,
  pub layout: ResolvedLayout,
  pub children: Vec<DebugAllocatedElement>,
}

struct RootVars {
  render_left: KasuariVariable,
  render_top: KasuariVariable,
  render_right: KasuariVariable,
  render_bottom: KasuariVariable,
}

impl Orchestrator {
  pub fn new(debug_enabled: bool) -> Self {
    Orchestrator {
      root_vars: RootVars {
        render_left: KasuariVariable::new(),
        render_top: KasuariVariable::new(),
        render_right: KasuariVariable::new(),
        render_bottom: KasuariVariable::new(),
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
  ) -> (std::time::Duration, std::time::Duration) {
    let layouting_start = std::time::Instant::now();
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
      layout_vars: AllocatedElementLayoutVars {
        self_bottom: self.root_vars.render_bottom,
        self_right: self.root_vars.render_right,
        self_left: self.root_vars.render_left,
        self_top: self.root_vars.render_top,
      },
    });
    let element = self.elements.last_mut().unwrap();
    let mut component = element.component.take().unwrap();

    component.construct(&mut ElementContext {
      parent_element: Some(0),
      elements: &mut self.elements,
      root_vars: &self.root_vars,
      render_width: width,
      render_height: height,
      prev_debug_node: &self.debug_tree,
      theme,
      app,
    });

    let element = &mut self.elements[0];
    element.component = Some(component);

    let mut solver = KasuariSolver::new();
    let const_constraints = solver.add_constraints([
      self.root_vars.render_left | EQ(Strength::REQUIRED) | 0.0,
      self.root_vars.render_top | EQ(Strength::REQUIRED) | 0.0,
      self.root_vars.render_right | EQ(Strength::REQUIRED) | width,
      self.root_vars.render_bottom | EQ(Strength::REQUIRED) | height,
    ]);
    if let Err(err) = const_constraints {
      eprintln!("Solver error on root constraints: {}", err);
    }

    // Set parent coverage layout for elements without any constraints
    for element_id in 0..self.elements.len() {
      let element = &self.elements[element_id];
      if element.constraints.is_empty() {
        let mut element_context = ElementContext {
          app,
          theme,
          parent_element: element.parent_element,
          render_height: height,
          render_width: width,
          root_vars: &self.root_vars,
          elements: &mut self.elements,
          prev_debug_node: &self.debug_tree,
        };
        element_context.set_element_constraints(
          &Element { id: element_id },
          ElementConstraints::cover_parent().constraints,
        );
      }
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

    let rendering_start = std::time::Instant::now();
    let layouting_duration = rendering_start - layouting_start;

    for element in self.elements.iter() {
      let bottom = solver.get_value(element.layout_vars.self_bottom);
      let right = solver.get_value(element.layout_vars.self_right);
      let left = solver.get_value(element.layout_vars.self_left);
      let top = solver.get_value(element.layout_vars.self_top);

      if let Some(component) = &element.component {
        component.render(&mut RenderContext {
          layout: &ResolvedLayout {
            width: (right - left) as f32,
            height: (bottom - top) as f32,
            x: left as f32,
            y: top as f32,
          },
          theme,
          renderer,
          input_state,
        });
      } else {
        eprintln!("Allocated element has no component")
      }
    }

    let rendering_duration = std::time::Instant::now() - rendering_start;

    if self.debug_enabled {
      // Build relationship tree
      let mut element_tree: HashMap<usize, Vec<usize>> = HashMap::new();
      for (index, _) in self.elements.iter().enumerate() {
        element_tree.insert(index, Vec::new());
      }
      for (id, element) in self.elements.iter().enumerate() {
        if let Some(parent) = element.parent_element {
          element_tree.get_mut(&parent).unwrap().push(id);
        }
      }
      self.debug_tree =
        Some(self.create_debug_element(&solver, &element_tree, &self.elements[0], 0));
    }

    (rendering_duration, layouting_duration)
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
      let bottom = solver.get_value(element.layout_vars.self_bottom) as f32;
      let right = solver.get_value(element.layout_vars.self_right) as f32;
      let left = solver.get_value(element.layout_vars.self_left) as f32;
      let top = solver.get_value(element.layout_vars.self_top) as f32;
      ResolvedLayout {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
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
      id: element_id,
      debug_info,
      component_name,
      layout,
      children: children
        .iter()
        .flat_map(|child| {
          let element = &self.elements[*child];
          if format!("{:?}", element.component).contains("Debugger") {
            return None;
          }
          Some(self.create_debug_element(solver, relationships, element, *child))
        })
        .collect(),
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Element {
  pub id: usize,
}

impl Element {
  pub fn left(&self) -> ElementConstraintVariable {
    ElementConstraintVariable::ElementLeft(*self)
  }

  pub fn right(&self) -> ElementConstraintVariable {
    ElementConstraintVariable::ElementRight(*self)
  }

  pub fn top(&self) -> ElementConstraintVariable {
    ElementConstraintVariable::ElementTop(*self)
  }

  pub fn bottom(&self) -> ElementConstraintVariable {
    ElementConstraintVariable::ElementBottom(*self)
  }
}

pub struct ElementContext<'a> {
  elements: &'a mut Vec<AllocatedElement>,
  parent_element: Option<usize>,
  root_vars: &'a RootVars,
  render_width: f32,
  render_height: f32,
  pub prev_debug_node: &'a Option<DebugAllocatedElement>,
  pub theme: &'a mut Theme,
  pub app: &'a mut App,
}

struct AllocatedElement {
  parent_element: Option<usize>,
  component: Option<Box<dyn Component>>,
  constraints: Vec<KasuariConstraint>,
  layout_vars: AllocatedElementLayoutVars,
  direct_child_component_occurrences: HashMap<TypeId, usize>,
  key: String,
}

struct AllocatedElementLayoutVars {
  self_left: KasuariVariable,
  self_top: KasuariVariable,
  self_right: KasuariVariable,
  self_bottom: KasuariVariable,
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
      key,
      direct_child_component_occurrences: HashMap::new(),
      layout_vars: AllocatedElementLayoutVars {
        self_left: KasuariVariable::new(),
        self_top: KasuariVariable::new(),
        self_right: KasuariVariable::new(),
        self_bottom: KasuariVariable::new(),
      },
    });
    Element { id }
  }

  pub fn construct_element(&mut self, element: &Element) {
    let mut component = self.elements[element.id].component.take().unwrap();
    component.construct(&mut ElementContext {
      elements: self.elements,
      parent_element: Some(element.id),
      render_height: self.render_height,
      render_width: self.render_width,
      root_vars: self.root_vars,
      prev_debug_node: self.prev_debug_node,
      theme: self.theme,
      app: self.app,
    });
    self.elements[element.id].component = Some(component);
  }

  pub fn get_context_for_child(&mut self, parent_element: &Element) -> ElementContext<'_> {
    ElementContext {
      elements: self.elements,
      parent_element: Some(parent_element.id),
      render_height: self.render_height,
      render_width: self.render_width,
      root_vars: self.root_vars,
      prev_debug_node: self.prev_debug_node,
      theme: self.theme,
      app: self.app,
    }
  }

  pub fn set_element_constraints(
    &mut self,
    element: &Element,
    constraints: Vec<ElementConstraint>,
  ) {
    let id = element.id;
    let element = self.elements.get(id).unwrap();
    let constraints = constraints
      .into_iter()
      .map(|constraint| self.map_constraint(constraint, element))
      .collect::<Vec<_>>();

    let element = self.elements.get_mut(id).unwrap();
    element.constraints.extend(constraints);
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
      ElementConstraintVariable::ScreenWidth => self.root_vars.render_right,
      ElementConstraintVariable::ScreenHeight => self.root_vars.render_bottom,
      // Self
      ElementConstraintVariable::SelfLeft => element.layout_vars.self_left,
      ElementConstraintVariable::SelfTop => element.layout_vars.self_top,
      ElementConstraintVariable::SelfRight => element.layout_vars.self_right,
      ElementConstraintVariable::SelfBottom => element.layout_vars.self_bottom,
      // Parent
      ElementConstraintVariable::ParentLeft => parent_element
        .map(|parent| parent.layout_vars.self_left)
        .unwrap_or_else(|| self.root_vars.render_left),
      ElementConstraintVariable::ParentTop => parent_element
        .map(|parent| parent.layout_vars.self_top)
        .unwrap_or_else(|| self.root_vars.render_top),
      ElementConstraintVariable::ParentRight => parent_element
        .map(|parent| parent.layout_vars.self_right)
        .unwrap_or_else(|| self.root_vars.render_right),
      ElementConstraintVariable::ParentBottom => parent_element
        .map(|parent| parent.layout_vars.self_bottom)
        .unwrap_or_else(|| self.root_vars.render_bottom),
      // Other Elements
      ElementConstraintVariable::ElementLeft(element) => self
        .elements
        .get(element.id)
        .map(|element| element.layout_vars.self_left)
        .or_else(|| parent_element.map(|parent| parent.layout_vars.self_left))
        .unwrap_or(self.root_vars.render_left),
      ElementConstraintVariable::ElementTop(element) => self
        .elements
        .get(element.id)
        .map(|element| element.layout_vars.self_top)
        .or_else(|| parent_element.map(|parent| parent.layout_vars.self_top))
        .unwrap_or(self.root_vars.render_top),
      ElementConstraintVariable::ElementRight(element) => self
        .elements
        .get(element.id)
        .map(|element| element.layout_vars.self_right)
        .or_else(|| parent_element.map(|parent| parent.layout_vars.self_right))
        .unwrap_or(self.root_vars.render_right),
      ElementConstraintVariable::ElementBottom(element) => self
        .elements
        .get(element.id)
        .map(|element| element.layout_vars.self_bottom)
        .or_else(|| parent_element.map(|parent| parent.layout_vars.self_bottom))
        .unwrap_or(self.root_vars.render_bottom),
    }
  }
}
