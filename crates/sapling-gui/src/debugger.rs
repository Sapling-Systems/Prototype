use raylib::color::Color;

use crate::{
  base::{Pressable, StyledView, TextView},
  layout::ElementConstraints,
  orchestrator::DebugAllocatedElement,
  prelude::*,
  theme::FontVariant,
};

#[derive(Debug)]
pub struct DebuggerView;

impl DebuggerView {
  pub fn new() -> Self {
    Self {}
  }
}

impl Component for DebuggerView {
  fn construct(&mut self, context: &mut crate::prelude::ElementContext) {
    StyledView::new()
      .with_background_color(Color::BLACK.alpha(0.4))
      .with_border(1.0, Color::RED.alpha(0.8))
      .with_border_radius_even(16.0)
      .with_layout(vec![
        ElementConstraints::absolute_top(32.0),
        ElementConstraints::absolute_right(32.0),
        ElementConstraints::fixed_size(600.0, 500.0),
      ])
      .with_children(|context| {
        if let Some(root_node) = context.prev_debug_node {
          let mut height = context.theme.spacing_default;
          render_node_recursive(root_node, 0, &mut height, context);
        }
      })
      .build(context);
  }
}

fn render_node_recursive(
  node: &DebugAllocatedElement,
  indentation: usize,
  height: &mut f32,
  context: &mut ElementContext,
) {
  let node = node.clone();

  Pressable::new(move || {
    println!("Element clicked in debugger:\n{}", node.debug_info);
  })
  .with_layout(vec![
    ElementConstraints::relative_left(
      indentation as f32 * context.theme.spacing_large + context.theme.spacing_default,
    ),
    ElementConstraints::relative_top(*height),
    ElementConstraints::fixed_size(10.0, 10.0),
  ])
  .with_children(move |context| {
    TextView::new(
      FontVariant::DefaultForegroundBold,
      node.component_name.clone(),
    )
    .build(context);
  })
  .build(context);

  StyledView::new()
    .with_background_color(Color::RED.alpha(0.4))
    .with_border(1.0, Color::RED.alpha(0.8))
    .with_border_radius_even(16.0)
    .with_layout(vec![
      ElementConstraints::relative_left(
        indentation as f32 * context.theme.spacing_large + context.theme.spacing_default,
      ),
      ElementConstraints::relative_top(*height),
      ElementConstraints::fixed_size(10.0, 10.0),
    ])
    .build(context);

  for child in &node.children {
    *height += 16.0;
    render_node_recursive(child, indentation + 1, height, context);
  }
}
