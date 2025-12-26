use sapling_data_model::Query;

use crate::{
  FoundFact, QueryEngine, SharedVariableAllocator, SharedVariableBank, machine::AbstractMachine,
};

pub trait QueryWatcher {
  fn on_match_changed(&self, changes: &[FoundFact]);
}

pub struct DatabaseWatcher {}

impl DatabaseWatcher {
  pub fn new() -> Self {
    DatabaseWatcher {}
  }

  pub fn query_and_watch<'a, T: QueryWatcher>(
    &'a mut self,
    query_engine: &QueryEngine,
    query: &Query,
    watcher: T,
    bank: SharedVariableBank,
    allocator: SharedVariableAllocator,
  ) -> AbstractMachine<'a> {
    todo!()
  }
}
