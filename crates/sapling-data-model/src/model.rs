#[derive(Clone, Debug)]
pub enum Subject {
  Static { uuid: u128 },
  Integer { value: i64 },
  Float { value: f64 },
  String { value: String },
}

impl Subject {
  pub fn type_name(&self) -> &'static str {
    match self {
      Subject::Static { .. } => "static",
      Subject::Integer { .. } => "integer",
      Subject::Float { .. } => "float",
      Subject::String { .. } => "string",
    }
  }

  pub fn is_same(&self, other: &Subject) -> bool {
    match (self, other) {
      (Subject::Static { uuid: uuid1 }, Subject::Static { uuid: uuid2 }) => uuid1 == uuid2,
      (Subject::Integer { value: value1 }, Subject::Integer { value: value2 }) => value1 == value2,
      (Subject::Float { value: value1 }, Subject::Float { value: value2 }) => value1 == value2,
      (Subject::String { value: value1 }, Subject::String { value: value2 }) => value1 == value2,
      _ => false,
    }
  }
}

#[derive(Clone, Debug)]
pub struct SubjectSelector {
  /// The target subject
  pub subject: Subject,
  /// Whether we are looking for the subject itself or for it's evaluated results.
  pub evaluated: bool,
  /// An optional property to further narrow down the subject.
  pub property: Option<Subject>,
}

#[derive(Clone, Debug)]
pub struct Fact {
  pub subject: SubjectSelector,
  pub property: SubjectSelector,
  pub operator: Subject,
  pub value: SubjectSelector,
  pub meta: Subject,
}

#[derive(Clone, Debug)]
pub struct Query {
  pub subject: Subject,
  pub property: Option<Subject>,
  pub meta: Option<Subject>,
  pub evaluated: bool,
}
