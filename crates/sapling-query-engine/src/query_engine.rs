use sapling_data_model::Query;
use std::sync::Arc;

use crate::{
  Database, System,
  database::match_subject,
  machine::{AbstractMachine, UnificationInstruction},
};

pub struct QueryEngine {
  database: Arc<Database>,
}

impl QueryEngine {
  pub fn new(database: Arc<Database>) -> Self {
    Self { database }
  }

  fn build_evaluation_instructions(
    &self,
    query: &Query,
    subject_variable: usize,
    yield_facts: bool,
  ) -> Vec<UnificationInstruction> {
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
        if yield_facts
          && query
            .property
            .as_ref()
            .map(|property| match_subject(property, &fact.property.subject))
            .unwrap_or(true)
        {
          instructions.push(UnificationInstruction::MaybeYield);
        }
        instructions.push(UnificationInstruction::CheckSubject {
          subject: query.subject.clone(),
        });
        instructions.push(UnificationInstruction::CheckProperty {
          property: fact.property.subject.clone(),
        });
        instructions.push(UnificationInstruction::CheckValue {
          value: fact.value.subject.clone(),
          property: fact.value.property.clone(),
        });
      }
      if yield_facts {
        instructions.push(UnificationInstruction::YieldAll);
      }

      return instructions;
    }

    let mut instructions = Vec::new();

    for query_fact in target_facts {
      instructions.push(UnificationInstruction::AllocateFrame { size: 64 });
      let expect_property_yield = query
        .property
        .as_ref()
        .map(|property| match_subject(property, &query_fact.property.subject))
        .unwrap_or(true);
      if yield_facts && expect_property_yield {
        instructions.push(UnificationInstruction::MaybeYield);
      }
      instructions.push(UnificationInstruction::CheckOperator {
        operator: System::CORE_OPERATOR_IS,
      });
      instructions.push(UnificationInstruction::UnifySubject {
        variable: subject_variable,
      });
      if !match_subject(&query_fact.property.subject, &System::CORE_WILDCARD_SUBJECT) {
        instructions.push(UnificationInstruction::CheckProperty {
          property: query_fact.property.subject.clone(),
        });
      }

      if query_fact.value.evaluated {
        let sub_fact_instructions = self.build_evaluation_instructions(
          &Query {
            evaluated: true,
            meta: None,
            property: query_fact.value.property.clone(),
            subject: query_fact.value.subject.clone(),
          },
          subject_variable + 1,
          false,
        );
        instructions.push(UnificationInstruction::UnifyValue {
          variable: subject_variable + 1,
        });
        instructions.extend(sub_fact_instructions);
      } else if !match_subject(&query_fact.value.subject, &System::CORE_WILDCARD_SUBJECT) {
        instructions.push(UnificationInstruction::CheckValue {
          value: query_fact.value.subject.clone(),
          property: query_fact.value.property.clone(),
        });
      }
    }
    if yield_facts {
      instructions.push(UnificationInstruction::YieldAll);
    }
    instructions
  }

  pub fn query<'a>(&'a self, query: &Query) -> AbstractMachine<'a> {
    let instructions = self.build_evaluation_instructions(query, 0, true);
    AbstractMachine::new(instructions, &self.database, &self)
  }
}
