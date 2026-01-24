use std::{path, thread::current};

use sapling_data_model::{Query, Subject, SubjectSelector};
use sapling_gui::prelude::*;

use crate::{
  components::{
    loc::LinesOfCodeView,
    panel::PanelView,
    structure_editor::data::{SubjectFactCollection, SubjectFactCollectionFact},
  },
  input::Action,
};

pub mod data;

#[cfg(test)]
mod tests;

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
          .with_layout(vec![ElementConstraints::relative_position()])
          .build(context);
        return;
      }
      Subject::Integer { value } => {
        TextView::new(FontVariant::EditorNumber, format!("{}", value.clone()))
          .with_line_height(line_height)
          .with_layout(vec![ElementConstraints::relative_position()])
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
      .with_layout(vec![ElementConstraints::relative_position()])
      .build(context);
    let subject_path = self.path.with(SelectionPathElement::Subject);
    if self.mode.is_selected(&subject_path) {
      selected_element = Some(subject_text_view);
    }

    let mut top_most_element = None;
    let mut first_fact_element = None;

    for fact in &self.collection.facts {
      let top_constraint = if let Some(top_most_element) = top_most_element {
        ElementConstraints::anchor_to_bottom_of(top_most_element, 0.0)
      } else {
        ElementConstraints::anchor_to_top_of(subject_text_view, 0.0)
      };

      if let Some(property) = fact.property.as_ref() {
        let fact_path = self.path.with(SelectionPathElement::Fact {
          property: property.subject.clone(),
        });

        let property_name = context.app.get_name(&property.subject);
        let property_text_view = TextView::new(FontVariant::EditorProperty, property_name)
          .with_line_height(line_height)
          .with_layout(vec![
            ElementConstraints::anchor_to_right_of(subject_text_view, context.theme.spacing_large),
            top_constraint.clone(),
          ])
          .build(context);

        if self
          .mode
          .is_selected(&fact_path.with(SelectionPathElement::Property))
        {
          selected_element = Some(property_text_view);
        }

        if first_fact_element.is_none() {
          first_fact_element = Some(property_text_view);
        }
        top_most_element = Some(property_text_view);

        if let Some(operator) = &fact.operator {
          let operator_name = context.app.get_name(operator);
          let operator_view = TextView::new(FontVariant::EditorOperator, operator_name)
            .with_line_height(line_height)
            .with_layout(vec![
              ElementConstraints::anchor_to_right_of(
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
            let value_view = SubjectCollectionView::new(
              (**value).clone(),
              self.mode.clone(),
              fact_path.with(SelectionPathElement::Value),
            )
            .with_layout(vec![
              ElementConstraints::anchor_to_right_of(operator_view, context.theme.spacing_default),
              top_constraint.clone(),
            ])
            .build(context);

            if self
              .mode
              .is_selected(&fact_path.with(SelectionPathElement::Value))
            {
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
          constraint!(
            self_x
              == subject_text_view.x()
                + subject_text_view.width()
                + (context.theme.spacing_large / 2.0)
          ),
          constraint!(self_width == 1.0),
          constraint!(self_y == first_fact_element.y()),
          constraint!(
            self_height
              == top_most_element.y() + top_most_element.height() - first_fact_element.y()
          ),
        ])
        .build(context);
    }

    if let Some(selected_element) = selected_element {
      StyledView::new()
        .with_border(2.0, context.theme.color_primary)
        .with_border_radius_even(context.theme.radius_default)
        .with_layout(vec![
          constraint!(self_x == selected_element.x() - context.theme.spacing_small),
          constraint!(self_y == selected_element.y() - context.theme.spacing_small),
          constraint!(self_width == selected_element.width() + context.theme.spacing_small * 2.0),
          constraint!(self_height == selected_element.height() + context.theme.spacing_small),
        ])
        .build(context);
    }
  }
}

#[derive(Debug)]
pub struct StructureEditor {
  query: Query,
}

impl StructureEditor {
  pub fn new(query: Query) -> Self {
    Self { query }
  }
}

impl Component for StructureEditor {
  fn construct(&mut self, context: &mut ElementContext) {
    let collection = SubjectFactCollection::new(
      SubjectSelector {
        evaluated: self.query.evaluated,
        property: self.query.property.clone(),
        subject: self.query.subject.clone(),
      },
      context.app,
    );

    let self_path = SelectionPath::empty();

    let (mode, mode_state) = MutableState::new(
      context,
      || StructureEditorMode::Select {
        selection_path: SelectionPath::default(),
      },
      "editor_mode",
    );

    println!("Selection Path: {:?}", mode);

    #[allow(clippy::single_match)]
    match &mode {
      StructureEditorMode::Select { selection_path } => {
        let left_selection_path = selection_path.move_to(Direction::Left, &collection);
        let right_selection_path = selection_path.move_to(Direction::Right, &collection);
        let up_selection_path = selection_path.move_to(Direction::Up, &collection);
        let down_selection_path = selection_path.move_to(Direction::Down, &collection);

        FocusableInteractiveView::new()
          .with_action_handler(Action::EditorSelectModeLeft, move |context| {
            mode_state.set_direct(
              context,
              StructureEditorMode::Select {
                selection_path: left_selection_path,
              },
            );
          })
          .with_action_handler(Action::EditorSelectModeRight, move |context| {
            mode_state.set_direct(
              context,
              StructureEditorMode::Select {
                selection_path: right_selection_path,
              },
            );
          })
          .with_action_handler(Action::EditorSelectModeUp, move |context| {
            mode_state.set_direct(
              context,
              StructureEditorMode::Select {
                selection_path: up_selection_path,
              },
            );
          })
          .with_action_handler(Action::EditorSelectModeDown, move |context| {
            mode_state.set_direct(
              context,
              StructureEditorMode::Select {
                selection_path: down_selection_path,
              },
            );
          })
          .build(context);
      }
      _ => {}
    }

    PanelView::new()
      .with_content(|context| {
        let loc_view = LinesOfCodeView::new(vec![0; 10], 2)
          .with_layout(vec![
            ElementConstraints::relative_top(context.theme.spacing_large),
            ElementConstraints::relative_left(context.theme.spacing_large),
          ])
          .build(context);

        SubjectCollectionView::new(collection, mode, self_path)
          .with_layout(vec![
            ElementConstraints::anchor_to_right_of(loc_view, context.theme.spacing_large),
            ElementConstraints::anchor_to_top_of(loc_view, context.theme.spacing_default),
          ])
          .build(context);
      })
      .with_layout(vec![ElementConstraints::absolute_position(32.0, 32.0)])
      .build(context);
  }
}

#[derive(Clone, Debug)]
pub(crate) enum StructureEditorMode {
  None,
  Select { selection_path: SelectionPath },
}

impl StructureEditorMode {
  fn is_selected(&self, comparison_path: &SelectionPath) -> bool {
    match self {
      StructureEditorMode::None => false,
      StructureEditorMode::Select { selection_path } => selection_path.matches(comparison_path),
    }
  }
}

#[derive(Clone, Debug)]
enum SelectionPathElement {
  Subject,
  Operator,
  Value,
  Property,
  Fact { property: Subject },
}

#[derive(Clone, Debug)]
struct SelectionPath {
  path: Vec<SelectionPathElement>,
}

impl Default for SelectionPath {
  fn default() -> Self {
    Self {
      path: vec![SelectionPathElement::Subject],
    }
  }
}

enum Selection<'a> {
  SubjectSelector(Option<&'a SubjectSelector>),
  Subject(Option<&'a Subject>),
  Collection(Option<&'a SubjectFactCollection>),
  Fact(Option<&'a SubjectFactCollectionFact>),
}

impl SelectionPath {
  fn empty() -> Self {
    Self { path: vec![] }
  }

  fn with(&self, element: SelectionPathElement) -> Self {
    let mut path = self.path.clone();
    path.push(element);
    Self { path }
  }

  fn popped(&self) -> Self {
    let mut path = self.path.clone();
    path.pop();
    Self { path }
  }

  fn matches(&self, other: &Self) -> bool {
    if self.path.len() != other.path.len() {
      return false;
    }
    self
      .path
      .iter()
      .zip(other.path.iter())
      .all(|(a, b)| match (a, b) {
        (SelectionPathElement::Subject, SelectionPathElement::Subject) => true,
        (SelectionPathElement::Operator, SelectionPathElement::Operator) => true,
        (SelectionPathElement::Value, SelectionPathElement::Value) => true,
        (SelectionPathElement::Property, SelectionPathElement::Property) => true,
        (
          SelectionPathElement::Fact {
            property: property1,
          },
          SelectionPathElement::Fact {
            property: property2,
          },
        ) => property1.is_same(property2),
        _ => false,
      })
  }

  fn traverse<'a>(&self, collection: &'a SubjectFactCollection) -> Option<Selection<'a>> {
    enum CurrentItem<'b> {
      Collection(&'b SubjectFactCollection),
      CollectionFact(&'b SubjectFactCollectionFact),
    }

    if self.path.is_empty() {
      return Some(Selection::Collection(Some(collection)));
    }

    let mut current = CurrentItem::Collection(collection);
    for (index, path_item) in self.path.iter().enumerate() {
      let is_last_item = index == self.path.len() - 1;

      match (path_item, &current) {
        (SelectionPathElement::Subject, CurrentItem::Collection(collection)) => {
          return Some(Selection::SubjectSelector(Some(&collection.subject)));
        }
        (SelectionPathElement::Operator, CurrentItem::CollectionFact(fact)) => {
          return Some(Selection::Subject(fact.operator.as_ref()));
        }
        (SelectionPathElement::Property, CurrentItem::CollectionFact(fact)) => {
          return Some(Selection::SubjectSelector(fact.property.as_ref()));
        }
        (SelectionPathElement::Value, CurrentItem::CollectionFact(fact)) => {
          if !is_last_item {
            if let Some(value) = fact.value.as_ref() {
              current = CurrentItem::Collection(&**value);
            }
            continue;
          }
          return Some(Selection::Collection(fact.value.as_ref().map(|val| &**val)));
        }
        (SelectionPathElement::Fact { property }, CurrentItem::Collection(collection)) => {
          let new_fact = collection.facts.iter().find(|fact| {
            if let Some(property_selctor) = &fact.property {
              property_selctor.subject.is_same(property)
            } else {
              false
            }
          });
          if !is_last_item {
            if let Some(new_fact) = new_fact {
              current = CurrentItem::CollectionFact(new_fact);
            }
            continue;
          }

          return Some(Selection::Fact(new_fact));
        }
        _ => return None,
      }
    }

    None
  }

  fn advance_fact(&self, collection: &SubjectFactCollection, negative: bool) -> Self {
    let Some(Selection::Fact(Some(property_fact))) = self.traverse(collection) else {
      return self.clone();
    };
    let Some(property) = &property_fact.property else {
      return self.clone();
    };

    let Some(Selection::Collection(Some(parent_collection))) = self.popped().traverse(collection)
    else {
      return self.clone();
    };

    let Some(current_fact_index) = parent_collection.facts.iter().position(|fact| {
      fact
        .property
        .as_ref()
        .map(|inner_property| inner_property.subject.is_same(&property.subject))
        .unwrap_or(false)
    }) else {
      return self.clone();
    };

    if negative && current_fact_index == 0 {
      return self.clone();
    } else if !negative && current_fact_index == parent_collection.facts.len() - 1 {
      return self.clone();
    }

    let next_fact = if negative {
      parent_collection.facts.get(current_fact_index - 1)
    } else {
      parent_collection.facts.get(current_fact_index + 1)
    };

    if let Some(next_fact) = next_fact {
      if let Some(next_property) = &next_fact
        .property
        .as_ref()
        .map(|property| &property.subject)
      {
        return self.popped().with(SelectionPathElement::Fact {
          property: (*next_property).clone(),
        });
      }
    }

    self.clone()
  }

  fn move_to(&self, direction: Direction, collection: &SubjectFactCollection) -> Self {
    let mut path_clone = self.path.clone();
    let last_item = self.path.last().unwrap();

    match (last_item, &direction) {
      (SelectionPathElement::Subject, Direction::Left) => {
        if self.path.len() > 1 {
          path_clone.pop();
          return Self { path: path_clone };
        }
      }
      (SelectionPathElement::Subject, Direction::Right) => {
        if let Some(Selection::Collection(Some(collection))) = self.popped().traverse(collection) {
          if let Some(first_property) = collection
            .facts
            .first()
            .and_then(|fact| fact.property.as_ref())
          {
            path_clone.pop();
            path_clone.push(SelectionPathElement::Fact {
              property: first_property.subject.clone(),
            });
            path_clone.push(SelectionPathElement::Property);
            return Self { path: path_clone };
          }
        }
      }
      (SelectionPathElement::Property, Direction::Left) => {
        path_clone.pop();
        path_clone.pop();
        path_clone.push(SelectionPathElement::Subject);
        return Self { path: path_clone };
      }
      (SelectionPathElement::Property, Direction::Right) => {
        path_clone.pop();
        path_clone.push(SelectionPathElement::Operator);
        return Self { path: path_clone };
      }
      (SelectionPathElement::Operator, Direction::Left) => {
        path_clone.pop();
        path_clone.push(SelectionPathElement::Property);
        return Self { path: path_clone };
      }
      (SelectionPathElement::Operator, Direction::Right) => {
        path_clone.pop();
        path_clone.push(SelectionPathElement::Value);
        return Self { path: path_clone };
      }
      (SelectionPathElement::Value, Direction::Left) => {
        path_clone.pop();
        path_clone.push(SelectionPathElement::Operator);
        return Self { path: path_clone };
      }
      (SelectionPathElement::Value, Direction::Right) => {
        if let Some(Selection::Collection(Some(collection))) = self.traverse(collection) {
          if matches!(collection.subject.subject, Subject::Static { .. })
            && collection.facts.len() >= 1
          {
            return self.with(SelectionPathElement::Subject);
          }
        }
      }
      (SelectionPathElement::Property, Direction::Down) => {
        return self
          .popped()
          .advance_fact(collection, false)
          .with(SelectionPathElement::Property);
      }
      (SelectionPathElement::Property, Direction::Up) => {
        return self
          .popped()
          .advance_fact(collection, true)
          .with(SelectionPathElement::Property);
      }
      (SelectionPathElement::Operator, Direction::Down) => {
        return self
          .popped()
          .advance_fact(collection, false)
          .with(SelectionPathElement::Operator);
      }
      (SelectionPathElement::Operator, Direction::Up) => {
        return self
          .popped()
          .advance_fact(collection, true)
          .with(SelectionPathElement::Operator);
      }
      (SelectionPathElement::Value, Direction::Down) => {
        return self
          .popped()
          .advance_fact(collection, false)
          .with(SelectionPathElement::Value);
      }
      (SelectionPathElement::Value, Direction::Up) => {
        return self
          .popped()
          .advance_fact(collection, true)
          .with(SelectionPathElement::Value);
      }
      _ => {
        return self.clone();
      }
    }

    self.clone()
  }
}

enum Direction {
  Left,
  Up,
  Right,
  Down,
}
