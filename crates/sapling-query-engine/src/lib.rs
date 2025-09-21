use std::sync::Arc;

use sapling_data_model::{Fact, Subject};

use crate::database::Database;

mod database;

pub struct QueryEngine {
  database: Arc<Database>,
}

impl QueryEngine {
  pub fn new(database: Arc<Database>) -> Self {
    Self { database }
  }

  pub fn query<'a>(&'a self, subject: &Subject) -> impl Iterator<Item = &'a Fact> {
    let target_facts = self.database.get_facts_for_subject(subject);
    target_facts.into_iter()
  }
}
