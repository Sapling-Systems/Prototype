use anyhow::{Context, Result};
use clap::Parser as ClapParser;
use colored::*;
use sapling_data_model::{Fact, Query, Subject};
use sapling_query_engine::{
  EvaluationType, ExplainConstraintEvaluationOutcome, FoundFact, QueryEngine,
  SharedVariableAllocator, SharedVariableBank, System,
};
use similar::{ChangeTag, TextDiff};
use std::cell::RefCell;
use std::path::Path;
use std::sync::Arc;
use std::{fs, rc::Rc};

mod parser;
use parser::{SubjectRegistry, TestLine};

#[derive(ClapParser, Debug)]
#[command(name = "spec-validator")]
#[command(about = "Validate Sapling query engine specifications")]
struct Args {
  /// Update test files with actual output when differences are found
  #[arg(short = 'u', long = "update")]
  update: bool,
}

const MEMORY_BANK_SIZE: usize = 128;

fn resolve_fact_references(
  database: &mut sapling_query_engine::Database,
  fact_identifiers: &std::collections::HashMap<String, usize>,
) {
  // Iterate through all facts and resolve @identifier references
  for fact in database.facts_mut() {
    // Helper to resolve a subject if it's a fact reference
    let resolve_subject = |subject: &Subject| -> Subject {
      if let Subject::String { value } = subject {
        if value.starts_with('@') {
          let identifier = &value[1..];
          if let Some(&fact_id) = fact_identifiers.get(identifier) {
            return Subject::Integer {
              value: fact_id as i64,
            };
          }
        }
      }
      subject.clone()
    };

    // Resolve references in subject selector
    fact.subject.subject = resolve_subject(&fact.subject.subject);
    if let Some(property) = &fact.subject.property {
      fact.subject.property = Some(resolve_subject(property));
    }

    // Resolve references in property selector
    fact.property.subject = resolve_subject(&fact.property.subject);
    if let Some(property) = &fact.property.property {
      fact.property.property = Some(resolve_subject(property));
    }

    // Resolve references in operator
    fact.operator = resolve_subject(&fact.operator);

    // Resolve references in value selector
    fact.value.subject = resolve_subject(&fact.value.subject);
    if let Some(property) = &fact.value.property {
      fact.value.property = Some(resolve_subject(property));
    }

    // Resolve references in meta
    fact.meta = resolve_subject(&fact.meta);
  }
}

fn print_diff(expected: &[String], actual: &[String]) {
  let expected_text = expected.join("\n");
  let actual_text = actual.join("\n");

  let diff = TextDiff::from_lines(&expected_text, &actual_text);

  println!("  {}", "Diff:".magenta().bold());

  for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
    if idx > 0 {
      println!("    {}", "---".dimmed());
    }

    for op in group {
      for change in diff.iter_changes(op) {
        let (sign, line) = match change.tag() {
          ChangeTag::Delete => ('-', change.value().trim_end().red()),
          ChangeTag::Insert => ('+', change.value().trim_end().green()),
          ChangeTag::Equal => (' ', change.value().trim_end().normal()),
        };

        println!("    {} {}", sign, line);
      }
    }
  }
}
const FACT_INDEX_OFFSET: usize = 8;

fn format_explain_result(
  engine: &QueryEngine,
  database: &sapling_query_engine::Database,
  result: &sapling_query_engine::ExplainResult,
) -> Vec<String> {
  let mut lines = Vec::new();

  // Format constraints
  for (idx, (constraint_id, fact_id)) in result.constraints.iter().enumerate() {
    // Get the constraint fact from the database
    let fact = database.get_fact(*fact_id);
    if let Some(fact) = fact {
      let fact_str = format_fact(engine, fact);
      lines.push(format!(
        "Constraint{}: {} [{}]",
        idx, constraint_id, fact_str
      ));
    } else {
      lines.push(format!("Constraint{}: {} [unknown]", idx, constraint_id));
    }
  }

  // Format subject
  if let Some(subject) = &result.subject {
    lines.push(format!("Subject: {}", format_subject(engine, subject)));
  }

  // Format fact events
  use sapling_query_engine::ExplainFactEvent;
  for event in &result.fact_events {
    match event {
      ExplainFactEvent::EvaluatingExpectedFact {
        constraint_id,
        fact_id,
      } => {
        let fact = database.get_fact(*fact_id);
        if let Some(fact) = fact {
          let fact_str = format_fact(engine, fact);
          lines.push(format!(
            "Fact{}: {} [{}]",
            constraint_id,
            fact_id - FACT_INDEX_OFFSET,
            fact_str
          ));
        } else {
          lines.push(format!(
            "Fact{}: {} [unknown]",
            constraint_id,
            fact_id - FACT_INDEX_OFFSET
          ));
        }
      }
      ExplainFactEvent::YieldingFact {
        constraint_id,
        fact_id,
        subject_variable,
      } => {
        let fact = database.get_fact(*fact_id);
        if let Some(fact) = fact {
          let fact_str = format_fact(engine, fact);
          lines.push(format!(
            "Yielded for Fact{}: {} [{}]{}",
            constraint_id,
            fact_id - FACT_INDEX_OFFSET,
            fact_str,
            if let Some(subject_variable) = subject_variable {
              format!(" (subject: {})", format_subject(engine, subject_variable))
            } else {
              String::new()
            }
          ));
        } else {
          lines.push(format!(
            "Yielded for Fact{}: {} [unknown]",
            constraint_id,
            fact_id - FACT_INDEX_OFFSET
          ));
        }
      }
      ExplainFactEvent::EvaluatingSubQuery {
        constraint_id,
        target,
        target_query,
        outcome,
      } => {
        let outcome = match outcome {
          ExplainConstraintEvaluationOutcome::Passed => "PASS",
          ExplainConstraintEvaluationOutcome::Rejected(..) => "REJECTED",
        };
        lines.push(format!(
          "Fact{}: Evaluating SubQuery ?{} yields {} => {}",
          constraint_id,
          format_subject(engine, target_query),
          format_subject(engine, target),
          outcome
        ));
      }
      ExplainFactEvent::EvaluatingConstraint {
        constraint_id,
        outcome,
        evaluation,
        ty,
      } => {
        use sapling_query_engine::ExplainConstraintEvaluation;
        match evaluation {
          ExplainConstraintEvaluation::Subject {
            target,
            actual,
            operator,
          } => {
            let outcome = match outcome {
              ExplainConstraintEvaluationOutcome::Passed => "PASS",
              ExplainConstraintEvaluationOutcome::Rejected(..) => "REJECTED",
            };
            lines.push(format!(
              "Fact{}: Subject {} {} {} => {}{}",
              constraint_id,
              format_subject(engine, actual),
              format_subject(engine, operator),
              if let Some(target) = target {
                format_subject(engine, target)
              } else {
                "~unset~".to_string()
              },
              outcome,
              if ty == &EvaluationType::Unification {
                " (unification)"
              } else {
                ""
              }
            ));
          }
          ExplainConstraintEvaluation::Property {
            target,
            actual,
            operator,
          } => {
            let outcome = match outcome {
              ExplainConstraintEvaluationOutcome::Passed => "PASS",
              ExplainConstraintEvaluationOutcome::Rejected(..) => "REJECTED",
            };
            lines.push(format!(
              "Fact{}: Property {} {} {} => {}",
              constraint_id,
              format_subject(engine, actual),
              format_subject(engine, operator),
              if let Some(target) = target {
                format_subject(engine, target)
              } else {
                "~unset~".to_string()
              },
              outcome
            ));
          }
          ExplainConstraintEvaluation::Operator {
            target,
            actual,
            operator,
          } => {
            let outcome = match outcome {
              ExplainConstraintEvaluationOutcome::Passed => "PASS",
              ExplainConstraintEvaluationOutcome::Rejected(..) => "REJECTED",
            };
            lines.push(format!(
              "Fact{}: Operator {} {} {} => {}{}",
              constraint_id,
              format_subject(engine, actual),
              format_subject(engine, operator),
              if let Some(target) = target {
                format_subject(engine, target)
              } else {
                "~unset~".to_string()
              },
              outcome,
              if ty == &EvaluationType::Unification {
                " (unification)"
              } else {
                ""
              }
            ));
          }
          ExplainConstraintEvaluation::Value {
            target,
            actual,
            operator,
          } => {
            let outcome = match outcome {
              ExplainConstraintEvaluationOutcome::Passed => "PASS",
              ExplainConstraintEvaluationOutcome::Rejected(..) => "REJECTED",
            };
            lines.push(format!(
              "Fact{}: Value {} {} {} => {}{}",
              constraint_id,
              format_subject(engine, actual),
              format_subject(engine, operator),
              if let Some(target) = target {
                format_subject(engine, target)
              } else {
                "~unset~".to_string()
              },
              outcome,
              if ty == &EvaluationType::Unification {
                " (unification)"
              } else {
                ""
              }
            ));
          }
        }
      }
    }
  }

  // Format variables
  let mut variables = result.variables.iter().collect::<Vec<_>>();
  variables.sort_by_key(|var| var.0);

  for (variable, value) in variables {
    lines.push(format!(
      "Unification Variable {} = {}",
      variable,
      format_subject(engine, value)
    ));
  }

  lines
}

fn format_subject(engine: &QueryEngine, subject: &Subject) -> String {
  match subject {
    Subject::Static { uuid } => {
      let bank = SharedVariableBank::new(MEMORY_BANK_SIZE);
      let allocator = SharedVariableAllocator::new();

      let name = engine
        .query(
          &Query {
            subject: subject.clone(),
            property: Some(System::CORE_PROPERTY_SUBJECT_NAME),
            meta: Some(System::CORE_META_INCLUDE),
            evaluated: false,
          },
          bank,
          allocator,
        )
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

  let mut value_str = if fact.value.evaluated {
    format!("?{}", format_subject(engine, &fact.value.subject))
  } else {
    format_subject(engine, &fact.value.subject)
  };

  if let Some(value_property) = &fact.value.property {
    value_str += &format!("/{}", format_subject(engine, value_property));
  }

  format!(
    "{}/{} {} {}",
    subject_str,
    property_str,
    format_subject(engine, &fact.operator),
    value_str
  )
}

fn update_test_file(file_path: &Path, old_lines: &[String], new_lines: &[String]) -> Result<()> {
  let content = fs::read_to_string(file_path)
    .with_context(|| format!("Failed to read file: {:?}", file_path))?;

  // Detect line ending style in the file
  let line_ending = if content.contains("\r\n") {
    "\r\n"
  } else {
    "\n"
  };

  // Find and replace the old expected output with the new actual output
  let mut updated_content = content.clone();

  // Build the old expected section (>> prefixed lines)
  let old_section = old_lines
    .iter()
    .map(|line| format!(">> {}", line))
    .collect::<Vec<_>>()
    .join(line_ending);

  // Build the new expected section (>> prefixed lines)
  let new_section = new_lines
    .iter()
    .map(|line| format!(">> {}", line))
    .collect::<Vec<_>>()
    .join(line_ending);

  // Replace the old section with the new one
  if old_section != new_section {
    updated_content = updated_content.replace(&old_section, &new_section);
    fs::write(file_path, updated_content)
      .with_context(|| format!("Failed to write updated file: {:?}", file_path))?;
    println!("  {}", "Updated test file".yellow().bold());
  }

  Ok(())
}

fn update_explain_test_file(
  file_path: &Path,
  old_lines: &[String],
  new_lines: &[String],
) -> Result<()> {
  let content = fs::read_to_string(file_path)
    .with_context(|| format!("Failed to read file: {:?}", file_path))?;

  // Detect line ending style in the file
  let line_ending = if content.contains("\r\n") {
    "\r\n"
  } else {
    "\n"
  };

  // Find and replace the old expected output with the new actual output
  let mut updated_content = content.clone();

  // Build the old expected section (#> prefixed lines)
  let old_section = old_lines
    .iter()
    .map(|line| format!("#> {}", line))
    .collect::<Vec<_>>()
    .join(line_ending);

  // Build the new expected section (#> prefixed lines)
  let new_section = new_lines
    .iter()
    .map(|line| format!("#> {}", line))
    .collect::<Vec<_>>()
    .join(line_ending);

  // Replace the old section with the new one
  if old_section != new_section {
    updated_content = updated_content.replace(&old_section, &new_section);
    fs::write(file_path, updated_content)
      .with_context(|| format!("Failed to write updated file: {:?}", file_path))?;
    println!("  {}", "Updated test file".yellow().bold());
  }

  Ok(())
}

fn run_test(file_path: &Path, update_mode: bool) -> Result<bool> {
  let content = fs::read_to_string(file_path)
    .with_context(|| format!("Failed to read file: {:?}", file_path))?;

  let mut registry = SubjectRegistry::new();
  let test_case = registry
    .parse_test_case(&content)
    .with_context(|| format!("Failed to parse test case: {:?}", file_path))?;

  let (mut database, mut fact_identifiers) = registry.into_database();

  let mut success = true;
  let mut query_count = 0;

  println!("{}", format!("Running test: {:?}", file_path).blue().bold());

  for line in test_case.lines {
    match line {
      TestLine::Fact(fact, fact_identifier) => {
        let fact_id = database.add_fact(fact);
        if let Some(identifier) = fact_identifier {
          fact_identifiers.insert(identifier, fact_id);
        }
      }
      TestLine::Query(query) => {
        // Resolve fact references before running queries
        resolve_fact_references(&mut database, &fact_identifiers);

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

        let bank = SharedVariableBank::new(MEMORY_BANK_SIZE);
        let allocator = SharedVariableAllocator::new();

        let mut machine = engine.query(
          &Query {
            evaluated: query.subject_evaluated,
            meta: None,
            property: query.property.clone(),
            subject: query.subject.clone(),
          },
          bank,
          allocator,
        );
        //println!("Instructions: {:#?}", machine.instructions);
        //machine.log_instructions = true;
        let actual_facts: Vec<FoundFact> = machine.collect();

        println!(
          "  {} ({} facts)",
          "Expected:".yellow(),
          query.expected_facts.len()
        );
        for expected in &query.expected_facts {
          let fact_str = format_fact(&engine, &expected.fact);
          if let Some(subject_mapping) = &expected.subject_mapping {
            println!(
              "    {} ;; subject={}",
              fact_str,
              format_subject(&engine, subject_mapping),
            );
          } else {
            println!("    {}", fact_str);
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

          if update_mode {
            // Build expected output lines from query
            let old_lines: Vec<String> = query
              .expected_facts
              .iter()
              .map(|expected| {
                let fact_str = format_fact(&engine, &expected.fact);
                if let Some(subject_mapping) = &expected.subject_mapping {
                  format!(
                    "{} ;; subject={}",
                    fact_str,
                    format_subject(&engine, subject_mapping)
                  )
                } else {
                  fact_str
                }
              })
              .collect();

            // Build actual output lines
            let new_lines: Vec<String> = actual_facts
              .iter()
              .map(|found_fact| {
                let fact_str = format_fact(&engine, found_fact.fact);
                if let Some(subject_binding) = &found_fact.subject_binding {
                  format!(
                    "{} ;; subject={}",
                    fact_str,
                    format_subject(&engine, subject_binding)
                  )
                } else {
                  fact_str
                }
              })
              .collect();

            update_test_file(file_path, &old_lines, &new_lines)?;
          }
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

            if update_mode {
              // Build expected output lines from query
              let old_lines: Vec<String> = query
                .expected_facts
                .iter()
                .map(|expected| {
                  let fact_str = format_fact(&engine, &expected.fact);
                  if let Some(subject_mapping) = &expected.subject_mapping {
                    format!(
                      "{} ;; subject={}",
                      fact_str,
                      format_subject(&engine, subject_mapping)
                    )
                  } else {
                    fact_str
                  }
                })
                .collect();

              // Build actual output lines
              let new_lines: Vec<String> = actual_facts
                .iter()
                .map(|found_fact| {
                  let fact_str = format_fact(&engine, found_fact.fact);
                  if let Some(subject_binding) = &found_fact.subject_binding {
                    format!(
                      "{} ;; subject={}",
                      fact_str,
                      format_subject(&engine, subject_binding)
                    )
                  } else {
                    fact_str
                  }
                })
                .collect();

              update_test_file(file_path, &old_lines, &new_lines)?;
            }
          }
        }

        println!();
      }
      TestLine::ExplainQuery(_explain_query) => {
        // Explain queries are ignored in regular tests
      }
    }
  }

  Ok(success)
}

fn run_explain_test(file_path: &Path, update_mode: bool) -> Result<bool> {
  let content = fs::read_to_string(file_path)
    .with_context(|| format!("Failed to read file: {:?}", file_path))?;

  let mut registry = SubjectRegistry::new();
  let test_case = registry
    .parse_test_case(&content)
    .with_context(|| format!("Failed to parse test case: {:?}", file_path))?;

  let (mut database, mut fact_identifiers) = registry.into_database();

  let mut success = true;
  let mut explain_count = 0;

  println!(
    "{}",
    format!("Running explain test: {:?}", file_path)
      .blue()
      .bold()
  );

  for line in test_case.lines {
    match line {
      TestLine::Fact(fact, fact_identifier) => {
        let fact_id = database.add_fact(fact);
        if let Some(identifier) = fact_identifier {
          fact_identifiers.insert(identifier, fact_id);
        }
      }
      TestLine::Query(_query) => {
        // Regular queries are ignored in explain tests
      }
      TestLine::ExplainQuery(explain_query) => {
        // Resolve fact references before running explain queries
        resolve_fact_references(&mut database, &fact_identifiers);

        explain_count += 1;
        let engine = QueryEngine::new(Arc::new(database.clone()));

        println!(
          "  {} {} {}",
          "Explain".green().bold(),
          explain_count,
          format_subject(&engine, &explain_query.subject),
        );

        // Call the explain function
        let bank = SharedVariableBank::new(32);
        let allocator = SharedVariableAllocator::new();

        let explain_result = engine.explain(&explain_query.subject, bank, allocator);

        // Format the result into lines
        let actual_lines = format_explain_result(&engine, &database, &explain_result);

        println!(
          "  {} ({} lines)",
          "Expected:".yellow(),
          explain_query.expected_lines.len()
        );
        for line in &explain_query.expected_lines {
          println!("    {}", line);
        }

        println!("  {} ({} lines)", "Actual:".cyan(), actual_lines.len());
        for line in &actual_lines {
          println!("    {}", line);
        }

        // Compare expected vs actual
        if explain_query.expected_lines == actual_lines {
          println!("  {}", "PASS".green().bold());
        } else {
          println!("  {}", "FAIL: Output doesn't match".red().bold());
          println!();
          print_diff(&explain_query.expected_lines, &actual_lines);
          success = false;

          if update_mode {
            update_explain_test_file(file_path, &explain_query.expected_lines, &actual_lines)?;
          }
        }

        println!();
      }
    }
  }

  Ok(success)
}

fn collect_spec_files(
  dir_path: &Path,
) -> Result<(Vec<std::path::PathBuf>, Vec<std::path::PathBuf>)> {
  let mut all_spec_files = Vec::new();
  let mut only_files = Vec::new();

  if !dir_path.exists() {
    return Ok((all_spec_files, only_files));
  }

  for entry in fs::read_dir(dir_path)? {
    let entry = entry?;
    let path = entry.path();

    #[allow(clippy::unnecessary_map_or)]
    if path.is_file() && path.extension().map_or(false, |ext| ext == "txt") {
      let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

      // Check if this is an .only.txt file
      if file_name.ends_with(".only.txt") {
        only_files.push(path.clone());
      }

      all_spec_files.push(path);
    }
  }

  Ok((all_spec_files, only_files))
}

fn run_validation_suite(
  dir_path: &Path,
  test_runner: fn(&Path, bool) -> Result<bool>,
  global_only_files: &[std::path::PathBuf],
  update_mode: bool,
) -> Result<(usize, usize)> {
  let mut total_tests = 0;
  let mut passed_tests = 0;

  if !dir_path.exists() {
    return Ok((0, 0));
  }

  // Collect all .txt files from this directory
  let (all_spec_files, _local_only_files) = collect_spec_files(dir_path)?;

  // Determine which files to run:
  // - If there are global .only.txt files, only run those from this directory
  // - Otherwise, run all files except .skip.txt files
  let files_to_run: Vec<_> = if !global_only_files.is_empty() {
    // Only run files from this directory that are in the global only list
    all_spec_files
      .into_iter()
      .filter(|path| global_only_files.contains(path))
      .collect()
  } else {
    // Run all files except .skip.txt files
    all_spec_files
      .into_iter()
      .filter(|path| {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        !file_name.ends_with(".skip.txt")
      })
      .collect()
  };

  // Run the selected test files
  for path in files_to_run {
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // Skip .skip.txt files (in case they were included via .only.txt logic)
    if file_name.ends_with(".skip.txt") {
      println!("{}", format!("Skipping: {:?}", path).yellow());
      continue;
    }

    total_tests += 1;
    match test_runner(&path, update_mode) {
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

  Ok((total_tests, passed_tests))
}

fn main() -> Result<()> {
  let args = Args::parse();
  let update_mode = args.update;

  if update_mode {
    println!("{}", "\n=== UPDATE MODE ENABLED ===".yellow().bold());
    println!(
      "{}",
      "Test files will be updated with actual output when differences are found.\n".yellow()
    );
  }

  let spec_dir = Path::new("./apps/spec-validator/spec");
  let spec_explain_dir = Path::new("./apps/spec-validator/spec-explain");

  // Collect .only.txt files globally from both directories
  let (_spec_files, spec_only_files) = collect_spec_files(spec_dir)?;
  let (_explain_files, explain_only_files) = collect_spec_files(spec_explain_dir)?;

  let mut global_only_files = Vec::new();
  global_only_files.extend(spec_only_files);
  global_only_files.extend(explain_only_files);

  // Print message if .only.txt files were found
  if !global_only_files.is_empty() {
    println!(
      "{}",
      format!(
        "\nFound {} .only.txt file(s), running only those and skipping all other tests:\n",
        global_only_files.len()
      )
      .yellow()
      .bold()
    );
    for file in &global_only_files {
      println!("  - {:?}", file);
    }
    println!();
  }

  // Run normal spec validation
  println!("\n{}\n", "=== Running Spec Validation ===".blue().bold());
  let (total_spec_tests, passed_spec_tests) =
    run_validation_suite(spec_dir, run_test, &global_only_files, update_mode)?;

  if total_spec_tests == 0 {
    println!("No spec tests found in {:?}", spec_dir);
  } else {
    println!(
      "\n{}",
      format!(
        "Spec Results: {}/{} tests passed",
        passed_spec_tests, total_spec_tests
      )
      .blue()
      .bold()
    );
  }

  // Run explain spec validation
  println!(
    "\n{}\n",
    "=== Running Explain Spec Validation ===".blue().bold()
  );
  let (total_explain_tests, passed_explain_tests) = run_validation_suite(
    spec_explain_dir,
    run_explain_test,
    &global_only_files,
    update_mode,
  )?;

  if total_explain_tests == 0 {
    println!("No explain spec tests found in {:?}", spec_explain_dir);
  } else {
    println!(
      "\n{}",
      format!(
        "Explain Spec Results: {}/{} tests passed",
        passed_explain_tests, total_explain_tests
      )
      .blue()
      .bold()
    );
  }

  // Overall summary
  let total_tests = total_spec_tests + total_explain_tests;
  let passed_tests = passed_spec_tests + passed_explain_tests;

  println!("\n{}", "=== Overall Summary ===".blue().bold());
  println!(
    "{}",
    format!("Total: {}/{} tests passed", passed_tests, total_tests)
      .blue()
      .bold()
  );

  if total_tests > 0 {
    if passed_tests == total_tests {
      println!("{}", "All tests passed!".green().bold());
    } else {
      println!(
        "{}",
        format!("{} tests failed", total_tests - passed_tests)
          .red()
          .bold()
      );
    }
  }

  Ok(())
}
