use std::collections::HashMap;

use sapling_data_model::Subject;

use crate::instructions::UnificationInstruction;

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
  pub variables: HashMap<String, Subject>,
  pub fact_events: Vec<ExplainFactEvent>,
  pub instruction: Vec<UnificationInstruction>,
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
    ty: EvaluationType,
    outcome: ExplainConstraintEvaluationOutcome,
  },
  EvaluatingSubQuery {
    constraint_id: usize,
    target: Subject,
    target_query: Subject,
    outcome: ExplainConstraintEvaluationOutcome,
  },
  YieldingFact {
    fact_id: usize,
    constraint_id: usize,
    subject_variable: Option<Subject>,
  },
}

#[derive(Debug, PartialEq, Eq)]
pub enum EvaluationType {
  Unification,
  Check,
}

impl ExplainFactEvent {
  pub fn update_outcome(&mut self, new_outcome: ExplainConstraintEvaluationOutcome) {
    match self {
      ExplainFactEvent::EvaluatingExpectedFact { .. } => {
        panic!("Cannot update outcome of an expected fact event");
      }
      ExplainFactEvent::YieldingFact { .. } => {
        panic!("Cannot update outcome of a yielding fact event");
      }
      ExplainFactEvent::EvaluatingSubQuery { outcome, .. } => {
        *outcome = new_outcome;
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
    target: Option<Subject>,
    actual: Subject,
    operator: Subject,
  },
  Property {
    target: Option<Subject>,
    actual: Subject,
    operator: Subject,
  },
  Operator {
    target: Option<Subject>,
    actual: Subject,
    operator: Subject,
  },
  Value {
    target: Option<Subject>,
    actual: Subject,
    operator: Subject,
  },
}
