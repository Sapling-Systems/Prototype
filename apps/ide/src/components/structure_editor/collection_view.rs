use sapling_data_model::{Query, Subject, SubjectSelector};
use sapling_gui::prelude::*;

use crate::components::structure_editor::{
  StructureEditorMode,
  data::{SelectionPath, SelectionPathElement, SubjectFactCollection},
  subject_editor::SubjectEditor,
};

#[derive(Debug)]
pub struct SubjectCollectionView {
  collection: SubjectFactCollection,
  mode: StructureEditorMode,
  path: SelectionPath,
}

impl SubjectCollectionView {
  pub fn new(
    collection: SubjectFactCollection,
    mode: StructureEditorMode,
    path: SelectionPath,
  ) -> Self {
    Self {
      collection,
      mode,
      path,
    }
  }
}

impl Component for SubjectCollectionView {
  fn construct(&mut self, context: &mut ElementContext) {
    let line_height = 1.3;
    match &self.collection.subject.subject {
      Subject::String { value } => {
        TextView::new(FontVariant::EditorString, value.clone())
          .with_line_height(line_height)
          .with_layout(vec![UserElementConstraints::relative_to_parent(0.0, 0.0)])
          .build(context);
        return;
      }
      Subject::Integer { value } => {
        TextView::new(FontVariant::EditorNumber, format!("{}", value.clone()))
          .with_line_height(line_height)
          .with_layout(vec![UserElementConstraints::relative_to_parent(0.0, 0.0)])
          .build(context);
        return;
      }
      Subject::Static { .. } => {}
      _ => {
        return;
      }
    }

    let subject_name = context.app.get_name(&self.collection.subject.subject);

    let mut selected_element = None;

    let subject_text_view = TextView::new(FontVariant::EditorSubject, subject_name)
      .with_line_height(line_height)
      .with_layout(vec![UserElementConstraints::relative_to_parent(0.0, 0.0)])
      .build(context);
    let subject_path = self.path.with(SelectionPathElement::Subject);
    if self.mode.is_selected(&subject_path) {
      selected_element = Some(subject_text_view);
    }

    let mut top_most_element = None;
    let mut first_fact_element = None;

    for fact in &self.collection.facts {
      let top_constraint = if let Some(top_most_element) = top_most_element {
        UserElementConstraints::anchor_to_bottom_of(top_most_element, 0.0)
      } else {
        UserElementConstraints::anchor_to_top_of(subject_text_view, 0.0)
      };

      if let Some(property) = fact.property.as_ref() {
        let fact_path = self.path.with(SelectionPathElement::Fact {
          property: property.subject.clone(),
        });

        let property_name = context.app.get_name(&property.subject);
        let property_text_view = TextView::new(FontVariant::EditorProperty, property_name)
          .with_line_height(line_height)
          .with_layout(vec![
            UserElementConstraints::anchor_to_right_of(
              subject_text_view,
              context.theme.spacing_large,
            ),
            top_constraint.clone(),
          ])
          .build(context);

        if first_fact_element.is_none() {
          first_fact_element = Some(property_text_view);
        }
        top_most_element = Some(property_text_view);

        if self
          .mode
          .is_selected(&fact_path.with(SelectionPathElement::Property))
        {
          selected_element = Some(property_text_view);
        }

        if let Some(operator) = &fact.operator {
          let operator_name = context.app.get_name(operator);
          let operator_view = TextView::new(FontVariant::EditorOperator, operator_name)
            .with_line_height(line_height)
            .with_layout(vec![
              UserElementConstraints::anchor_to_right_of(
                property_text_view,
                context.theme.spacing_default,
              ),
              top_constraint.clone(),
            ])
            .build(context);
          if self
            .mode
            .is_selected(&fact_path.with(SelectionPathElement::Operator))
          {
            selected_element = Some(operator_view);
          }

          if let Some(value) = &fact.value {
            let value_path = fact_path.with(SelectionPathElement::Value);
            let value_view = if self.mode.is_editing(&value_path) {
              SubjectEditor::new(value.subject.subject.clone())
                .with_layout(vec![
                  UserElementConstraints::anchor_to_right_of(
                    operator_view,
                    context.theme.spacing_default,
                  ),
                  top_constraint.clone(),
                ])
                .build(context)
            } else {
              SubjectCollectionView::new(
                (**value).clone(),
                self.mode.clone(),
                fact_path.with(SelectionPathElement::Value),
              )
              .with_layout(vec![
                UserElementConstraints::anchor_to_right_of(
                  operator_view,
                  context.theme.spacing_default,
                ),
                top_constraint.clone(),
              ])
              .build(context)
            };

            if self.mode.is_selected(&value_path) {
              selected_element = Some(value_view);
            }

            top_most_element = Some(value_view);
          }
        }
      }
    }

    if let (Some(first_fact_element), Some(top_most_element)) =
      (first_fact_element, top_most_element)
    {
      StyledView::new()
        .with_background_color(context.theme.color_divider)
        .with_layout(vec![
          UserElementConstraints::anchor_to_right_of(
            subject_text_view,
            context.theme.spacing_default,
          ),
          UserElementConstraints::anchor_to_top_of(first_fact_element, 0.0),
          UserElementConstraints::fixed_width(1.0),
          UserElementConstraints::scale_to_bottom_of(top_most_element, 0.0),
        ])
        .build(context);
    }

    if let Some(selected_element) = selected_element {
      StyledView::new()
        .with_border(2.0, context.theme.color_primary)
        .with_border_radius_even(context.theme.radius_default)
        .with_layout(vec![UserElementConstraints::cover_element(
          selected_element,
          -context.theme.spacing_small,
          -context.theme.spacing_small,
        )])
        .build(context);
    }
  }
}
