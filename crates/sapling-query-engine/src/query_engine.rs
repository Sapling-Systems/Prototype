use sapling_data_model::{Fact, Query, Subject};
use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Arc};

use crate::{
  Database, SharedVariableAllocator, SharedVariableBank, System,
  database::match_subject,
  explain::{ExplainQuery, ExplainResult},
  instructions::UnificationInstruction,
  machine::AbstractMachine,
  meta::QueryMeta,
  variable_allocator::VariableAllocator,
  variable_bank::VariableBank,
  watcher::QueryWatcher,
};

pub struct QueryEngine {
  database: Arc<Database>,
}

impl QueryEngine {
  pub fn new(database: Arc<Database>) -> Self {
    Self { database }
  }

  fn get_evaluated_query(&self, query: &Query, enforced_precedence: &[usize]) -> EvaluatedQuery {
    let mut target_subject: Option<&Subject> = None;
    let mut constraints = Vec::new();

    for (fact_index, fact) in self.database.iter_naive_facts() {
      if fact.subject.evaluated {
        // TODO: Lookup this subject to figure out if it evaluates to currently looked for subject
        continue;
      }

      // Must match subject
      if !match_subject(&fact.subject.subject, &query.subject) {
        continue;
      }

      // Skip meta facts
      if match_subject(&fact.meta, &System::CORE_META) {
        continue;
      }

      // Change query target
      if match_subject(&fact.property.subject, &System::CORE_QUERY_TARGET) {
        target_subject = Some(&fact.value.subject);
        continue;
      }

      // Ignore non-query operators
      if match_subject(&fact.operator, &System::CORE_OPERATOR_IS) {
        continue;
      }

      constraints.push((fact_index, constraints.len(), fact));
    }

    constraints.sort_unstable_by_key(|&(_, index, _)| {
      enforced_precedence
        .iter()
        .position(|&i| i == index)
        .unwrap_or(index + 100)
    });

    EvaluatedQuery {
      constraints,
      target_subject,
    }
  }

  fn build_evaluation_instructions(
    &self,
    query: &Query,
    yield_facts: bool,
    target_facts_precedence: &[usize],
    explain: Option<&ExplainQuery>,
    skip_empty: bool,
    variable_allocator: SharedVariableAllocator,
    preset_subject_variable: Option<usize>,
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

    let evaluated_query = self.get_evaluated_query(query, target_facts_precedence);

    let mut instructions = Vec::new();
    let query_name = System::get_subject_name(&self.database, &query.subject);

    if evaluated_query.constraints.is_empty() {
      if skip_empty {
        return instructions;
      }

      let subject_variable =
        preset_subject_variable.unwrap_or_else(|| variable_allocator.allocate_raw_variable());

      // Yield everything since there are no further restrictions
      instructions.push(UnificationInstruction::AllocateFrame { size: 1 });
      instructions.push(UnificationInstruction::DebugComment {
        comment: format!("?{} - empty", query_name.unwrap_or_default()),
      });
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

    if let Some(_explain) = explain {
      for (fact_index, query_fact_index, _) in &evaluated_query.constraints {
        instructions.push(UnificationInstruction::TraceConstraintCreate {
          constraint: *query_fact_index,
          fact_index: *fact_index,
        })
      }
    }

    let subject_variable =
      preset_subject_variable.unwrap_or_else(|| variable_allocator.allocate_raw_variable());

    let mut trace_bound = false;
    let mut forced_subject = false;

    for (_, query_fact_index, query_fact) in evaluated_query.constraints.into_iter() {
      let fact_string = System::get_human_readable_fact(&self.database, query_fact);
      instructions.push(UnificationInstruction::AllocateFrame { size: 64 });
      instructions.push(UnificationInstruction::DebugComment {
        comment: format!("Frame for [{}]", fact_string),
      });

      if !forced_subject && let Some(query_target) = evaluated_query.target_subject {
        instructions.push(UnificationInstruction::BindVariable {
          variable: subject_variable,
          binding: query_target.clone(),
        });
        forced_subject = true;
      }

      if let Some(explain) = explain
        && !trace_bound
      {
        if let Some(target_subject) = &explain.target_subject {
          trace_bound = true;
          instructions.push(UnificationInstruction::BindVariable {
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
        if query_fact.property.evaluated
          && match_subject(&query_fact.property.subject, &System::CORE_INTEGER_PROPERTY)
        {
          instructions.push(UnificationInstruction::CheckPropertyConstAnyInteger);
        } else {
          instructions.push(UnificationInstruction::CheckProperty {
            property: query_fact.property.subject.clone(),
          });
        }
      }

      if query_fact.value.evaluated {
        let variable = variable_allocator.allocate_for_subject(&query_fact.value.subject);

        let sub_fact_instructions = self.build_evaluation_instructions(
          &Query {
            evaluated: true,
            meta: None,
            property: query_fact.value.property.clone(),
            subject: query_fact.value.subject.clone(),
          },
          false,
          &[],
          None,
          true,
          variable_allocator.clone(),
          Some(variable),
        );

        instructions.push(UnificationInstruction::DebugComment {
          comment: format!(
            "Sub query for ?{}",
            System::get_subject_name(&self.database, &query_fact.value.subject).unwrap_or_default()
          ),
        });

        instructions.push(UnificationInstruction::UnifyValue { variable });
        instructions.push(UnificationInstruction::TraceSubQuery {
          query: query_fact.value.subject.clone(),
          variable,
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

  pub fn query<'a>(
    &'a self,
    query: &Query,
    bank: SharedVariableBank,
    allocator: SharedVariableAllocator,
  ) -> AbstractMachine<'a> {
    let instructions =
      self.build_evaluation_instructions(query, true, &[], None, false, allocator.clone(), None);
    AbstractMachine::new(instructions, &self.database, self, bank, allocator)
  }

  fn explain_raw(
    &self,
    explain: &ExplainQuery,
    bank: SharedVariableBank,
    allocator: SharedVariableAllocator,
  ) -> ExplainResult {
    let mut enforced_fact_precedence = explain.facts.keys().cloned().collect::<Vec<_>>();
    enforced_fact_precedence.sort_unstable();

    let instructions = self.build_evaluation_instructions(
      &Query {
        evaluated: true,
        meta: None,
        property: None,
        subject: explain.query_subject.clone(),
      },
      true,
      &enforced_fact_precedence,
      Some(explain),
      false,
      allocator.clone(),
      None,
    );

    let mut machine = AbstractMachine::new(instructions, &self.database, &self, bank, allocator);
    //machine.log_instructions = true;
    while machine.next().is_some() {}

    machine.explain_result
  }

  pub fn explain(
    &self,
    explain_subject: &Subject,
    bank: SharedVariableBank,
    allocator: SharedVariableAllocator,
  ) -> ExplainResult {
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

    self.explain_raw(&query, bank, allocator)
  }
}

#[derive(Debug)]
struct EvaluatedQuery<'a> {
  constraints: Vec<(usize, usize, &'a Fact)>,
  target_subject: Option<&'a Subject>,
}
