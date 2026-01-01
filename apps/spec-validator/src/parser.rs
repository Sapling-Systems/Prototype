use anyhow::{Context, Result};
use pest::Parser;
use pest_derive::Parser;
use sapling_app::App;
use sapling_data_model::{Fact, Subject, SubjectSelector};
use sapling_query_engine::{Database, System};
use sapling_std::StandardLibrary;
use std::collections::HashMap;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct SpecParser;

#[derive(Debug, Clone)]
pub struct ExpectedFact {
  pub fact: Fact,
  pub subject_mapping: Option<Subject>,
}

#[derive(Debug, Clone)]
pub struct Query {
  pub subject: Subject,
  pub subject_evaluated: bool,
  pub expected_facts: Vec<ExpectedFact>,
  pub property: Option<Subject>,
}

#[derive(Debug, Clone)]
pub struct ExplainQuery {
  pub subject: Subject,
  pub expected_lines: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum TestLine {
  Fact(Fact, Option<String>), // Fact and optional identifier
  Query(Query),
  ExplainQuery(ExplainQuery),
}

#[derive(Debug, Clone)]
pub struct TestCase {
  pub lines: Vec<TestLine>,
}

pub struct SubjectRegistry {
  static_subjects: HashMap<String, Subject>,
  fact_identifiers: HashMap<String, usize>,
  app: App,
}

impl SubjectRegistry {
  pub fn new() -> Self {
    let mut app = App::new(128);
    app.add_plugin(StandardLibrary::default());

    Self {
      static_subjects: HashMap::new(),
      fact_identifiers: HashMap::new(),
      app,
    }
  }

  fn get_or_create_static_subject(&mut self, name: &str) -> Subject {
    let system_subject = self.app.get_global_by_name(name);
    if let Some(system_subject) = system_subject {
      return system_subject;
    }

    if let Some(subject) = self.static_subjects.get(name) {
      subject.clone()
    } else {
      let subject = self.app.get_raw_database_mut().new_static_subject();
      self.app.get_raw_database_mut().add_fact(Fact {
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
      Rule::fact_ref_identifier => {
        let name = pair.as_str();
        // Store the fact reference as a string subject for now
        // It will be resolved later when we have all fact IDs
        Ok(Subject::String {
          value: name.to_string(),
        })
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
      Rule::wildcard_subject => Ok(System::CORE_WILDCARD_SUBJECT.clone()),
      _ => unreachable!("Unexpected subject rule: {:?}", pair.as_rule()),
    }
  }

  fn parse_subject_selector(
    &mut self,
    pair: pest::iterators::Pair<Rule>,
  ) -> Result<(SubjectSelector, bool)> {
    let mut evaluated = false;
    let mut property_evaluated = false;
    let mut subject = None;
    let mut property = None;

    let pairc = pair.clone();
    for inner_pair in pair.into_inner() {
      let rule = inner_pair.as_rule();
      match rule {
        Rule::evaluated_marker => {
          let tag = inner_pair.as_node_tag();
          if tag == Some("property") {
            property_evaluated = true;
          } else {
            evaluated = true;
          }
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

    Ok((
      SubjectSelector {
        subject: subject.context("Subject selector must have a subject")?,
        evaluated,
        property,
      },
      property_evaluated,
    ))
  }

  fn parse_fact(
    &mut self,
    pair: pest::iterators::Pair<Rule>,
  ) -> Result<(Fact, Option<Subject>, Option<String>)> {
    let mut left_selector = None;
    let mut value_selector = None;
    let mut meta_subjects = Vec::new();
    let mut subject_mapping = None;
    let mut fact_identifier = None;

    let mut operator = System::CORE_OPERATOR_IS.clone();

    let pair_clone = pair.clone();

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
        Rule::subject_mapping => {
          for mapping_pair in inner_pair.into_inner() {
            if let Rule::subject = mapping_pair.as_rule() {
              subject_mapping = Some(self.parse_subject(mapping_pair)?);
            }
          }
        }
        Rule::fact_identifier => {
          for id_pair in inner_pair.into_inner() {
            if let Rule::fact_ref_identifier = id_pair.as_rule() {
              let name = id_pair.as_str();
              // Remove the @ prefix
              fact_identifier = Some(name[1..].to_string());
            }
          }
        }
        _ => {}
      }
    }

    let (left, property_evaluated) = left_selector.context("Fact must have a left selector")?;

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
          evaluated: property_evaluated,
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

    let fact = Fact {
      subject: subject_selector,
      property: property_selector,
      operator,
      value: value_selector.context("Fact must have a value")?.0,
      meta,
    };

    Ok((fact, subject_mapping, fact_identifier))
  }

  pub fn parse_test_case(&mut self, input: &str) -> Result<TestCase> {
    let pairs = SpecParser::parse(Rule::test_file, input).context("Spec parser")?;

    let mut lines = Vec::new();
    let mut current_query_subject: Option<(Subject, bool)> = None;
    let mut current_query_property: Option<Subject> = None;
    let mut current_expected_facts = Vec::new();
    let mut current_explain_subject: Option<Subject> = None;
    let mut current_expected_explain_lines = Vec::new();

    for pair in pairs {
      match pair.as_rule() {
        Rule::test_file => {
          for test_line in pair.into_inner() {
            match test_line.as_rule() {
              Rule::test_line => {
                for line_content in test_line.into_inner() {
                  match line_content.as_rule() {
                    Rule::fact => {
                      // Flush any pending queries
                      if let Some((subject, evaluated)) = current_query_subject.take() {
                        lines.push(TestLine::Query(Query {
                          subject,
                          subject_evaluated: evaluated,
                          expected_facts: current_expected_facts,
                          property: current_query_property.clone(),
                        }));
                        current_expected_facts = Vec::new();
                      }
                      if let Some(explain_subject) = current_explain_subject.take() {
                        lines.push(TestLine::ExplainQuery(ExplainQuery {
                          subject: explain_subject,
                          expected_lines: current_expected_explain_lines,
                        }));
                        current_expected_explain_lines = Vec::new();
                      }
                      let (fact, _subject_mapping, fact_identifier) =
                        self.parse_fact(line_content)?;

                      lines.push(TestLine::Fact(fact, fact_identifier));
                    }
                    Rule::query_line => {
                      // Flush any pending queries
                      if let Some((subject, evaluated)) = current_query_subject.take() {
                        lines.push(TestLine::Query(Query {
                          subject,
                          subject_evaluated: evaluated,
                          expected_facts: current_expected_facts,
                          property: current_query_property.clone(),
                        }));
                        current_expected_facts = Vec::new();
                      }
                      if let Some(explain_subject) = current_explain_subject.take() {
                        lines.push(TestLine::ExplainQuery(ExplainQuery {
                          subject: explain_subject,
                          expected_lines: current_expected_explain_lines,
                        }));
                        current_expected_explain_lines = Vec::new();
                      }

                      for query_pair in line_content.into_inner() {
                        if let Rule::subject_selector = query_pair.as_rule() {
                          let (selector, _) = self.parse_subject_selector(query_pair)?;
                          current_query_subject = Some((selector.subject, selector.evaluated));
                          current_query_property = selector.property;
                        }
                      }
                    }
                    Rule::expected_line => {
                      for expected_pair in line_content.into_inner() {
                        if let Rule::fact = expected_pair.as_rule() {
                          let (fact, subject_mapping, _fact_identifier) =
                            self.parse_fact(expected_pair)?;
                          current_expected_facts.push(ExpectedFact {
                            fact,
                            subject_mapping,
                          });
                        }
                      }
                    }
                    Rule::expected_explain_line => {
                      // If we have a regular query pending, this is the start of an explain query
                      // Convert it to an explain query
                      if let Some((subject, _evaluated)) = current_query_subject.take() {
                        current_explain_subject = Some(subject);
                        current_query_property = None;
                        current_expected_facts.clear();
                      }

                      // Extract the text after "#>"
                      let text = line_content.as_str();
                      let explain_line = text.strip_prefix("#>").unwrap_or(text).trim().to_string();
                      current_expected_explain_lines.push(explain_line);
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

    // Flush any remaining queries
    if let Some((subject, evaluated)) = current_query_subject {
      lines.push(TestLine::Query(Query {
        subject,
        subject_evaluated: evaluated,
        expected_facts: current_expected_facts,
        property: current_query_property.clone(),
      }));
    }
    if let Some(explain_subject) = current_explain_subject {
      lines.push(TestLine::ExplainQuery(ExplainQuery {
        subject: explain_subject,
        expected_lines: current_expected_explain_lines,
      }));
    }

    Ok(TestCase { lines })
  }

  pub fn into_database(self) -> (App, HashMap<String, usize>) {
    (self.app, self.fact_identifiers)
  }
}
