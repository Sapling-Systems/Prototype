use sapling_data_model::{Fact, Subject};

use crate::{meta::QueryMeta, system::System};

#[derive(Clone, Debug)]
pub struct Database {
  pub(crate) raw: Vec<Fact>,
  subject_next_id: u128,
}

impl Database {
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self {
    let mut db = Self {
      raw: Vec::with_capacity(1024),
      subject_next_id: 0,
    };
    System::install(&mut db);
    db
  }

  pub fn new_static_subject(&mut self) -> Subject {
    let subject = Subject::Static {
      uuid: self.subject_next_id,
    };
    self.subject_next_id += 1;
    subject
  }

  pub fn add_fact(&mut self, fact: Fact) -> usize {
    self.raw.push(fact);
    self.raw.len() - 1
  }

  pub fn get_fact(&self, index: usize) -> Option<&Fact> {
    self.raw.get(index)
  }

  pub fn get_query_meta(&self, meta_subject: &Subject) -> QueryMeta {
    if match_subject(meta_subject, &System::CORE_META_INCLUDE) {
      return QueryMeta {
        include_system_meta: true,
      };
    }

    QueryMeta::default()
  }

  pub fn get_facts_for_subject(
    &self,
    subject: &Subject,
    query_meta: &QueryMeta,
    assignments: bool,
  ) -> Vec<&Fact> {
    let mut results = Vec::new();
    for fact in &self.raw {
      if fact.subject.evaluated {
        // TODO: Lookup this subject to figure out if it evaluates to currently looked for subject
        continue;
      }

      // Must match subject
      if !match_subject(&fact.subject.subject, subject) {
        continue;
      }

      // Skip meta facts
      // TODO: this should be based on the queries meta
      if match_subject(&fact.meta, &System::CORE_META) && !query_meta.include_system_meta {
        continue;
      }

      if !assignments && match_subject(&fact.operator, &System::CORE_OPERATOR_IS) {
        continue;
      }

      results.push(fact);
    }
    results
  }
}

#[inline]
pub(crate) fn match_subject(a: &Subject, b: &Subject) -> bool {
  match (a, b) {
    (Subject::Static { uuid: a_uuid }, Subject::Static { uuid: b_uuid }) => a_uuid == b_uuid,
    (Subject::Integer { value: a_value }, Subject::Integer { value: b_value }) => {
      a_value == b_value
    }
    (Subject::Float { value: a_value }, Subject::Float { value: b_value }) => a_value == b_value,
    (Subject::String { value: a_value }, Subject::String { value: b_value }) => a_value == b_value,
    _ => false,
  }
}
