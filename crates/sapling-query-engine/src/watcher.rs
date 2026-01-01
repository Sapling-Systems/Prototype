use std::{
  fmt::Debug,
  hash::{DefaultHasher, Hash, Hasher},
};

use sapling_data_model::Query;

use crate::{Database, QueryEngine, SharedVariableAllocator, SharedVariableBank};

pub trait QueryWatcher: Debug {
  fn on_change(
    &mut self,
    database: &mut Database,
    query_engine: &QueryEngine,
    variable_bank: SharedVariableBank,
    variable_allocator: SharedVariableAllocator,
  );
}

#[derive(Debug)]
struct SingleWatcher {
  root_query: Query,
  last_hash: u64,
  watcher: Box<dyn QueryWatcher>,
}

impl SingleWatcher {
  fn generate_result_hash(fact_ids: &[usize]) -> u64 {
    let mut hasher = DefaultHasher::new();
    fact_ids.hash(&mut hasher);
    hasher.finish()
  }

  fn new<T: QueryWatcher + 'static>(query: &Query, watcher: T) -> Self {
    let last_hash = Self::generate_result_hash(&[]);

    SingleWatcher {
      root_query: query.clone(),
      last_hash,
      watcher: Box::new(watcher),
    }
  }

  fn recursive_gather_dependencies(
    database: &Database,
    query_engine: &QueryEngine,
    variable_bank: SharedVariableBank,
    variable_allocator: SharedVariableAllocator,
    query: &Query,
    result: &mut Vec<usize>,
  ) {
    let machine = query_engine.query(
      database,
      query,
      variable_bank.clone(),
      variable_allocator.clone(),
    );
    for fact in machine {
      result.push(fact.fact_index);

      if fact.fact.value.evaluated {
        Self::recursive_gather_dependencies(
          database,
          query_engine,
          variable_bank.clone(),
          variable_allocator.clone(),
          &Query {
            evaluated: fact.fact.value.evaluated,
            property: fact.fact.value.property.clone(),
            subject: fact.fact.value.subject.clone(),
            meta: None,
          },
          result,
        );
      }
    }
  }

  fn handle_new_fact(
    &mut self,
    database: &mut Database,
    query_engine: &QueryEngine,
    variable_bank: SharedVariableBank,
    variable_allocator: SharedVariableAllocator,
    _new_fact_index: usize,
  ) {
    let mut fact_ids = Vec::new();
    Self::recursive_gather_dependencies(
      database,
      query_engine,
      variable_bank.clone(),
      variable_allocator.clone(),
      &self.root_query,
      &mut fact_ids,
    );
    let hash = Self::generate_result_hash(&fact_ids);

    if hash != self.last_hash {
      self.watcher.on_change(
        database,
        query_engine,
        variable_bank.clone(),
        variable_allocator.clone(),
      );
      self.last_hash = hash;
    }

    variable_bank.truncate_checkpoint(0);
  }
}

#[derive(Debug)]
pub struct DatabaseWatcher {
  watchers: Vec<SingleWatcher>,
}

impl DatabaseWatcher {
  pub fn new() -> Self {
    DatabaseWatcher {
      watchers: Vec::new(),
    }
  }

  pub fn watch<'a, T: QueryWatcher + 'static>(&'a mut self, query: &Query, watcher: T) {
    let watcher = SingleWatcher::new(query, watcher);
    self.watchers.push(watcher);
  }

  pub fn handle_new_fact(
    &mut self,
    database: &mut Database,
    query_engine: &QueryEngine,
    variable_bank: SharedVariableBank,
    variable_allocator: SharedVariableAllocator,
    new_fact_index: usize,
  ) {
    for watcher in &mut self.watchers {
      watcher.handle_new_fact(
        database,
        query_engine,
        variable_bank.clone(),
        variable_allocator.clone(),
        new_fact_index,
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use std::sync::atomic::{AtomicUsize, Ordering};

  use sapling_data_model::{Fact, Query, Subject, SubjectSelector};

  use crate::{
    Database, QueryEngine, SharedVariableAllocator, SharedVariableBank, System,
    watcher::{DatabaseWatcher, QueryWatcher},
  };

  #[test]
  fn test_watcher_simple() {
    static CHANGE_COUNT: AtomicUsize = AtomicUsize::new(0);

    let mut database = Database::new();
    System::install(&mut database);

    let query = database.new_static_subject();
    let prop1 = database.new_static_subject();

    database.add_fact(Fact {
      meta: Subject::String {
        value: "default meta".into(),
      },
      operator: System::CORE_OPERATOR_EQ.clone(),
      subject: SubjectSelector {
        evaluated: false,
        subject: query.clone(),
        property: None,
      },
      property: SubjectSelector {
        evaluated: false,
        subject: prop1.clone(),
        property: None,
      },
      value: SubjectSelector {
        evaluated: false,
        subject: Subject::String {
          value: "find me".into(),
        },
        property: None,
      },
    });

    let mut watcher = DatabaseWatcher::new();
    let query_engine = QueryEngine::new();
    let variable_allocator = SharedVariableAllocator::new();
    let variable_bank = SharedVariableBank::new(128);

    let query = Query {
      evaluated: true,
      meta: None,
      property: None,
      subject: query,
    };
    watcher.watch(&query, TestWatcher);
    assert_eq!(CHANGE_COUNT.load(Ordering::Relaxed), 0);

    let data1 = database.new_static_subject();
    let fact1 = database.add_fact(Fact {
      meta: System::CORE_META.clone(),
      operator: System::CORE_OPERATOR_IS.clone(),
      subject: SubjectSelector {
        evaluated: false,
        subject: data1.clone(),
        property: None,
      },
      property: SubjectSelector {
        evaluated: false,
        subject: prop1.clone(),
        property: None,
      },
      value: SubjectSelector {
        evaluated: false,
        subject: Subject::String {
          value: "don't find me".into(),
        },
        property: None,
      },
    });

    watcher.handle_new_fact(
      &mut database,
      &query_engine,
      variable_bank.clone(),
      variable_allocator.clone(),
      fact1,
    );
    assert_eq!(CHANGE_COUNT.load(Ordering::Relaxed), 0);

    let data2 = database.new_static_subject();
    let fact2 = database.add_fact(Fact {
      meta: System::CORE_META.clone(),
      operator: System::CORE_OPERATOR_IS.clone(),
      subject: SubjectSelector {
        evaluated: false,
        subject: data2.clone(),
        property: None,
      },
      property: SubjectSelector {
        evaluated: false,
        subject: prop1.clone(),
        property: None,
      },
      value: SubjectSelector {
        evaluated: false,
        subject: Subject::String {
          value: "find me".into(),
        },
        property: None,
      },
    });

    watcher.handle_new_fact(
      &mut database,
      &query_engine,
      variable_bank,
      variable_allocator,
      fact2,
    );

    assert_eq!(CHANGE_COUNT.load(Ordering::Relaxed), 1);

    #[derive(Clone, Debug)]
    struct TestWatcher;
    impl QueryWatcher for TestWatcher {
      fn on_change(
        &mut self,
        _database: &mut Database,
        _query_engine: &QueryEngine,
        _variable_bank: SharedVariableBank,
        _variable_allocator: SharedVariableAllocator,
      ) {
        CHANGE_COUNT.fetch_add(1, Ordering::Relaxed);
      }
    }
  }
}
