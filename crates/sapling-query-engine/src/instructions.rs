use sapling_data_model::Subject;

use crate::machine::VariableBinding;

#[derive(Debug, Clone)]
pub enum UnificationInstruction {
  // Frame instructions
  AllocateFrame {
    size: usize,
  },

  // Yield instructions
  MaybeYield,
  YieldAll,

  // Check instructions
  CheckSubject {
    subject: Subject,
  },
  CheckProperty {
    property: Subject,
  },
  CheckValue {
    value: Subject,
    property: Option<Subject>,
  },
  CheckOperator {
    operator: Subject,
  },
  CheckMeta {
    skip_system: bool,
  },

  // Unifications instructions
  UnifySubject {
    variable: usize,
  },
  UnifyProperty {
    variable: usize,
  },
  UnifyValue {
    variable: usize,
  },

  // Skip instructions
  SkipSubject {
    subject: Subject,
  },

  // Tracing instructions
  TraceBindVariable {
    variable: usize,
    binding: Subject,
  },
  TraceStartFact {
    constraint: usize,
    fact: usize,
  },
  TraceConstraintCreate {
    constraint: usize,
    fact_index: usize,
  },
}
