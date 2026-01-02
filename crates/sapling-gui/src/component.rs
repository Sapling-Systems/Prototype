use crate::{
  layout::{ElementConstraint, ElementConstraints, ResolvedLayout},
  orchestrator::{Element, ElementContext},
  prelude::Renderer,
  theme::Theme,
};

pub struct LayoutedComponent<T: ComponentElement> {
  layout_constraints: Vec<ElementConstraints>,
  component: T,
}

pub struct ParentComponent<T: ComponentElement> {
  children: Box<dyn FnOnce(&mut ElementContext)>,
  component: T,
}

pub trait ComponentElement: Sized {
  fn with_children<F: FnOnce(&mut ElementContext) + 'static>(
    self,
    factory: F,
  ) -> ParentComponent<Self>;
  fn with_layout(self, layout_constraints: Vec<ElementConstraints>) -> LayoutedComponent<Self>;
  fn build(self, context: &mut ElementContext) -> Element;
}

pub trait Component {
  fn construct(&self, _context: &mut ElementContext) {}
  fn render(&self, _layout: &ResolvedLayout, _renderer: &mut dyn Renderer, _theme: &mut Theme) {}
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

  fn with_children<F: FnOnce(&mut ElementContext) + 'static>(
    self,
    factory: F,
  ) -> ParentComponent<Self> {
    ParentComponent {
      component: self,
      children: Box::new(factory),
    }
  }

  fn with_layout(self, layout_constraints: Vec<ElementConstraints>) -> LayoutedComponent<Self> {
    LayoutedComponent {
      layout_constraints,
      component: self,
    }
  }
}

impl<T: ComponentElement> ComponentElement for ParentComponent<T> {
  fn build(self, context: &mut ElementContext) -> Element {
    let element = self.component.build(context);
    let mut child_context = context.get_context_for_child(&element);
    (self.children)(&mut child_context);
    element
  }

  fn with_children<F: FnOnce(&mut ElementContext) + 'static>(
    self,
    factory: F,
  ) -> ParentComponent<Self> {
    ParentComponent {
      component: self,
      children: Box::new(factory),
    }
  }

  fn with_layout(self, layout_constraints: Vec<ElementConstraints>) -> LayoutedComponent<Self> {
    LayoutedComponent {
      layout_constraints,
      component: self,
    }
  }
}

impl<T: Component + 'static> ComponentElement for T {
  fn build(self, context: &mut ElementContext) -> Element {
    let element = context.allocate_element(self);
    context.construct_element(&element);
    element
  }

  fn with_children<F: FnOnce(&mut ElementContext) + 'static>(
    self,
    factory: F,
  ) -> ParentComponent<Self> {
    ParentComponent {
      children: Box::new(factory),
      component: self,
    }
  }

  fn with_layout(self, layout_constraints: Vec<ElementConstraints>) -> LayoutedComponent<Self> {
    LayoutedComponent {
      layout_constraints,
      component: self,
    }
  }
}
