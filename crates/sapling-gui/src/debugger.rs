use raylib::color::Color;

use crate::{
  base::{MutableState, Pressable, StyledView, TextView},
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
    let (selected_node, selected_node_state) =
      MutableState::<Option<usize>>::new(context, || None, "selected_node");

    StyledView::new()
      .with_background_color(Color::BLACK.alpha(0.4))
      .with_border(1.0, Color::RED.alpha(0.8))
      .with_border_radius_even(16.0)
      .with_layout(vec![
        ElementConstraints::absolute_top(32.0),
        ElementConstraints::absolute_right(32.0),
        ElementConstraints::fixed_size(600.0, 500.0),
      ])
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
        .with_layout(vec![
          ElementConstraints::absolute_top(38.0),
          ElementConstraints::absolute_right(32.0),
          ElementConstraints::fixed_size(300.0, 500.0),
        ])
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
          .with_layout(vec![ElementConstraints::cover_parent()])
          .build(context);

          for line in component_debug.lines() {
            TextView::new(
              FontVariant::Custom {
                color: Color::WHITE,
                size: 12.0,
              },
              line.to_string(),
            )
            .with_layout(vec![
              ElementConstraints::relative_top(offset),
              ElementConstraints::relative_left(0.0),
            ])
            .build(context);
            offset += 14.0;
          }

          offset += 12.0;
          TextView::new(
            FontVariant::Custom {
              color: Color::WHITE,
              size: 16.0,
            },
            "Layout Constraints".to_string(),
          )
          .with_layout(vec![
            ElementConstraints::relative_top(offset),
            ElementConstraints::relative_left(0.0),
          ])
          .build(context);
          offset += 18.0;

          for constraint in &constraints.constraints {
            ConstraintTextView::new(constraint.clone())
              .with_layout(vec![
                ElementConstraints::relative_top(offset),
                ElementConstraints::relative_left(0.0),
              ])
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
          .with_layout(vec![
            ElementConstraints::relative_top(offset),
            ElementConstraints::relative_left(0.0),
          ])
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
          .with_layout(vec![
            ElementConstraints::relative_top(offset),
            ElementConstraints::relative_left(0.0),
          ])
          .build(context);
          offset += 18.0;
        })
        .build(context);

      HighlightOverlayView::new(selected_node.clone(), Color::RED.alpha(0.4)).build(context);

      let mut dependencies = Vec::new();
      for constraint in &selected_node.layout_constraints.constraints {
        dependencies.extend(constraint.expression.terms.iter().filter_map(
          |term| match term.variable {
            ElementConstraintVariable::ParentX => selected_node.parent_id,
            ElementConstraintVariable::ParentY => selected_node.parent_id,
            ElementConstraintVariable::ParentWidth => selected_node.parent_id,
            ElementConstraintVariable::ParentHeight => selected_node.parent_id,
            ElementConstraintVariable::ElementX(element) => Some(element.id),
            ElementConstraintVariable::ElementY(element) => Some(element.id),
            ElementConstraintVariable::ElementWidth(element) => Some(element.id),
            ElementConstraintVariable::ElementHeight(element) => Some(element.id),
            _ => None,
          },
        ));
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
    ElementConstraints::relative_left(
      indentation as f32 * context.theme.spacing_large + context.theme.spacing_default,
    ),
    ElementConstraints::relative_top(*height),
    ElementConstraints::fixed_size(10.0, 10.0),
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
      ElementConstraints::relative_left(
        indentation as f32 * context.theme.spacing_large + context.theme.spacing_default,
      ),
      ElementConstraints::relative_top(*height),
      ElementConstraints::fixed_size(10.0, 10.0),
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
        ElementConstraints::absolute_position(self.node.layout.x, self.node.layout.y),
        ElementConstraints::fixed_size(self.node.layout.width, self.node.layout.height),
      ])
      .build(context);

    TextView::new(
      FontVariant::Custom {
        color: self.color,
        size: 12.0,
      },
      format!("${}", self.node.id),
    )
    .with_layout(vec![
      ElementConstraints::absolute_position(
        self.node.layout.x + self.node.layout.width + 2.0,
        self.node.layout.y - 8.0,
      ),
      ElementConstraints::fixed_size(self.node.layout.width, self.node.layout.height),
    ])
    .build(context);
  }
}

#[derive(Debug)]
struct ConstraintTextView {
  constraint: ElementConstraint,
}

impl ConstraintTextView {
  fn new(constraint: ElementConstraint) -> Self {
    Self { constraint }
  }
}

impl Component for ConstraintTextView {
  fn construct(&mut self, context: &mut ElementContext) {
    let operator = match self.constraint.operator {
      ElementConstraintOperator::Equal => "=",
      ElementConstraintOperator::GreaterOrEqual => ">=",
      ElementConstraintOperator::LessOrEqual => "<=",
    };

    let mut expression = String::new();
    for (index, term) in self.constraint.expression.terms.iter().enumerate() {
      let var_name = match term.variable {
        ElementConstraintVariable::ElementX(element) => format!("${}::x", element.id),
        ElementConstraintVariable::ElementY(element) => format!("${}::y", element.id),
        ElementConstraintVariable::ElementWidth(element) => format!("${}::width", element.id),
        ElementConstraintVariable::ElementHeight(element) => format!("${}::height", element.id),
        ElementConstraintVariable::ParentX => "parent::x".to_string(),
        ElementConstraintVariable::ParentY => "parent::y".to_string(),
        ElementConstraintVariable::ParentWidth => "parent::width".to_string(),
        ElementConstraintVariable::ParentHeight => "parent::height".to_string(),
        ElementConstraintVariable::SelfX => "x".to_string(),
        ElementConstraintVariable::SelfY => "y".to_string(),
        ElementConstraintVariable::SelfWidth => "width".to_string(),
        ElementConstraintVariable::SelfHeight => "height".to_string(),
        ElementConstraintVariable::ScreenWidth => "screen::width".to_string(),
        ElementConstraintVariable::ScreenHeight => "screen::height".to_string(),
      };
      if term.coefficient == 1.0 {
        expression.push_str(&var_name);
      } else if term.coefficient == -1.0 {
        expression.push_str(&format!("-{}", var_name));
      } else {
        expression.push_str(&format!("{} * {}", term.coefficient, var_name));
      }

      if index != self.constraint.expression.terms.len() - 1 {
        expression.push_str(" + ");
      }
    }

    if self.constraint.expression.constant != 0.0 {
      expression.push_str(&format!(" + {}", self.constraint.expression.constant));
    }

    let text = format!(
      "{} {} 0 [{}]",
      expression, operator, self.constraint.strength
    );

    TextView::new(
      FontVariant::Custom {
        color: Color::WHITE,
        size: 14.0,
      },
      text,
    )
    .build(context);
  }
}
