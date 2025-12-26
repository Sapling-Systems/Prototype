use sapling_data_model::Subject;

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
  CheckPropertyConstAnyInteger,
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

  // Variable instructions
  BindVariable {
    variable: usize,
    binding: Subject,
  },

  // Tracing instructions
  TraceStartFact {
    constraint: usize,
    fact: usize,
  },
  TraceConstraintCreate {
    constraint: usize,
    fact_index: usize,
  },
  TraceSubQuery {
    query: Subject,
    variable: usize,
  },
  TraceLogYield {
    constraint: usize,
    fact_index: usize,
  },

  // Debug instructions
  DebugComment {
    comment: String,
  },
}
