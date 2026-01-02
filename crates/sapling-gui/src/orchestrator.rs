use kasuari::{
  Constraint as KasuariConstraint, Expression, RelationalOperator as KasuariRelationalOperator,
  Solver as KasuariSolver, Strength, Term as KasuariTerm, Variable as KasuariVariable,
};

use crate::{
  component::Component,
  layout::{
    ElementConstraint, ElementConstraintOperator, ElementConstraintVariable, ResolvedLayout,
  },
};

pub struct Orchestrator {
  elements: Vec<AllocatedElement>,
  root_vars: RootVars,
}

struct RootVars {
  render_left: KasuariVariable,
  render_top: KasuariVariable,
  render_right: KasuariVariable,
  render_bottom: KasuariVariable,
}

impl Orchestrator {
  pub fn new() -> Self {
    Orchestrator {
      root_vars: RootVars {
        render_left: KasuariVariable::new(),
        render_top: KasuariVariable::new(),
        render_right: KasuariVariable::new(),
        render_bottom: KasuariVariable::new(),
      },
      elements: Vec::new(),
    }
  }

  pub fn construct_and_render<T: Component + 'static>(&mut self, root: T, width: f32, height: f32) {
    self.elements.clear();

    use kasuari::Strength;
    use kasuari::WeightedRelation::*;

    // construction phase
    self.elements.push(AllocatedElement {
      parent_element: None,
      component: Some(Box::new(root)),
      constraints: vec![],
      layout_vars: AllocatedElementLayoutVars {
        self_bottom: self.root_vars.render_bottom,
        self_right: self.root_vars.render_right,
        self_left: self.root_vars.render_left,
        self_top: self.root_vars.render_top,
      },
    });
    let element = self.elements.last_mut().unwrap();
    let component = element.component.take().unwrap();

    component.construct(&mut ElementContext {
      parent_element: Some(0),
      elements: &mut self.elements,
      root_vars: &self.root_vars,
      render_width: width,
      render_height: height,
    });

    let element = &mut self.elements[0];
    element.component = Some(component);

    println!("Construction ended with {} elements", self.elements.len());

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

    let element_constraints = self
      .elements
      .iter()
      .flat_map(|element| element.constraints.iter().cloned())
      .collect::<Vec<_>>();

    let element_constraints = solver.add_constraints(element_constraints);
    if let Err(err) = element_constraints {
      eprintln!("Solver error on element constraints: {:?}", err);
    }

    for (id, element) in self.elements.iter().enumerate() {
      let bottom = solver.get_value(element.layout_vars.self_bottom);
      let right = solver.get_value(element.layout_vars.self_right);
      let left = solver.get_value(element.layout_vars.self_left);
      let top = solver.get_value(element.layout_vars.self_top);

      println!(
        "Element {:?} (parent: {:?}) has bounds ({}, {}, {}, {})",
        id, element.parent_element, left, top, right, bottom
      );

      if let Some(component) = &element.component {
        component.render(&ResolvedLayout {
          width: (right - left) as f32,
          height: (bottom - top) as f32,
          x: left as f32,
          y: top as f32,
        });
      } else {
        eprintln!("Allocated element has no component")
      }
    }
  }
}

pub struct Element {
  id: usize,
}

pub struct ElementContext<'a> {
  elements: &'a mut Vec<AllocatedElement>,
  parent_element: Option<usize>,
  root_vars: &'a RootVars,
  render_width: f32,
  render_height: f32,
}

struct AllocatedElement {
  parent_element: Option<usize>,
  component: Option<Box<dyn Component>>,
  constraints: Vec<KasuariConstraint>,
  layout_vars: AllocatedElementLayoutVars,
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
    self.elements.push(AllocatedElement {
      parent_element: self.parent_element,
      component: Some(Box::new(component)),
      constraints: Vec::new(),
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
    let component = self.elements[element.id].component.take().unwrap();
    component.construct(&mut ElementContext {
      elements: self.elements,
      parent_element: Some(element.id),
      render_height: self.render_height,
      render_width: self.render_width,
      root_vars: self.root_vars,
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

    println!(
      "Setting constraints on element {} (parent = {:?})",
      id, element.parent_element
    );
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
      ElementConstraintVariable::SelfLeft => element.layout_vars.self_left,
      ElementConstraintVariable::SelfTop => element.layout_vars.self_top,
      ElementConstraintVariable::SelfRight => element.layout_vars.self_right,
      ElementConstraintVariable::SelfBottom => element.layout_vars.self_bottom,
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
    }
  }
}
