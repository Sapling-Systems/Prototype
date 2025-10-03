use sapling_data_model::{Fact, Query, Subject};
use std::{ops::Sub, sync::Arc};

use crate::{
  Database, System,
  machine::{AbstractMachine, UnificationInstruction},
};

pub struct QueryEngine {
  database: Arc<Database>,
}

impl QueryEngine {
  pub fn new(database: Arc<Database>) -> Self {
    Self { database }
  }

  pub fn query<'a>(&'a self, query: &Query) -> AbstractMachine<'a> {
    let meta = query
      .meta
      .as_ref()
      .map(|m| self.database.get_query_meta(m))
      .unwrap_or_default();

    let target_facts = self
      .database
      .get_facts_for_subject(&query.subject, &meta, !query.evaluated);

    if !query.evaluated {
      let mut instructions = vec![];

      for fact in target_facts {
        instructions.push(UnificationInstruction::AllocateFrame { size: 0 });
        instructions.push(UnificationInstruction::NextFact);
        instructions.push(UnificationInstruction::CheckSubject {
          subject: query.subject.clone(),
        });
        instructions.push(UnificationInstruction::CheckProperty {
          property: fact.property.subject.clone(),
        });
        instructions.push(UnificationInstruction::CheckValue {
          value: fact.value.subject.clone(),
        });
        instructions.push(UnificationInstruction::MaybeYield);
      }
      instructions.push(UnificationInstruction::YieldAll);

      return AbstractMachine::new(instructions, &self.database);
    }

    let mut instructions = Vec::new();
    let subject_variable = 0;

    for query_fact in target_facts {
      instructions.push(UnificationInstruction::AllocateFrame { size: 1 });
      instructions.push(UnificationInstruction::NextFact);
      instructions.push(UnificationInstruction::CheckOperator {
        operator: System::CORE_OPERATOR_IS,
      });
      instructions.push(UnificationInstruction::UnifySubject {
        variable: subject_variable,
      });
      instructions.push(UnificationInstruction::CheckProperty {
        property: query_fact.property.subject.clone(),
      });
      instructions.push(UnificationInstruction::CheckValue {
        value: query_fact.value.subject.clone(),
      });
      instructions.push(UnificationInstruction::MaybeYield);
    }
    instructions.push(UnificationInstruction::YieldAll);

    println!("Instructions generated: \n{:#?}", instructions);

    AbstractMachine::new(instructions, &self.database)
  }
}
