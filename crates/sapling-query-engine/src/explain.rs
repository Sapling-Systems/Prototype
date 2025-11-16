use std::collections::HashMap;

use sapling_data_model::Subject;

#[derive(Debug)]
pub struct ExplainQuery {
  pub query_subject: Subject,
  pub target_subject: Option<Subject>,
  pub facts: HashMap<usize, usize>,
}

#[derive(Debug)]
pub struct ExplainResult {
  pub constraints: Vec<(usize, usize)>,
  pub subject: Option<Subject>,
  pub fact_events: Vec<ExplainFactEvent>,
}

#[derive(Debug)]
pub enum ExplainFactEvent {
  EvaluatingExpectedFact {
    constraint_id: usize,
    fact_id: usize,
  },
  EvaluatingConstraint {
    constraint_id: usize,
    evaluation: ExplainConstraintEvaluation,
    outcome: ExplainConstraintEvaluationOutcome,
  },
}

impl ExplainFactEvent {
  pub fn update_outcome(&mut self, new_outcome: ExplainConstraintEvaluationOutcome) {
    match self {
      ExplainFactEvent::EvaluatingExpectedFact { .. } => {
        panic!("Cannot update outcome of an expected fact event");
      }
      ExplainFactEvent::EvaluatingConstraint { outcome, .. } => {
        *outcome = new_outcome;
      }
    }
  }
}

#[derive(Debug)]
pub enum ExplainConstraintEvaluationOutcome {
  Passed,
  Rejected(ExplainConstraintEvaluationOutcomeReason),
}

#[derive(Debug)]
pub enum ExplainConstraintEvaluationOutcomeReason {
  NotFound,
}

#[derive(Debug)]
pub enum ExplainConstraintEvaluation {
  Subject {
    target: Subject,
    actual: Subject,
    operator: Subject,
  },
  Property {
    target: Subject,
    actual: Subject,
    operator: Subject,
  },
  Operator {
    target: Subject,
    actual: Subject,
    operator: Subject,
  },
  Value {
    target: Subject,
    actual: Subject,
    operator: Subject,
  },
}
