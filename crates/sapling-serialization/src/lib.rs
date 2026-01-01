use sapling_data_model::{Fact, Query, Subject};
use thiserror::Error;

pub trait SaplingSerializable<T: SerializerContext> {
  fn serialize_to_facts(&self, context: &mut T, name: &str) -> Subject;
}

pub trait SerializerContext {
  fn new_static_subject(&mut self, name: &str) -> Subject;
  fn add_fact(&mut self, fact: Fact);
}

macro_rules! impl_serializable_integer {
  ($type:ty) => {
    impl<T: SerializerContext> SaplingSerializable<T> for $type {
      fn serialize_to_facts(&self, _context: &mut T, _name: &str) -> Subject {
        Subject::Integer {
          value: *self as i64,
        }
      }
    }
  };
}

impl_serializable_integer!(i8);
impl_serializable_integer!(i16);
impl_serializable_integer!(i32);
impl_serializable_integer!(i64);
impl_serializable_integer!(u8);
impl_serializable_integer!(u16);
impl_serializable_integer!(u32);
impl_serializable_integer!(u64);

macro_rules! impl_serializable_string {
  ($type:ty) => {
    impl<T: SerializerContext> SaplingSerializable<T> for $type {
      fn serialize_to_facts(&self, _context: &mut T, _name: &str) -> Subject {
        Subject::String {
          value: self.to_string(),
        }
      }
    }
  };
}

impl_serializable_string!(String);
impl_serializable_string!(str);

pub trait SaplingDeserializable<T: DeserializerContext>: Sized {
  fn first_level_queries(subject: &sapling_data_model::Subject, context: &mut T) -> Vec<Query>;
  fn deserialize_subject(subject: &Subject, context: &mut T) -> Result<Self, DeserializeError>;
  fn deserialize_all(context: &mut T) -> Vec<Result<Self, DeserializeError>>;
}

pub trait DeserializerContext {
  fn query<'db, 'q>(&'db mut self, query: &'q Query) -> Vec<&'db Fact>;
  fn get_subject_name(&mut self, subject: &Subject) -> String;
  fn new_static_subject(&mut self, name: &str) -> Subject;
}

#[derive(Error, Debug)]
pub enum DeserializeError {
  #[error("Invalid type expected '{expected}' got '{actual}'")]
  InvalidType { expected: String, actual: String },
  #[error("Property '{property}' is missing for subject '{subject}'")]
  MissingFact { subject: String, property: String },
}

macro_rules! impl_deserializable_integer {
  ($type:ty) => {
    impl<T: DeserializerContext> SaplingDeserializable<T> for $type {
      fn first_level_queries(
        _subject: &sapling_data_model::Subject,
        _context: &mut T,
      ) -> Vec<Query> {
        vec![]
      }

      fn deserialize_subject(
        subject: &Subject,
        _context: &mut T,
      ) -> Result<Self, DeserializeError> {
        match subject {
          Subject::Integer { value } => Ok(*value as $type),
          _ => Err(DeserializeError::InvalidType {
            expected: Subject::Integer { value: 0 }.type_name().to_string(),
            actual: subject.type_name().to_string(),
          }),
        }
      }

      fn deserialize_all(_context: &mut T) -> Vec<Result<Self, DeserializeError>> {
        todo!("not supported on integers")
      }
    }
  };
}

impl_deserializable_integer!(i8);
impl_deserializable_integer!(i16);
impl_deserializable_integer!(i32);
impl_deserializable_integer!(i64);
impl_deserializable_integer!(u8);
impl_deserializable_integer!(u16);
impl_deserializable_integer!(u32);
impl_deserializable_integer!(u64);

macro_rules! impl_deserializable_string {
  ($type:ty) => {
    impl<T: DeserializerContext> SaplingDeserializable<T> for $type {
      fn first_level_queries(
        _subject: &sapling_data_model::Subject,
        _context: &mut T,
      ) -> Vec<Query> {
        vec![]
      }
      fn deserialize_subject(
        subject: &Subject,
        _context: &mut T,
      ) -> Result<Self, DeserializeError> {
        match subject {
          Subject::String { value } => Ok(value.to_string()),
          _ => Err(DeserializeError::InvalidType {
            expected: Subject::String {
              value: String::new(),
            }
            .type_name()
            .to_string(),
            actual: subject.type_name().to_string(),
          }),
        }
      }

      fn deserialize_all(_context: &mut T) -> Vec<Result<Self, DeserializeError>> {
        todo!("not supported on strings")
      }
    }
  };
}

impl_deserializable_string!(String);

pub fn __macro_query_deep<T: DeserializerContext, TOut: SaplingDeserializable<T>>(
  context: &mut T,
  query: &Query,
) -> Result<TOut, DeserializeError> {
  let facts = context.query(query);
  let first_fact =
    facts
      .first()
      .map(|fact| (*fact).clone())
      .ok_or_else(|| DeserializeError::MissingFact {
        subject: context.get_subject_name(&query.subject),
        property: query
          .property
          .as_ref()
          .map(|p| context.get_subject_name(p))
          .unwrap_or_else(|| "-".into()),
      })?;

  if first_fact.value.evaluated || first_fact.value.property.is_some() {
    return __macro_query_deep(
      context,
      &Query {
        evaluated: first_fact.value.evaluated,
        subject: first_fact.value.subject.clone(),
        meta: None,
        property: first_fact.value.property.clone(),
      },
    );
  }

  TOut::deserialize_subject(&first_fact.value.subject, context)
}
