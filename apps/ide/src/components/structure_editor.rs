use sapling_data_model::{Query, SubjectSelector};
use sapling_gui::prelude::*;

use crate::{
  components::{
    loc::LinesOfCodeView,
    panel::PanelView,
    structure_editor::{
      collection_view::SubjectCollectionView,
      data::{Direction, SelectionPath, SubjectFactCollection},
      state::StructureEditorMode,
    },
  },
  input::Action,
};

pub mod collection_view;
pub mod data;
pub mod state;
pub mod subject_editor;

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
        let current_selection_path = selection_path.clone();

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
          .with_action_handler(Action::EditorSelectModeEdit, move |context| {
            mode_state.set_direct(
              context,
              StructureEditorMode::Edit {
                selection_path: current_selection_path,
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
          .with_layout(vec![UserElementConstraints::relative_to_parent(
            context.theme.spacing_large,
            context.theme.spacing_large,
          )])
          .build(context);

        SubjectCollectionView::new(collection, mode, self_path)
          .with_layout(vec![
            UserElementConstraints::anchor_to_right_of(loc_view, context.theme.spacing_large),
            UserElementConstraints::anchor_to_top_of(loc_view, context.theme.spacing_default),
          ])
          .build(context);
      })
      .with_layout(vec![UserElementConstraints::absolute_position(32.0, 32.0)])
      .build(context);
  }
}
