use std::collections::HashMap;

use sapling_data_model::Subject;
use sapling_query_engine::{Database, System};

#[derive(Default)]
pub struct AppRegistry {
  global_subjects: HashMap<String, Subject>,
}

impl AppRegistry {
  pub fn create_global(&mut self, database: &mut Database, name: String) -> Subject {
    let subject = System::new_named_static(database, &name);
    self.global_subjects.insert(name, subject.clone());
    subject
  }

  pub fn get_global_by_name(&self, name: &str) -> Option<Subject> {
    let system_subject = System::get_named_subject(name);
    system_subject.or_else(|| self.global_subjects.get(name).cloned())
  }
}
