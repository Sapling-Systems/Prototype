use crate::components::structure_editor::data::SelectionPath;

#[derive(Clone, Debug)]
pub(crate) enum StructureEditorMode {
  None,
  Select { selection_path: SelectionPath },
  Edit { selection_path: SelectionPath },
}

impl StructureEditorMode {
  pub fn is_selected(&self, comparison_path: &SelectionPath) -> bool {
    match self {
      StructureEditorMode::Select { selection_path } => selection_path.matches(comparison_path),
      _ => false,
    }
  }

  pub fn is_editing(&self, comparison_path: &SelectionPath) -> bool {
    match self {
      StructureEditorMode::Edit { selection_path } => selection_path.matches(comparison_path),
      _ => false,
    }
  }
}
