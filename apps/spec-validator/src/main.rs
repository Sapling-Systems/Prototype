use std::fs;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use colored::*;

use sapling_data_model::{Fact, Query, Subject};
use sapling_query_engine::{QueryEngine, System};

mod parser;
use parser::SubjectRegistry;

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
      if let Some(Subject::String { value }) = name.map(|fact| &fact.value.subject) {
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

  // Add all facts to the database
  for fact in test_case.facts {
    database.add_fact(fact);
  }

  let engine = QueryEngine::new(Arc::new(database));
  let mut success = true;

  println!("{}", format!("Running test: {:?}", file_path).blue().bold());

  for (i, query) in test_case.queries.iter().enumerate() {
    println!(
      "  {} {} {}{}{}",
      "Query".green().bold(),
      i + 1,
      if query.subject_evaluated { "?" } else { "" },
      format_subject(&engine, &query.subject),
      match &query.property {
        Some(subject) => format!("/{}", format_subject(&engine, subject)),
        None => "".to_string(),
      }
    );

    let actual_facts: Vec<&Fact> = engine
      .query(&Query {
        evaluated: query.subject_evaluated,
        meta: None,
        property: query.property.clone(),
        subject: query.subject.clone(),
      })
      .collect();

    println!(
      "  {} ({} facts)",
      "Expected:".yellow(),
      query.expected_facts.len()
    );
    for fact in &query.expected_facts {
      println!("    {}", format_fact(&engine, fact));
    }

    println!("  {} ({} facts)", "Actual:".cyan(), actual_facts.len());
    for fact in &actual_facts {
      println!("    {}", format_fact(&engine, fact));
    }

    // Compare expected vs actual
    if actual_facts.len() != query.expected_facts.len() {
      println!("  {}", "FAIL: Different number of facts".red().bold());
      success = false;
    } else {
      // Simple comparison for now - in a real implementation you'd want more sophisticated matching
      let mut matches = true;
      for expected in &query.expected_facts {
        let found = actual_facts.iter().any(|actual| {
          // Simple structural comparison - this could be improved
          format_fact(&engine, actual) == format_fact(&engine, expected)
        });
        if !found {
          matches = false;
          break;
        }
      }

      if matches {
        println!("  {}", "PASS".green().bold());
      } else {
        println!("  {}", "FAIL: Facts don't match".red().bold());
        success = false;
      }
    }

    println!();
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
