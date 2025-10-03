use sapling_data_model::Fact;

use crate::Database;

#[derive(Clone)]
pub struct NaiveFactIterator<'a> {
  database: &'a Database,
  current_index: usize,
}

impl<'a> Iterator for NaiveFactIterator<'a> {
  type Item = &'a Fact;

  fn next(&mut self) -> Option<Self::Item> {
    self.database.raw.get(self.current_index).inspect(|_| {
      self.current_index += 1;
    })
  }
}

impl Database {
  pub(crate) fn iter_naive_facts<'a>(&'a self) -> NaiveFactIterator<'a> {
    NaiveFactIterator {
      database: self,
      current_index: 0,
    }
  }
}
