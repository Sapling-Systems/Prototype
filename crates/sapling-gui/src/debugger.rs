use raylib::color::Color;

use crate::{
  base::{MutableState, Pressable, StyledView, TextView},
  layout::ConstraintVariable,
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
    let (selected_node, selected_node_state) =
      MutableState::<Option<usize>>::new(context, || None, "selected_node");

    StyledView::new()
      .with_background_color(Color::BLACK.alpha(0.4))
      .with_border(1.0, Color::RED.alpha(0.8))
      .with_border_radius_even(16.0)
      .with_layout(vec![UserElementConstraints::floating_top_right(
        32.0, 32.0, 600.0, 500.0,
      )])
      .with_children(move |context| {
        if let Some(root_node) = context
          .prev_debug_nodes
          .as_ref()
          .and_then(|nodes| nodes.first())
        {
          let mut height = context.theme.spacing_default;
          render_node_recursive(root_node, 0, &mut height, context, selected_node_state);
        }
      })
      .build(context);

    if let Some(selected_node) = selected_node.and_then(|node_id| {
      context
        .prev_debug_nodes
        .as_ref()
        .map(|nodes| &nodes[node_id])
    }) {
      let component_name = selected_node.component_name.clone();
      let component_debug = selected_node.debug_info.clone();
      let constraints = selected_node.layout_constraints.clone();
      let layout = selected_node.layout.clone();

      LayoutView
        .with_layout(vec![UserElementConstraints::floating_top_right(
          38.0, 32.0, 300.0, 500.0,
        )])
        .with_children(move |context| {
          let mut offset = 50.0;
          TextView::new(
            FontVariant::Custom {
              color: Color::WHITE,
              size: 18.0,
            },
            component_name,
          )
          .with_horizontal_alignment(TextHorizontalAlignment::Center)
          .with_layout(vec![UserElementConstraints::cover_parent(0.0, 0.0)])
          .build(context);

          /*
          for line in component_debug.lines() {
            TextView::new(
              FontVariant::Custom {
                color: Color::WHITE,
                size: 12.0,
              },
              line.to_string(),
            )
            .with_layout(vec![UserElementConstraints::relative_to_parent(
              0.0, offset,
            )])
            .build(context);
            offset += 14.0;
          }
          */

          offset += 12.0;
          TextView::new(
            FontVariant::Custom {
              color: Color::WHITE,
              size: 16.0,
            },
            "Layout Constraints".to_string(),
          )
          .with_layout(vec![UserElementConstraints::relative_to_parent(
            0.0, offset,
          )])
          .build(context);
          offset += 18.0;

          for constraint in &constraints {
            ConstraintTextView::new(constraint.clone())
              .with_layout(vec![UserElementConstraints::relative_to_parent(
                0.0, offset,
              )])
              .build(context);
            offset += 16.0;
          }
          offset += 12.0;
          TextView::new(
            FontVariant::Custom {
              color: Color::WHITE,
              size: 16.0,
            },
            "Resolved Layout".to_string(),
          )
          .with_layout(vec![UserElementConstraints::relative_to_parent(
            0.0, offset,
          )])
          .build(context);
          offset += 18.0;

          TextView::new(
            FontVariant::Custom {
              color: Color::WHITE,
              size: 14.0,
            },
            format!(
              "X: {} Y: {} W: {} H: {}",
              layout.x, layout.y, layout.width, layout.height
            ),
          )
          .with_layout(vec![UserElementConstraints::relative_to_parent(
            0.0, offset,
          )])
          .build(context);
          offset += 18.0;
        })
        .build(context);

      HighlightOverlayView::new(selected_node.clone(), Color::RED.alpha(0.4)).build(context);

      let mut dependencies = Vec::new();
      for constraint in &selected_node.layout_constraints {
        for dependency in constraint.get_explicit_mentioned_variables() {
          let added = match dependency {
            ConstraintVariable::ParentX => selected_node.parent_id,
            ConstraintVariable::ParentY => selected_node.parent_id,
            ConstraintVariable::ParentWidth => selected_node.parent_id,
            ConstraintVariable::ParentHeight => selected_node.parent_id,
            ConstraintVariable::ElementX { id } => Some(id),
            ConstraintVariable::ElementY { id } => Some(id),
            ConstraintVariable::ElementWidth { id } => Some(id),
            ConstraintVariable::ElementHeight { id } => Some(id),
            _ => None,
          };
          dependencies.extend(added);
        }
      }
      dependencies.sort_unstable();
      dependencies.dedup();

      for dependency in &dependencies {
        if let Some(node) = context
          .prev_debug_nodes
          .as_ref()
          .and_then(|nodes| nodes.get(*dependency))
        {
          HighlightOverlayView::new(node.clone(), Color::BLUE.alpha(0.4)).build(context);
        }
      }
    }
  }
}

fn render_node_recursive(
  node: &DebugAllocatedElement,
  indentation: usize,
  height: &mut f32,
  context: &mut ElementContext,
  selected_node_state: MutableState<Option<usize>>,
) {
  let node = node.clone();
  if node.component_name.contains("DebuggerView") {
    return;
  }

  Pressable::new(move |context| {
    selected_node_state.set_direct(context, Some(node.id));
    println!("Element clicked in debugger:\n{}", node.debug_info);
  })
  .with_layout(vec![
    UserElementConstraints::relative_to_parent(
      indentation as f32 * context.theme.spacing_large + context.theme.spacing_default,
      *height,
    ),
    UserElementConstraints::fixed_size(10.0, 10.0),
  ])
  .with_children(move |context| {
    TextView::new(
      FontVariant::DefaultForegroundBold,
      format!("{} ({})", node.component_name, node.id),
    )
    .build(context);
  })
  .build(context);

  StyledView::new()
    .with_background_color(Color::RED.alpha(0.4))
    .with_border(1.0, Color::RED.alpha(0.8))
    .with_border_radius_even(16.0)
    .with_layout(vec![
      UserElementConstraints::relative_to_parent(
        indentation as f32 * context.theme.spacing_large + context.theme.spacing_default,
        *height,
      ),
      UserElementConstraints::fixed_size(10.0, 10.0),
    ])
    .build(context);

  for child in &node.children {
    *height += 16.0;
    render_node_recursive(
      &context.prev_debug_nodes.as_ref().unwrap()[*child],
      indentation + 1,
      height,
      context,
      selected_node_state,
    );
  }
}

#[derive(Debug)]
struct HighlightOverlayView {
  node: DebugAllocatedElement,
  color: Color,
}

impl HighlightOverlayView {
  fn new(node: DebugAllocatedElement, color: Color) -> Self {
    Self { node, color }
  }
}

impl Component for HighlightOverlayView {
  fn construct(&mut self, context: &mut ElementContext) {
    StyledView::new()
      .with_border(2.0, self.color)
      .with_layout(vec![
        UserElementConstraints::absolute_position(self.node.layout.x, self.node.layout.y),
        UserElementConstraints::fixed_size(self.node.layout.width, self.node.layout.height),
      ])
      .build(context);

    TextView::new(
      FontVariant::Custom {
        color: self.color,
        size: 12.0,
      },
      format!("${}", self.node.id),
    )
    .with_layout(vec![UserElementConstraints::absolute_position(
      self.node.layout.x + self.node.layout.width + 2.0,
      self.node.layout.y - 8.0,
    )])
    .build(context);
  }
}

#[derive(Debug)]
struct ConstraintTextView {
  constraint: CompiledConstraint,
}

impl ConstraintTextView {
  fn new(constraint: CompiledConstraint) -> Self {
    Self { constraint }
  }
}

impl Component for ConstraintTextView {
  fn construct(&mut self, context: &mut ElementContext) {
    TextView::new(
      FontVariant::Custom {
        color: Color::WHITE,
        size: 14.0,
      },
      self.constraint.get_formular(),
    )
    .build(context);
  }
}
