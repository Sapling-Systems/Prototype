use sapling_gui::prelude::*;

use crate::components::structure_editor::data::SubjectFactCollection;

pub mod data;

#[derive(Debug)]
pub struct StructureEditor {
  collection: SubjectFactCollection,
}

impl StructureEditor {
  pub fn new(collection: SubjectFactCollection) -> Self {
    Self { collection }
  }
}

impl Component for StructureEditor {
  fn construct(&mut self, context: &mut ElementContext) {
    TextView::new(FontVariant::Primary, "Structure Editor".into())
      .with_layout(vec![ElementConstraints::relative_position()])
      .build(context);
  }
}

/*
 */
