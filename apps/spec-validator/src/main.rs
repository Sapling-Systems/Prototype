use anyhow::{Context, Result};
use colored::*;
use sapling_data_model::{Fact, Query, Subject};
use sapling_query_engine::{FoundFact, QueryEngine, System};
use std::fs;
use std::path::Path;
use std::sync::Arc;

mod parser;
use parser::{SubjectRegistry, TestLine};

fn format_subject(engine: &QueryEngine, subject: &Subject) -> String {
  match subject {
    Subject::Static { uuid } => {
      let name = engine
        .query(&Query {
          subject: subject.clone(),
          property: Some(System::CORE_PROPERTY_SUBJECT_NAME),
          meta: Some(System::CORE_META_INCLUDE),
          evaluated: false,
        })
        .next();

      if let Some(Subject::String { value }) = name.map(|fact| &fact.fact.value.subject) {
        return value.clone();
      }

      format!("static_{}", uuid)
    }
    Subject::Integer { value } => value.to_string(),
    Subject::Float { value } => value.to_string(),
    Subject::String { value } => format!("\"{}\"", value),
  }
}

fn format_fact(engine: &QueryEngine, fact: &Fact) -> String {
  let subject_str = if fact.subject.evaluated {
    format!("?{}", format_subject(engine, &fact.subject.subject))
  } else {
    format_subject(engine, &fact.subject.subject)
  };

  let property_str = if fact.property.evaluated {
    format!("?{}", format_subject(engine, &fact.property.subject))
  } else {
    format_subject(engine, &fact.property.subject)
  };

  let value_str = if fact.value.evaluated {
    format!("?{}", format_subject(engine, &fact.value.subject))
  } else {
    format_subject(engine, &fact.value.subject)
  };

  format!(
    "{}/{} {} {}",
    subject_str,
    property_str,
    format_subject(engine, &fact.operator),
    value_str
  )
}

fn run_test(file_path: &Path) -> Result<bool> {
  let content = fs::read_to_string(file_path)
    .with_context(|| format!("Failed to read file: {:?}", file_path))?;

  let mut registry = SubjectRegistry::new();
  let test_case = registry
    .parse_test_case(&content)
    .with_context(|| format!("Failed to parse test case: {:?}", file_path))?;

  let mut database = registry.into_database();

  let mut success = true;
  let mut query_count = 0;

  println!("{}", format!("Running test: {:?}", file_path).blue().bold());

  for line in test_case.lines {
    match line {
      TestLine::Fact(fact) => {
        database.add_fact(fact);
      }
      TestLine::Query(query) => {
        query_count += 1;
        let engine = QueryEngine::new(Arc::new(database.clone()));

        println!(
          "  {} {} {}{}{}",
          "Query".green().bold(),
          query_count,
          if query.subject_evaluated { "?" } else { "" },
          format_subject(&engine, &query.subject),
          match &query.property {
            Some(subject) => format!("/{}", format_subject(&engine, subject)),
            None => "".to_string(),
          }
        );

        // Check for explain hints
        let explain_facts: Vec<_> = query
          .expected_facts
          .iter()
          .enumerate()
          .filter(|(_, expected)| expected.explain)
          .collect();

        if explain_facts.len() > 1 {
          println!(
            "  {}",
            "FAIL: Multiple !explain hints found in single query"
              .red()
              .bold()
          );
          success = false;
          continue;
        }

        let machine = engine.query(&Query {
          evaluated: query.subject_evaluated,
          meta: None,
          property: query.property.clone(),
          subject: query.subject.clone(),
        });
        let instructions = machine.instructions.clone();

        let actual_facts: Vec<FoundFact> = machine.collect();

        // Set up explainer if there's an explain hint
        let mut explain_result = None;
        if let Some((expected_index, _)) = explain_facts.first() {
          let expected_fact = &query.expected_facts[*expected_index];

          // Find the actual result that matches the expected fact with !explain
          if let Some((index, _)) = database.iter_naive_facts().find(|(_, actual)| {
            format_fact(&engine, actual) == format_fact(&engine, &expected_fact.fact)
          }) {
            // Run query again with explainer
            let mut explain_machine = engine.query(&Query {
              evaluated: query.subject_evaluated,
              meta: None,
              property: query.property.clone(),
              subject: query.subject.clone(),
            });

            // Run the query with explainer to trigger explanation logic
            explain_result = Some(1);
          }
        }

        println!(
          "  {} ({} facts)",
          "Expected:".yellow(),
          query.expected_facts.len()
        );
        for expected in &query.expected_facts {
          let fact_str = format_fact(&engine, &expected.fact);
          let explain_suffix = if expected.explain { " !explain" } else { "" };
          if let Some(subject_mapping) = &expected.subject_mapping {
            println!(
              "    {} ;; subject={}{}",
              fact_str,
              format_subject(&engine, subject_mapping),
              explain_suffix
            );
          } else {
            println!("    {}{}", fact_str, explain_suffix);
          }
        }

        println!("  {} ({} facts)", "Actual:".cyan(), actual_facts.len());
        for found_fact in &actual_facts {
          let fact_str = format_fact(&engine, found_fact.fact);
          if let Some(subject_binding) = &found_fact.subject_binding {
            println!(
              "    {} ;; subject={}",
              fact_str,
              format_subject(&engine, subject_binding)
            );
          } else {
            println!("    {}", fact_str);
          }
        }

        // Compare expected vs actual
        if actual_facts.len() != query.expected_facts.len() {
          println!("  {}", "FAIL: Different number of facts".red().bold());
          success = false;
        } else {
          let mut matches = true;
          let mut failure_reasons = Vec::new();

          // Create a list to track which actual facts have been matched
          let mut matched_actual_indices = Vec::new();

          for expected in &query.expected_facts {
            let mut found_match = false;

            for (idx, actual) in actual_facts.iter().enumerate() {
              // Skip if this actual fact has already been matched
              if matched_actual_indices.contains(&idx) {
                continue;
              }

              // Check if the facts match
              let facts_match =
                format_fact(&engine, actual.fact) == format_fact(&engine, &expected.fact);

              if !facts_match {
                continue;
              }

              // Check if the subject mappings match
              let mapping_matches = match (&expected.subject_mapping, &actual.subject_binding) {
                (None, None) => true,
                (Some(expected_subj), Some(actual_subj)) => {
                  format_subject(&engine, expected_subj) == format_subject(&engine, actual_subj)
                }
                (None, Some(actual_subj)) => {
                  failure_reasons.push(format!(
                    "Fact '{}': Subject mapping was not expected but got: {}",
                    format_fact(&engine, &expected.fact),
                    format_subject(&engine, actual_subj)
                  ));
                  false
                }
                (Some(expected_subj), None) => {
                  failure_reasons.push(format!(
                    "Fact '{}': Expected subject mapping '{}' but got None",
                    format_fact(&engine, &expected.fact),
                    format_subject(&engine, expected_subj)
                  ));
                  false
                }
              };

              if mapping_matches {
                found_match = true;
                matched_actual_indices.push(idx);
                break;
              }
            }

            if !found_match {
              matches = false;
              if failure_reasons.is_empty() {
                failure_reasons.push(format!(
                  "No matching fact found for: {}{}",
                  format_fact(&engine, &expected.fact),
                  if let Some(subj) = &expected.subject_mapping {
                    format!(" ;; subject={}", format_subject(&engine, subj))
                  } else {
                    String::new()
                  }
                ));
              }
              break;
            }
          }

          if matches {
            println!("  {}", "PASS".green().bold());
          } else {
            for reason in &failure_reasons {
              println!("  {}", format!("FAIL: {}", reason).red().bold());
            }
            if failure_reasons.is_empty() {
              println!("  {}", "FAIL: Facts don't match".red().bold());
            }
            success = false;
          }
        }

        // Print explain result if there was an explain hint
        if let Some(explainer) = explain_result {
          /*
          println!("  {}", "Explain:".magenta().bold());
          let explain_text = explainer.explain_text(&database);
          for line in explain_text.lines() {
            println!("    {}", line);
          }
          */

          println!("  {}", "Instr:".magenta().bold());
          let instrs = format!("{:#?}", instructions);
          for line in instrs.lines() {
            println!("    {}", line);
          }
        }

        println!();
      }
    }
  }

  Ok(success)
}

fn main() -> Result<()> {
  let spec_dir = Path::new("./apps/spec-validator/spec");

  if !spec_dir.exists() {
    eprintln!("Spec directory not found. Please ensure 'spec' directory exists.");
    return Ok(());
  }

  let mut total_tests = 0;
  let mut passed_tests = 0;

  for entry in fs::read_dir(spec_dir)? {
    let entry = entry?;
    let path = entry.path();

    #[allow(clippy::unnecessary_map_or)]
    if path.is_file() && path.extension().map_or(false, |ext| ext == "txt") {
      total_tests += 1;
      match run_test(&path) {
        Ok(true) => {
          passed_tests += 1;
          println!("{}", "✓ PASSED".green().bold());
        }
        Ok(false) => {
          println!("{}", "✗ FAILED".red().bold());
        }
        Err(e) => {
          println!("{}: {:?}", "✗ ERROR".red().bold(), e);
        }
      }
      println!("{}", "─".repeat(60));
    }
  }

  println!(
    "\n{}",
    format!("Results: {}/{} tests passed", passed_tests, total_tests)
      .blue()
      .bold()
  );

  if passed_tests == total_tests {
    println!("{}", "All tests passed! 🎉".green().bold());
  } else {
    println!(
      "{}",
      format!("{} tests failed", total_tests - passed_tests)
        .red()
        .bold()
    );
  }

  Ok(())
}
