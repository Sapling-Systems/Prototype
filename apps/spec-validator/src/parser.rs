use anyhow::{Context, Result};
use pest::Parser;
use pest_derive::Parser;
use sapling_data_model::{Fact, Subject, SubjectSelector};
use sapling_query_engine::{Database, System};
use std::collections::HashMap;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct SpecParser;

#[derive(Debug, Clone)]
pub struct Query {
  pub subject: Subject,
  pub subject_evaluated: bool,
  pub expected_facts: Vec<Fact>,
}

#[derive(Debug, Clone)]
pub struct TestCase {
  pub facts: Vec<Fact>,
  pub queries: Vec<Query>,
}

pub struct SubjectRegistry {
  static_subjects: HashMap<String, Subject>,
  database: Database,
}

impl SubjectRegistry {
  pub fn new() -> Self {
    Self {
      static_subjects: HashMap::new(),
      database: Database::new(),
    }
  }

  fn get_or_create_static_subject(&mut self, name: &str) -> Subject {
    if let Some(subject) = self.static_subjects.get(name) {
      subject.clone()
    } else {
      let subject = self.database.new_static_subject();
      self.database.add_fact(Fact {
        subject: SubjectSelector {
          subject: subject.clone(),
          evaluated: false,
          property: None,
        },
        property: SubjectSelector {
          subject: System::CORE_PROPERTY_SUBJECT_NAME,
          evaluated: false,
          property: None,
        },
        operator: System::CORE_OPERATOR_IS,
        value: SubjectSelector {
          subject: Subject::String {
            value: name.to_string(),
          },
          evaluated: false,
          property: None,
        },
        meta: System::CORE_META,
      });
      self
        .static_subjects
        .insert(name.to_string(), subject.clone());
      subject
    }
  }

  fn parse_subject(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<Subject> {
    match pair.as_rule() {
      Rule::subject => {
        // subject is a wrapper rule, parse its inner content
        #[allow(clippy::never_loop)]
        for inner_pair in pair.into_inner() {
          return self.parse_subject(inner_pair);
        }
        Err(anyhow::anyhow!("Empty subject rule"))
      }
      Rule::identifier => {
        let name = pair.as_str();
        Ok(self.get_or_create_static_subject(name))
      }
      Rule::integer => {
        let value = pair.as_str().parse::<i64>()?;
        Ok(Subject::Integer { value })
      }
      Rule::float => {
        let value = pair.as_str().parse::<f64>()?;
        Ok(Subject::Float { value })
      }
      Rule::string => {
        let value = pair.as_str();
        // Remove quotes (both single and double)
        let value = if value.starts_with('"') && value.ends_with('"') {
          &value[1..value.len() - 1]
        } else if value.starts_with('\'') && value.ends_with('\'') {
          &value[1..value.len() - 1]
        } else {
          // Fallback: remove first and last char
          &value[1..value.len() - 1]
        };
        Ok(Subject::String {
          value: value.to_string(),
        })
      }
      _ => unreachable!("Unexpected subject rule: {:?}", pair.as_rule()),
    }
  }

  fn parse_subject_selector(
    &mut self,
    pair: pest::iterators::Pair<Rule>,
  ) -> Result<SubjectSelector> {
    let mut evaluated = false;
    let mut subject = None;
    let mut property = None;

    for inner_pair in pair.into_inner() {
      match inner_pair.as_rule() {
        Rule::evaluated_marker => {
          evaluated = true;
        }
        Rule::subject => {
          if subject.is_none() {
            subject = Some(self.parse_subject(inner_pair)?);
          } else {
            property = Some(self.parse_subject(inner_pair)?);
          }
        }
        _ => {}
      }
    }

    Ok(SubjectSelector {
      subject: subject.context("Subject selector must have a subject")?,
      evaluated,
      property,
    })
  }

  fn parse_fact(&mut self, pair: pest::iterators::Pair<Rule>) -> Result<Fact> {
    let mut left_selector = None;
    let mut value_selector = None;
    let mut meta_subjects = Vec::new();

    let mut operator = System::CORE_OPERATOR_IS.clone();

    for inner_pair in pair.into_inner() {
      match inner_pair.as_rule() {
        Rule::subject_selector => {
          if left_selector.is_none() {
            left_selector = Some(self.parse_subject_selector(inner_pair)?);
          } else {
            value_selector = Some(self.parse_subject_selector(inner_pair)?);
          }
        }
        Rule::assignment_operator => {
          operator = System::CORE_OPERATOR_IS.clone();
        }
        Rule::equals_operator => {
          operator = System::CORE_OPERATOR_EQ.clone();
        }
        Rule::meta_list => {
          for meta_pair in inner_pair.into_inner() {
            if let Rule::meta_subject = meta_pair.as_rule() {
              for subject_pair in meta_pair.into_inner() {
                if let Rule::subject = subject_pair.as_rule() {
                  meta_subjects.push(self.parse_subject(subject_pair)?);
                }
              }
            }
          }
        }
        _ => {}
      }
    }

    let left = left_selector.context("Fact must have a left selector")?;

    // Decompose the left selector into subject and property
    let (subject_selector, property_selector) = if let Some(property) = left.property {
      (
        SubjectSelector {
          subject: left.subject,
          evaluated: left.evaluated,
          property: None,
        },
        SubjectSelector {
          subject: property,
          evaluated: false,
          property: None,
        },
      )
    } else {
      return Err(anyhow::anyhow!(
        "Left selector must have a property (subject/property format)"
      ));
    };

    // Use first meta subject or create a default one
    let meta = if meta_subjects.is_empty() {
      Subject::String {
        value: "default".to_string(),
      }
    } else {
      meta_subjects[0].clone()
    };

    Ok(Fact {
      subject: subject_selector,
      property: property_selector,
      operator,
      value: value_selector.context("Fact must have a value")?,
      meta,
    })
  }

  pub fn parse_test_case(&mut self, input: &str) -> Result<TestCase> {
    let pairs = SpecParser::parse(Rule::test_file, input).context("Spec parser")?;

    let mut facts = Vec::new();
    let mut queries = Vec::new();
    let mut current_query_subject: Option<(Subject, bool)> = None;
    let mut current_expected_facts = Vec::new();

    for pair in pairs {
      match pair.as_rule() {
        Rule::test_file => {
          for test_line in pair.into_inner() {
            match test_line.as_rule() {
              Rule::test_line => {
                for line_content in test_line.into_inner() {
                  match line_content.as_rule() {
                    Rule::fact => {
                      if let Some((subject, evaluated)) = current_query_subject.take() {
                        queries.push(Query {
                          subject,
                          subject_evaluated: evaluated,
                          expected_facts: current_expected_facts,
                        });
                        current_expected_facts = Vec::new();
                      }
                      facts.push(self.parse_fact(line_content)?);
                    }
                    Rule::query_line => {
                      if let Some((subject, evaluated)) = current_query_subject.take() {
                        queries.push(Query {
                          subject,
                          subject_evaluated: evaluated,
                          expected_facts: current_expected_facts,
                        });
                        current_expected_facts = Vec::new();
                      }
                      for query_pair in line_content.into_inner() {
                        if let Rule::subject_selector = query_pair.as_rule() {
                          let selector = self.parse_subject_selector(query_pair)?;
                          current_query_subject = Some((selector.subject, selector.evaluated));
                        }
                      }
                    }
                    Rule::expected_line => {
                      for expected_pair in line_content.into_inner() {
                        if let Rule::fact = expected_pair.as_rule() {
                          current_expected_facts.push(self.parse_fact(expected_pair)?);
                        }
                      }
                    }
                    _ => {}
                  }
                }
              }
              _ => {}
            }
          }
        }
        _ => {}
      }
    }

    // Don't forget the last query if it exists
    if let Some((subject, evaluated)) = current_query_subject {
      queries.push(Query {
        subject,
        subject_evaluated: evaluated,
        expected_facts: current_expected_facts,
      });
    }

    Ok(TestCase { facts, queries })
  }

  pub fn into_database(self) -> Database {
    self.database
  }
}
