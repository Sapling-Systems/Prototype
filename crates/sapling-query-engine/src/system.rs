use std::{
  collections::HashMap,
  sync::{LazyLock, Mutex},
};

use sapling_data_model::{Fact, Subject, SubjectSelector};

use crate::{Database, database::match_subject};

pub struct System;

static CORE_PROPERTY_NAME_MAP: LazyLock<Mutex<HashMap<String, Subject>>> =
  LazyLock::new(|| Mutex::new(HashMap::new()));

impl System {
  pub const CORE_META: Subject = Subject::Static { uuid: 0 };
  pub const CORE_OPERATOR_IS: Subject = Subject::Static { uuid: 1 };
  pub const CORE_OPERATOR_EQ: Subject = Subject::Static { uuid: 2 };
  pub const CORE_PROPERTY_SUBJECT_NAME: Subject = Subject::Static { uuid: 3 };
  pub const CORE_PROPERTY_META_ENTRY: Subject = Subject::Static { uuid: 4 };
  pub const CORE_META_INCLUDE: Subject = Subject::Static { uuid: 5 };
  pub const CORE_WILDCARD_SUBJECT: Subject = Subject::Static { uuid: 6 };
  pub const CORE_INTEGER_PROPERTY: Subject = Subject::Static { uuid: 7 };
  pub const CORE_QUERY_TARGET: Subject = Subject::Static { uuid: 8 };
  pub const CORE_SERIALIZATION_SOURCE: Subject = Subject::Static { uuid: 9 };

  pub(crate) fn install(database: &mut Database) {
    Self::add_core_subject(database, "Core Metadata");
    Self::add_core_subject(database, "=");
    Self::add_core_subject(database, "==");
    Self::add_core_subject(database, "Subject Name");
    Self::add_core_subject(database, "Meta Entry");
    Self::add_core_subject(database, "Meta Core Included");
    Self::add_core_subject(database, "*");
    Self::add_core_subject(database, "SystemIntegerProperty");
    Self::add_core_subject(database, "SystemQueryTarget");
    Self::add_core_subject(database, "SystemSerializationSource");
  }

  pub fn get_named_subject(name: &str) -> Option<Subject> {
    let map = CORE_PROPERTY_NAME_MAP.lock().unwrap();
    map.get(name).cloned()
  }

  pub fn get_subject_name(database: &Database, subject: &Subject) -> Option<String> {
    match subject {
      Subject::Integer { value } => return Some(value.to_string()),
      Subject::Float { value } => return Some(value.to_string()),
      _ => {}
    };

    database
      .raw
      .iter()
      .find(|fact| {
        match_subject(subject, &fact.subject.subject)
          && match_subject(&fact.property.subject, &System::CORE_PROPERTY_SUBJECT_NAME)
      })
      .and_then(|fact| match &fact.value.subject {
        Subject::String { value } => Some(value.clone()),
        _ => None,
      })
  }

  pub fn new_named_static(database: &mut Database, name: &str) -> Subject {
    let subject = database.new_static_subject();
    database.add_fact(Fact {
      subject: SubjectSelector {
        evaluated: false,
        subject: subject.clone(),
        property: None,
      },
      property: SubjectSelector {
        subject: System::CORE_PROPERTY_SUBJECT_NAME.clone(),
        evaluated: false,
        property: None,
      },
      meta: System::CORE_META,
      operator: System::CORE_OPERATOR_IS.clone(),
      value: SubjectSelector {
        subject: Subject::String {
          value: name.to_string(),
        },
        evaluated: false,
        property: None,
      },
    });
    subject
  }

  pub fn get_human_readable_fact(database: &Database, fact: &Fact) -> String {
    format!(
      "{}{}/{} {} {}",
      if fact.subject.evaluated { "?" } else { "" },
      Self::get_subject_name(database, &fact.subject.subject).unwrap_or_default(),
      Self::get_subject_name(database, &fact.property.subject).unwrap_or_default(),
      Self::get_subject_name(database, &fact.operator).unwrap_or_default(),
      match &fact.value.subject {
        Subject::String { value } => value.clone(),
        Subject::Float { value } => value.to_string(),
        Subject::Integer { value } => value.to_string(),
        Subject::Static { .. } =>
          Self::get_subject_name(database, &fact.value.subject).unwrap_or_default(),
        _ => "???".to_string(),
      }
    )
  }

  pub(crate) fn add_core_subject(database: &mut Database, name: &'static str) -> Subject {
    let subject = database.new_static_subject();

    // Name
    database.add_fact(Fact {
      subject: SubjectSelector {
        evaluated: false,
        property: None,
        subject: subject.clone(),
      },
      operator: Self::CORE_OPERATOR_IS.clone(),
      property: SubjectSelector {
        evaluated: false,
        property: None,
        subject: Self::CORE_PROPERTY_SUBJECT_NAME.clone(),
      },
      value: SubjectSelector {
        subject: Subject::String {
          value: name.to_string(),
        },
        evaluated: false,
        property: None,
      },
      meta: Self::CORE_META.clone(),
    });

    // Register
    let mut property_map = CORE_PROPERTY_NAME_MAP.lock().unwrap();
    property_map.insert(name.to_string(), subject.clone());

    subject
  }
}
