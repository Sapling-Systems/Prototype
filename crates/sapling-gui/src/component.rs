use std::{any::Any, fmt::Debug};

use crate::{
  layout::UserElementConstraints,
  orchestrator::{Element, ElementContext},
  prelude::RenderContext,
};

pub struct LayoutedComponent<T: ComponentElement> {
  layout_constraints: Vec<UserElementConstraints>,
  component: T,
}

pub struct ParentComponent<T: ComponentElement> {
  children: Box<dyn FnOnce(&mut ElementContext)>,
  component: T,
}

pub trait ComponentElement: Sized + Debug + 'static {
  fn with_children<F: FnOnce(&mut ElementContext) + 'static>(
    self,
    factory: F,
  ) -> ParentComponent<Self> {
    ParentComponent {
      component: self,
      children: Box::new(factory),
    }
  }

  fn with_layout(self, layout_constraints: Vec<UserElementConstraints>) -> LayoutedComponent<Self> {
    LayoutedComponent {
      layout_constraints,
      component: self,
    }
  }

  fn build(self, context: &mut ElementContext) -> Element;
}

pub trait Component: Debug + Any {
  fn construct(&mut self, _context: &mut ElementContext) {}
  fn render(&self, _context: &mut RenderContext) {}
}

impl<T: ComponentElement> ComponentElement for LayoutedComponent<T> {
  fn build(self, context: &mut ElementContext) -> Element {
    let element = self.component.build(context);
    context.set_element_constraints(
      &element,
      self
        .layout_constraints
        .into_iter()
        .flat_map(|c| c.constraints.into_iter())
        .collect(),
    );
    element
  }
}

impl<T: ComponentElement> std::fmt::Debug for LayoutedComponent<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("LayoutedComponent")
      .field("component", &self.component)
      .finish()
  }
}

impl<T: ComponentElement> ComponentElement for ParentComponent<T> {
  fn build(self, context: &mut ElementContext) -> Element {
    let element = self.component.build(context);
    let mut child_context = context.get_context_for_child(&element);
    (self.children)(&mut child_context);
    element
  }
}

impl<T: ComponentElement> std::fmt::Debug for ParentComponent<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ParentComponent")
      .field("component", &self.component)
      .finish()
  }
}

impl<T: Component + 'static> ComponentElement for T {
  fn build(self, context: &mut ElementContext) -> Element {
    let element = context.allocate_element(self);
    context.construct_element(&element);
    element
  }
}

pub type ChildrenProperty = Option<Box<dyn FnOnce(&mut ElementContext)>>;
