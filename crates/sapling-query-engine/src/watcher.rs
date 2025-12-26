use std::fmt::Debug;

use sapling_data_model::{Fact, Query};

use crate::{
  Database, QueryEngine, SharedVariableAllocator, SharedVariableBank,
  instructions::UnificationInstruction, machine::AbstractMachine,
};

pub trait QueryWatcher: Debug {
  fn on_change(&mut self);
  fn box_clone(&self) -> Box<dyn QueryWatcher>;
}

#[derive(Debug)]
struct SingleWatcher {
  check_instructions: Vec<UnificationInstruction>,
  watcher: Box<dyn QueryWatcher>,
}

impl Clone for SingleWatcher {
  fn clone(&self) -> Self {
    SingleWatcher {
      check_instructions: self.check_instructions.clone(),
      watcher: self.watcher.box_clone(),
    }
  }
}

impl SingleWatcher {
  fn new<T: QueryWatcher + 'static>(
    query: &Query,
    database: &Database,
    query_engine: &QueryEngine,
    variable_allocator: SharedVariableAllocator,
    watcher: T,
  ) -> Self {
    let instructions = query_engine.build_evaluation_instructions(
      database,
      query,
      true,
      &[],
      None,
      false,
      variable_allocator,
      None,
      Some(1),
    );

    SingleWatcher {
      check_instructions: instructions,
      watcher: Box::new(watcher),
    }
  }

  fn handle_new_fact(
    &mut self,
    database: &Database,
    query_engine: &QueryEngine,
    variable_bank: SharedVariableBank,
    variable_allocator: SharedVariableAllocator,
    new_fact_index: usize,
  ) {
    let mut instructions = self.check_instructions.clone();
    for instruction in &mut instructions {
      match instruction {
        UnificationInstruction::AllocateFact { fact_index, .. } => {
          *fact_index = new_fact_index;
        }
        _ => {}
      }
    }

    let mut machine = AbstractMachine::new(
      instructions,
      database,
      query_engine,
      variable_bank.clone(),
      variable_allocator,
    );
    if machine.next().is_some() {
      self.watcher.on_change();
    }

    variable_bank.truncate_checkpoint(0);
  }
}

#[derive(Debug, Clone)]
pub struct DatabaseWatcher {
  watchers: Vec<SingleWatcher>,
}

impl DatabaseWatcher {
  pub fn new() -> Self {
    DatabaseWatcher {
      watchers: Vec::new(),
    }
  }

  pub fn watch<'a, T: QueryWatcher + 'static>(
    &'a mut self,
    database: &Database,
    query_engine: &QueryEngine,
    query: &Query,
    watcher: T,
    allocator: SharedVariableAllocator,
  ) {
    let watcher = SingleWatcher::new(query, database, query_engine, allocator, watcher);
    self.watchers.push(watcher);
  }

  pub fn handle_new_fact(
    &mut self,
    database: &Database,
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

    let query = Query {
      evaluated: true,
      meta: None,
      property: None,
      subject: query,
    };

    let mut watcher = DatabaseWatcher::new();
    let query_engine = QueryEngine::new();
    let variable_allocator = SharedVariableAllocator::new();
    let variable_bank = SharedVariableBank::new(128);
    watcher.watch(
      &database,
      &query_engine,
      &query,
      TestWatcher,
      variable_allocator.clone(),
    );

    assert_eq!(CHANGE_COUNT.load(Ordering::Relaxed), 0);

    println!("Handle fact 1");
    watcher.handle_new_fact(
      &database,
      &query_engine,
      variable_bank.clone(),
      variable_allocator.clone(),
      fact1,
    );

    assert_eq!(CHANGE_COUNT.load(Ordering::Relaxed), 0);

    println!("Handle fact 2");
    watcher.handle_new_fact(
      &database,
      &query_engine,
      variable_bank,
      variable_allocator,
      fact2,
    );

    assert_eq!(CHANGE_COUNT.load(Ordering::Relaxed), 1);

    #[derive(Clone, Debug)]
    struct TestWatcher;
    impl QueryWatcher for TestWatcher {
      fn box_clone(&self) -> Box<dyn QueryWatcher> {
        Box::new(self.clone())
      }
      fn on_change(&mut self) {
        CHANGE_COUNT.fetch_add(1, Ordering::Relaxed);
      }
    }
  }
}
