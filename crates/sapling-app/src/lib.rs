use sapling_data_model::{Fact, Query, Subject};
use sapling_query_engine::{
  Database, DatabaseWatcher, ExplainQuery, FoundFact, QueryEngine, SharedVariableAllocator,
  SharedVariableBank,
};

use crate::plugin::{AppPlugin, AppPluginInstallContext};

mod plugin;
mod serialization;

pub struct App {
  database: Database,
  watcher: DatabaseWatcher,
  query_engine: QueryEngine,
  variable_allocator: SharedVariableAllocator,
  variable_bank: SharedVariableBank,
}

impl App {
  pub fn new(bank_size: usize) -> Self {
    let database = Database::new();
    let watcher = DatabaseWatcher::new();
    let query_engine = QueryEngine::new();
    let variable_allocator = SharedVariableAllocator::new();
    let variable_bank = SharedVariableBank::new(bank_size);

    Self {
      database,
      watcher,
      query_engine,
      variable_allocator,
      variable_bank,
    }
  }

  pub fn get_raw_database(&self) -> &Database {
    &self.database
  }

  pub fn get_raw_database_mut(&mut self) -> &mut Database {
    &mut self.database
  }

  pub fn add_plugin<TPlugin: AppPlugin>(&mut self, mut plugin: TPlugin) {
    plugin.install_plugin(&mut AppPluginInstallContext::new(
      &mut self.database,
      &mut self.watcher,
      &mut self.query_engine,
      self.variable_bank.clone(),
      self.variable_allocator.clone(),
    ));
  }

  pub fn query_once<'a>(&'a self, query: &Query) -> impl Iterator<Item = FoundFact<'a>> {
    self.variable_allocator.reset();
    self.variable_bank.reset();
    self.query_engine.query(
      &self.database,
      query,
      self.variable_bank.clone(),
      self.variable_allocator.clone(),
    )
  }

  pub fn explain_once(&self, subject: &Subject) -> sapling_query_engine::ExplainResult {
    self.variable_allocator.reset();
    self.variable_bank.reset();
    self.query_engine.explain(
      &self.database,
      subject,
      self.variable_bank.clone(),
      self.variable_allocator.clone(),
    )
  }

  pub fn add_fact(&mut self, fact: Fact) -> usize {
    let index = self.database.add_fact(fact);
    self.watcher.handle_new_fact(
      &mut self.database,
      &self.query_engine,
      self.variable_bank.clone(),
      self.variable_allocator.clone(),
      index,
    );
    index
  }
}
