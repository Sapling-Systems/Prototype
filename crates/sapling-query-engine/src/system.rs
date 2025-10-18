use sapling_data_model::{Fact, Subject, SubjectSelector};

use crate::{Database, database::match_subject};

pub struct System;

impl System {
  pub const CORE_META: Subject = Subject::Static { uuid: 0 };
  pub const CORE_OPERATOR_IS: Subject = Subject::Static { uuid: 1 };
  pub const CORE_OPERATOR_EQ: Subject = Subject::Static { uuid: 2 };
  pub const CORE_PROPERTY_SUBJECT_NAME: Subject = Subject::Static { uuid: 3 };
  pub const CORE_PROPERTY_META_ENTRY: Subject = Subject::Static { uuid: 4 };
  pub const CORE_META_INCLUDE: Subject = Subject::Static { uuid: 5 };
  pub const CORE_WILDCARD_SUBJECT: Subject = Subject::Static { uuid: 6 };

  pub(crate) fn install(database: &mut Database) {
    Self::add_core_subject(database, "Core Metadata");
    Self::add_core_subject(database, "=");
    Self::add_core_subject(database, "==");
    Self::add_core_subject(database, "Subject Name");
    Self::add_core_subject(database, "Meta Entry");
    Self::add_core_subject(database, "Meta Core Included");
    Self::add_core_subject(database, "*");
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

    subject
  }
}
