use sapling_query_engine::{
  Database, DatabaseWatcher, QueryEngine, SharedVariableAllocator, SharedVariableBank, System,
};
use sapling_serialization::{DeserializerContext, SerializerContext};

use crate::registry::AppRegistry;

pub struct AppPluginSerializerContext<'a> {
  database: &'a mut Database,
  query_engine: &'a QueryEngine,
  variable_bank: SharedVariableBank,
  variable_allocator: SharedVariableAllocator,
  registry: Option<&'a mut AppRegistry>,
}

impl<'a> AppPluginSerializerContext<'a> {
  pub fn new(
    database: &'a mut Database,
    query_engine: &'a QueryEngine,
    variable_bank: SharedVariableBank,
    variable_allocator: SharedVariableAllocator,
    registry: Option<&'a mut AppRegistry>,
  ) -> Self {
    Self {
      database,
      query_engine,
      variable_bank,
      variable_allocator,
      registry,
    }
  }
}

impl<'a> SerializerContext for AppPluginSerializerContext<'a> {
  fn add_fact(&mut self, fact: sapling_data_model::Fact) {
    self.database.add_fact(fact);
  }
  fn new_static_subject(&mut self, name: &str) -> sapling_data_model::Subject {
    self
      .registry
      .as_mut()
      .expect("Registry shoud be called earlier")
      .create_global(self.database, name.into())
  }
}

impl<'a> DeserializerContext for AppPluginSerializerContext<'a> {
  fn new_static_subject(&mut self, name: &str) -> sapling_data_model::Subject {
    self
      .registry
      .as_mut()
      .expect("Registry shoud be called earlier")
      .create_global(self.database, name.into())
  }
  fn get_subject_name(&mut self, subject: &sapling_data_model::Subject) -> String {
    System::get_subject_name(self.database, subject).unwrap()
  }
  fn query<'db, 'q>(
    &'db mut self,
    query: &'q sapling_data_model::Query,
  ) -> Vec<&'db sapling_data_model::Fact> {
    let machine = self.query_engine.query(
      self.database,
      query,
      self.variable_bank.clone(),
      self.variable_allocator.clone(),
    );
    machine.map(|f| f.fact).collect()
  }
}
