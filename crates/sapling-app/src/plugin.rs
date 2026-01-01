use sapling_data_model::{Fact, Subject, SubjectSelector};
use sapling_query_engine::{
  Database, DatabaseWatcher, QueryEngine, QueryWatcher, SharedVariableAllocator,
  SharedVariableBank, System,
};
use sapling_serialization::{DeserializeError, SaplingDeserializable, SaplingSerializable};

use crate::{registry::AppRegistry, serialization::AppPluginSerializerContext};

pub trait AppPlugin {
  fn install_plugin(&mut self, context: &mut AppPluginInstallContext);
}

pub struct AppPluginInstallContext<'a> {
  database: &'a mut Database,
  watcher: &'a mut DatabaseWatcher,
  registry: &'a mut AppRegistry,
  query_engine: &'a mut QueryEngine,
  variable_bank: SharedVariableBank,
  variable_allocator: SharedVariableAllocator,
}

impl<'a> AppPluginInstallContext<'a> {
  pub(crate) fn new(
    database: &'a mut Database,
    watcher: &'a mut DatabaseWatcher,
    query_engine: &'a mut QueryEngine,
    variable_bank: SharedVariableBank,
    variable_allocator: SharedVariableAllocator,
    registry: &'a mut AppRegistry,
  ) -> Self {
    Self {
      database,
      watcher,
      query_engine,
      variable_bank,
      variable_allocator,
      registry,
    }
  }

  pub fn add_interop_fn<F, TArg, TOut>(&mut self, name: &str, result_name: &str, func: F)
  where
    F: Fn(&TArg) -> TOut + Clone + 'static,
    for<'b> TArg: SaplingDeserializable<AppPluginSerializerContext<'b>>,
    for<'b> TOut: SaplingSerializable<AppPluginSerializerContext<'b>>,
  {
    let subject = self.registry.create_global(self.database, name.into());
    let result_subject = self
      .registry
      .create_global(self.database, result_name.into());

    let queries_for_input = {
      let mut context = AppPluginSerializerContext::new(
        self.database,
        self.query_engine,
        self.variable_bank.clone(),
        self.variable_allocator.clone(),
        Some(&mut self.registry),
      );
      TArg::first_level_queries(&subject, &mut context)
    };

    let handler = Box::new(
      move |subject: &Subject,
            name: &str,
            database: &mut Database,
            query_engine: &QueryEngine,
            variable_bank: SharedVariableBank,
            variable_allocator: SharedVariableAllocator| {
        let result = {
          let mut context = AppPluginSerializerContext::new(
            database,
            query_engine,
            variable_bank.clone(),
            variable_allocator.clone(),
            None,
          );
          let argument = TArg::deserialize_subject(subject, &mut context);
          if let Err(err) = argument {
            match err {
              DeserializeError::MissingFact { .. } => {
                // expected error when subject does not have any properties yet
              }
              _ => {
                eprintln!(
                  "Failed to deserialize argument for interop function '{}' - {:?}",
                  name, err
                );
              }
            }
            return;
          }
          let result = func(&argument.unwrap());
          result.serialize_to_facts(&mut context, &name)
        };

        database.add_fact(Fact {
          subject: SubjectSelector {
            evaluated: false,
            property: None,
            subject: subject.clone(),
          },
          property: SubjectSelector {
            evaluated: false,
            property: None,
            subject: result_subject.clone(),
          },
          meta: Subject::String {
            value: "default".into(),
          },
          operator: System::CORE_OPERATOR_IS,
          value: SubjectSelector {
            subject: result,
            evaluated: false,
            property: None,
          },
        });
      },
    );

    for query in queries_for_input {
      self.watcher.watch(
        &query,
        InteropFunctionWatcher {
          subject: subject.clone(),
          name: result_name.into(),
          handler: handler.clone(),
        },
      );
    }
  }
}

struct InteropFunctionWatcher {
  subject: Subject,
  name: String,
  handler: Box<
    dyn Fn(
      &Subject,
      &str,
      &mut Database,
      &QueryEngine,
      SharedVariableBank,
      SharedVariableAllocator,
    ),
  >,
}

impl std::fmt::Debug for InteropFunctionWatcher {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("InteropFunctionWatcher").finish()
  }
}

impl QueryWatcher for InteropFunctionWatcher {
  fn on_change(
    &mut self,
    database: &mut Database,
    query_engine: &QueryEngine,
    variable_bank: SharedVariableBank,
    variable_allocator: SharedVariableAllocator,
  ) {
    (*self.handler)(
      &self.subject,
      &self.name,
      database,
      query_engine,
      variable_bank,
      variable_allocator,
    )
  }
}
