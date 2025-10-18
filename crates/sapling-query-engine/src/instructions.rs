use sapling_data_model::Subject;

#[derive(Debug, Clone)]
pub enum UnificationInstruction {
  AllocateFrame {
    size: usize,
  },

  MaybeYield,
  YieldAll,

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

  UnifySubject {
    variable: usize,
  },
  UnifyProperty {
    variable: usize,
  },
  UnifyValue {
    variable: usize,
  },

  SkipSubject {
    subject: Subject,
  },
}
