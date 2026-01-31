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

#[derive(Clone, Debug)]
pub enum SelectionPathElement {
  Subject,
  Operator,
  Value,
  Property,
  Fact { property: Subject },
}

#[derive(Clone, Debug)]
pub struct SelectionPath {
  path: Vec<SelectionPathElement>,
}

impl Default for SelectionPath {
  fn default() -> Self {
    Self {
      path: vec![SelectionPathElement::Subject],
    }
  }
}

pub enum Selection<'a> {
  SubjectSelector(Option<&'a SubjectSelector>),
  Subject(Option<&'a Subject>),
  Collection(Option<&'a SubjectFactCollection>),
  Fact(Option<&'a SubjectFactCollectionFact>),
}

impl SelectionPath {
  pub fn empty() -> Self {
    Self { path: vec![] }
  }

  pub fn with(&self, element: SelectionPathElement) -> Self {
    let mut path = self.path.clone();
    path.push(element);
    Self { path }
  }

  pub fn popped(&self) -> Self {
    let mut path = self.path.clone();
    path.pop();
    Self { path }
  }

  pub fn matches(&self, other: &Self) -> bool {
    if self.path.len() != other.path.len() {
      return false;
    }
    self
      .path
      .iter()
      .zip(other.path.iter())
      .all(|(a, b)| match (a, b) {
        (SelectionPathElement::Subject, SelectionPathElement::Subject) => true,
        (SelectionPathElement::Operator, SelectionPathElement::Operator) => true,
        (SelectionPathElement::Value, SelectionPathElement::Value) => true,
        (SelectionPathElement::Property, SelectionPathElement::Property) => true,
        (
          SelectionPathElement::Fact {
            property: property1,
          },
          SelectionPathElement::Fact {
            property: property2,
          },
        ) => property1.is_same(property2),
        _ => false,
      })
  }

  fn traverse<'a>(&self, collection: &'a SubjectFactCollection) -> Option<Selection<'a>> {
    enum CurrentItem<'b> {
      Collection(&'b SubjectFactCollection),
      CollectionFact(&'b SubjectFactCollectionFact),
    }

    if self.path.is_empty() {
      return Some(Selection::Collection(Some(collection)));
    }

    let mut current = CurrentItem::Collection(collection);
    for (index, path_item) in self.path.iter().enumerate() {
      let is_last_item = index == self.path.len() - 1;

      match (path_item, &current) {
        (SelectionPathElement::Subject, CurrentItem::Collection(collection)) => {
          return Some(Selection::SubjectSelector(Some(&collection.subject)));
        }
        (SelectionPathElement::Operator, CurrentItem::CollectionFact(fact)) => {
          return Some(Selection::Subject(fact.operator.as_ref()));
        }
        (SelectionPathElement::Property, CurrentItem::CollectionFact(fact)) => {
          return Some(Selection::SubjectSelector(fact.property.as_ref()));
        }
        (SelectionPathElement::Value, CurrentItem::CollectionFact(fact)) => {
          if !is_last_item {
            if let Some(value) = fact.value.as_ref() {
              current = CurrentItem::Collection(&**value);
            }
            continue;
          }
          return Some(Selection::Collection(fact.value.as_ref().map(|val| &**val)));
        }
        (SelectionPathElement::Fact { property }, CurrentItem::Collection(collection)) => {
          let new_fact = collection.facts.iter().find(|fact| {
            if let Some(property_selctor) = &fact.property {
              property_selctor.subject.is_same(property)
            } else {
              false
            }
          });
          if !is_last_item {
            if let Some(new_fact) = new_fact {
              current = CurrentItem::CollectionFact(new_fact);
            }
            continue;
          }

          return Some(Selection::Fact(new_fact));
        }
        _ => return None,
      }
    }

    None
  }

  fn advance_fact(&self, collection: &SubjectFactCollection, negative: bool) -> Self {
    let Some(Selection::Fact(Some(property_fact))) = self.traverse(collection) else {
      return self.clone();
    };
    let Some(property) = &property_fact.property else {
      return self.clone();
    };

    let Some(Selection::Collection(Some(parent_collection))) = self.popped().traverse(collection)
    else {
      return self.clone();
    };

    let Some(current_fact_index) = parent_collection.facts.iter().position(|fact| {
      fact
        .property
        .as_ref()
        .map(|inner_property| inner_property.subject.is_same(&property.subject))
        .unwrap_or(false)
    }) else {
      return self.clone();
    };

    if negative && current_fact_index == 0 {
      return self.clone();
    } else if !negative && current_fact_index == parent_collection.facts.len() - 1 {
      return self.clone();
    }

    let next_fact = if negative {
      parent_collection.facts.get(current_fact_index - 1)
    } else {
      parent_collection.facts.get(current_fact_index + 1)
    };

    if let Some(next_fact) = next_fact {
      if let Some(next_property) = &next_fact
        .property
        .as_ref()
        .map(|property| &property.subject)
      {
        return self.popped().with(SelectionPathElement::Fact {
          property: (*next_property).clone(),
        });
      }
    }

    self.clone()
  }

  pub fn move_to(&self, direction: Direction, collection: &SubjectFactCollection) -> Self {
    let mut path_clone = self.path.clone();
    let last_item = self.path.last().unwrap();

    match (last_item, &direction) {
      (SelectionPathElement::Subject, Direction::Left) => {
        if self.path.len() > 1 {
          path_clone.pop();
          return Self { path: path_clone };
        }
      }
      (SelectionPathElement::Subject, Direction::Right) => {
        if let Some(Selection::Collection(Some(collection))) = self.popped().traverse(collection) {
          if let Some(first_property) = collection
            .facts
            .first()
            .and_then(|fact| fact.property.as_ref())
          {
            path_clone.pop();
            path_clone.push(SelectionPathElement::Fact {
              property: first_property.subject.clone(),
            });
            path_clone.push(SelectionPathElement::Property);
            return Self { path: path_clone };
          }
        }
      }
      (SelectionPathElement::Property, Direction::Left) => {
        path_clone.pop();
        path_clone.pop();
        path_clone.push(SelectionPathElement::Subject);
        return Self { path: path_clone };
      }
      (SelectionPathElement::Property, Direction::Right) => {
        path_clone.pop();
        path_clone.push(SelectionPathElement::Operator);
        return Self { path: path_clone };
      }
      (SelectionPathElement::Operator, Direction::Left) => {
        path_clone.pop();
        path_clone.push(SelectionPathElement::Property);
        return Self { path: path_clone };
      }
      (SelectionPathElement::Operator, Direction::Right) => {
        path_clone.pop();
        path_clone.push(SelectionPathElement::Value);
        return Self { path: path_clone };
      }
      (SelectionPathElement::Value, Direction::Left) => {
        path_clone.pop();
        path_clone.push(SelectionPathElement::Operator);
        return Self { path: path_clone };
      }
      (SelectionPathElement::Value, Direction::Right) => {
        if let Some(Selection::Collection(Some(collection))) = self.traverse(collection) {
          if matches!(collection.subject.subject, Subject::Static { .. })
            && collection.facts.len() >= 1
          {
            return self.with(SelectionPathElement::Subject);
          }
        }
      }
      (SelectionPathElement::Property, Direction::Down) => {
        return self
          .popped()
          .advance_fact(collection, false)
          .with(SelectionPathElement::Property);
      }
      (SelectionPathElement::Property, Direction::Up) => {
        return self
          .popped()
          .advance_fact(collection, true)
          .with(SelectionPathElement::Property);
      }
      (SelectionPathElement::Operator, Direction::Down) => {
        return self
          .popped()
          .advance_fact(collection, false)
          .with(SelectionPathElement::Operator);
      }
      (SelectionPathElement::Operator, Direction::Up) => {
        return self
          .popped()
          .advance_fact(collection, true)
          .with(SelectionPathElement::Operator);
      }
      (SelectionPathElement::Value, Direction::Down) => {
        return self
          .popped()
          .advance_fact(collection, false)
          .with(SelectionPathElement::Value);
      }
      (SelectionPathElement::Value, Direction::Up) => {
        return self
          .popped()
          .advance_fact(collection, true)
          .with(SelectionPathElement::Value);
      }
      _ => {
        return self.clone();
      }
    }

    self.clone()
  }
}

pub enum Direction {
  Left,
  Up,
  Right,
  Down,
}
