use sapling_app::App;
use sapling_data_model::{Query, Subject, SubjectSelector};
use sapling_query_engine::FoundFact;

#[derive(Debug, Clone)]
pub struct SubjectFactCollection {
  pub subject: SubjectSelector,
  pub facts: Vec<SubjectFactCollectionFact>,
}

#[derive(Debug, Clone)]
pub struct SubjectFactCollectionFact {
  pub property: Option<SubjectSelector>,
  pub operator: Option<Subject>,
  pub value: Option<Box<SubjectFactCollection>>,
}

impl SubjectFactCollection {
  pub fn new(subject: SubjectSelector, app: &App) -> Self {
    let facts = app
      .query_once(&Query {
        evaluated: false,
        meta: None,
        property: None,
        subject: subject.subject.clone(),
      })
      .collect::<Vec<_>>();

    Self {
      subject,
      facts: facts
        .into_iter()
        .map(|fact| SubjectFactCollectionFact::new(&fact, app))
        .collect::<Vec<_>>(),
    }
  }
}

impl SubjectFactCollectionFact {
  pub fn new(fact: &FoundFact, app: &App) -> Self {
    let property = fact.fact.property.clone();
    let operator = fact.fact.operator.clone();
    let value_raw = fact.fact.value.clone();

    let value = SubjectFactCollection::new(value_raw, app);

    SubjectFactCollectionFact {
      property: Some(property),
      operator: Some(operator),
      value: Some(Box::new(value)),
    }
  }
}
