use sapling_data_model::{Query, Subject};
use std::{collections::HashMap, sync::Arc};

use crate::{
  Database, System,
  database::match_subject,
  explain::{ExplainQuery, ExplainResult},
  instructions::UnificationInstruction,
  machine::{AbstractMachine, VariableBinding},
  meta::QueryMeta,
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
    target_facts_precedence: &[usize],
    explain: Option<&ExplainQuery>,
    skip_empty: bool,
  ) -> Vec<UnificationInstruction> {
    let meta = query
      .meta
      .as_ref()
      .map(|m| self.database.get_query_meta(m))
      .unwrap_or_default();

    if !query.evaluated {
      let mut instructions = vec![];

      instructions.push(UnificationInstruction::AllocateFrame { size: 0 });
      instructions.push(UnificationInstruction::CheckSubject {
        subject: query.subject.clone(),
      });

      if let Some(property) = &query.property {
        instructions.push(UnificationInstruction::CheckProperty {
          property: property.clone(),
        });
      }

      if !meta.include_system_meta {
        instructions.push(UnificationInstruction::CheckMeta { skip_system: true });
      }

      if yield_facts {
        instructions.push(UnificationInstruction::MaybeYield);
        instructions.push(UnificationInstruction::YieldAll);
      }

      return instructions;
    }

    let target_facts = self.database.get_facts_for_subject(
      &query.subject,
      &meta,
      !query.evaluated,
      target_facts_precedence,
    );

    let mut instructions = Vec::new();

    if target_facts.is_empty() {
      if skip_empty {
        return instructions;
      }

      // Yield everything since there are no further restrictions
      instructions.push(UnificationInstruction::AllocateFrame { size: 1 });
      instructions.push(UnificationInstruction::MaybeYield);
      instructions.push(UnificationInstruction::SkipSubject {
        subject: query.subject.clone(),
      });
      instructions.push(UnificationInstruction::UnifySubject {
        variable: subject_variable,
      });
      if yield_facts {
        instructions.push(UnificationInstruction::YieldAll);
      }
      return instructions;
    }

    if let Some(explain) = explain {
      for (fact_index, query_fact_index, _) in &target_facts {
        instructions.push(UnificationInstruction::TraceConstraintCreate {
          constraint: *query_fact_index,
          fact_index: *fact_index,
        })
      }
    }

    let mut trace_bound = false;
    for (_, query_fact_index, query_fact) in target_facts.into_iter() {
      instructions.push(UnificationInstruction::AllocateFrame { size: 64 });

      if let Some(explain) = explain
        && !trace_bound
      {
        if let Some(target_subject) = &explain.target_subject {
          trace_bound = true;
          instructions.push(UnificationInstruction::TraceBindVariable {
            variable: subject_variable,
            binding: target_subject.clone(),
          });
        }
      }

      let expect_property_yield = query
        .property
        .as_ref()
        .map(|property| match_subject(property, &query_fact.property.subject))
        .unwrap_or(true);

      if let Some(explain) = explain {
        if let Some(expected_fact_index) = explain.facts.get(&query_fact_index) {
          instructions.push(UnificationInstruction::TraceStartFact {
            fact: *expected_fact_index,
            constraint: query_fact_index,
          });
        }
      }

      instructions.push(UnificationInstruction::CheckOperator {
        operator: System::CORE_OPERATOR_IS,
      });
      instructions.push(UnificationInstruction::UnifySubject {
        variable: subject_variable,
      });
      if yield_facts && expect_property_yield {
        instructions.push(UnificationInstruction::MaybeYield);
      }
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
          &[],
          None,
          true,
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
      if let Some(explain) = explain {
        let mut collection = explain.facts.iter().collect::<Vec<_>>();
        // Prevent random ordering due to hash map iteration
        // Only relevant for spec validator to be able to enforce order everywhere else
        collection.sort_by_key(|(constraint, _)| *constraint);

        for (constraint, fact_index) in collection {
          instructions.push(UnificationInstruction::TraceLogYield {
            constraint: *constraint,
            fact_index: *fact_index,
          })
        }
      }
      instructions.push(UnificationInstruction::YieldAll);
    }

    instructions
  }

  pub fn query<'a>(&'a self, query: &Query) -> AbstractMachine<'a> {
    let instructions = self.build_evaluation_instructions(query, 0, true, &[], None, false);
    AbstractMachine::new(instructions, &self.database, self)
  }

  fn explain_raw(&self, explain: &ExplainQuery) -> ExplainResult {
    let mut enforced_fact_precedence = explain.facts.keys().cloned().collect::<Vec<_>>();
    enforced_fact_precedence.sort_unstable();

    let instructions = self.build_evaluation_instructions(
      &Query {
        evaluated: true,
        meta: None,
        property: None,
        subject: explain.query_subject.clone(),
      },
      0,
      true,
      &enforced_fact_precedence,
      Some(explain),
      false,
    );

    let mut machine = AbstractMachine::new(instructions, &self.database, &self);
    while machine.next().is_some() {}

    machine.explain_result
  }

  pub fn explain(&self, explain_subject: &Subject) -> ExplainResult {
    let target_facts = self.database.get_facts_for_subject(
      explain_subject,
      &QueryMeta {
        include_system_meta: false,
      },
      true,
      &[],
    );

    let mut target_subject = None;
    let mut query_subject = None;
    let mut facts = HashMap::new();

    for (_, _, fact) in target_facts {
      let Some(name) = System::get_subject_name(&self.database, &fact.property.subject) else {
        continue;
      };

      if name == "query" {
        query_subject = Some(fact.value.subject.clone());
      } else if name == "subject" {
        target_subject = Some(fact.value.subject.clone());
      } else if name.starts_with("fact") {
        let fact_index = name.strip_prefix("fact").unwrap().parse().unwrap();
        facts.insert(
          fact_index,
          match fact.value.subject {
            Subject::Integer { value } => value as usize,
            _ => panic!("Invalid fact for explain"),
          },
        );
      }
    }

    let query = ExplainQuery {
      facts,
      query_subject: query_subject.unwrap(),
      target_subject,
    };

    self.explain_raw(&query)
  }
}
