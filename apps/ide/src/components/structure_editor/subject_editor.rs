use sapling_data_model::Subject;
use sapling_gui::prelude::*;

#[derive(Debug, Clone)]
pub struct SubjectEditor {
  initial_value: Subject,
}

impl SubjectEditor {
  pub fn new(initial_value: Subject) -> Self {
    Self { initial_value }
  }
}

impl Component for SubjectEditor {
  fn construct(&mut self, context: &mut ElementContext) {
    let (subject, subject_state) =
      MutableState::<Subject>::new(context, || self.initial_value.clone(), "subject");

    let container = StyledView::new()
      .with_border(1.0, context.theme.color_background_secondary)
      .with_border_radius_even(context.theme.radius_default)
      .with_children(move |context| match subject {
        Subject::String { value } => {
          let type_view = StyledView::new()
            .with_background_color(context.theme.color_tertiary)
            .with_border_radius_even(context.theme.radius_large)
            .with_children(move |context| {
              TextView::new(FontVariant::EditorEditType, "String".into()).build(context);
            })
            .build(context);

          TextView::new(FontVariant::EditorString, value.clone())
            .with_layout(vec![UserElementConstraints::anchor_to_right_of(
              type_view,
              context.theme.spacing_default,
            )])
            .build(context);
        }
        _ => {}
      })
      .build(context);
  }
}
