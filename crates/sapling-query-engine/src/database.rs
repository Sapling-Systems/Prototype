use sapling_data_model::{Fact, Subject};

pub struct Database {
  raw: Vec<Fact>,
  subject_next_id: u128,
}

impl Database {
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self {
    Self {
      raw: Vec::with_capacity(1024),
      subject_next_id: 0,
    }
  }

  pub fn new_static_subject(&mut self) -> Subject {
    let subject = Subject::Static {
      uuid: self.subject_next_id,
    };
    self.subject_next_id += 1;
    subject
  }

  pub fn get_facts_for_subject(&self, subject: &Subject) -> Vec<&Fact> {
    let mut results = Vec::new();
    for fact in &self.raw {
      if fact.subject.evaluated {
        // TODO: Lookup this subject to figure out if it evaluates to currently looked for subject
        continue;
      }

      if match_subject(&fact.subject.subject, subject) {
        results.push(fact);
      }
    }
    results
  }
}

#[inline]
fn match_subject(a: &Subject, b: &Subject) -> bool {
  match (a, b) {
    (Subject::Static { uuid: a_uuid }, Subject::Static { uuid: b_uuid }) => a_uuid == b_uuid,
    (Subject::Integer { value: a_value }, Subject::Integer { value: b_value }) => {
      a_value == b_value
    }
    (Subject::Float { value: a_value }, Subject::Float { value: b_value }) => {
      todo!("handling of float precision??")
    }
    (Subject::String { value: a_value }, Subject::String { value: b_value }) => a_value == b_value,
    _ => false,
  }
}
