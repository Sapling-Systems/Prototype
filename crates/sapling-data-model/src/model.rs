#[derive(Clone, Debug)]
pub enum Subject {
  Static { uuid: u128 },
  Integer { value: i64 },
  Float { value: f64 },
  String { value: String },
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
