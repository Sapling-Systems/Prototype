use sapling_app::App;
use sapling_data_model::{Fact, Subject, SubjectSelector};
use sapling_query_engine::System;

pub fn insert_demo_data(app: &mut App) {
  let first_name = app.create_named_subject("First Name");
  let last_name = app.create_named_subject("Last Name");
  let best_friend = app.create_named_subject("Best Friend");
  let age = app.create_named_subject("Age");

  let person1 = app.create_named_subject("Person 1");
  let person2 = app.create_named_subject("Person 2");

  app.add_fact(Fact {
    subject: SubjectSelector {
      subject: person1.clone(),
      evaluated: false,
      property: None,
    },
    property: SubjectSelector {
      evaluated: false,
      subject: first_name.clone(),
      property: None,
    },
    value: SubjectSelector {
      subject: Subject::String {
        value: "Rene".into(),
      },
      evaluated: false,
      property: None,
    },
    operator: System::CORE_OPERATOR_IS.clone(),
    meta: Subject::String {
      value: "default".to_string(),
    },
  });

  app.add_fact(Fact {
    subject: SubjectSelector {
      subject: person1.clone(),
      evaluated: false,
      property: None,
    },
    property: SubjectSelector {
      evaluated: false,
      subject: last_name.clone(),
      property: None,
    },
    value: SubjectSelector {
      subject: Subject::String {
        value: "Eichhorn".into(),
      },
      evaluated: false,
      property: None,
    },
    operator: System::CORE_OPERATOR_IS.clone(),
    meta: Subject::String {
      value: "default".to_string(),
    },
  });

  app.add_fact(Fact {
    subject: SubjectSelector {
      subject: person1.clone(),
      evaluated: false,
      property: None,
    },
    property: SubjectSelector {
      evaluated: false,
      subject: best_friend.clone(),
      property: None,
    },
    value: SubjectSelector {
      subject: person2.clone(),
      evaluated: false,
      property: None,
    },
    operator: System::CORE_OPERATOR_IS.clone(),
    meta: Subject::String {
      value: "default".to_string(),
    },
  });

  app.add_fact(Fact {
    subject: SubjectSelector {
      subject: person1.clone(),
      evaluated: false,
      property: None,
    },
    property: SubjectSelector {
      evaluated: false,
      subject: age.clone(),
      property: None,
    },
    value: SubjectSelector {
      subject: Subject::Integer { value: 31 },
      evaluated: false,
      property: None,
    },
    operator: System::CORE_OPERATOR_IS.clone(),
    meta: Subject::String {
      value: "default".to_string(),
    },
  });

  app.add_fact(Fact {
    subject: SubjectSelector {
      subject: person2.clone(),
      evaluated: false,
      property: None,
    },
    property: SubjectSelector {
      evaluated: false,
      subject: first_name.clone(),
      property: None,
    },
    value: SubjectSelector {
      subject: Subject::String {
        value: "John".into(),
      },
      evaluated: false,
      property: None,
    },
    operator: System::CORE_OPERATOR_IS.clone(),
    meta: Subject::String {
      value: "default".to_string(),
    },
  });

  app.add_fact(Fact {
    subject: SubjectSelector {
      subject: person2.clone(),
      evaluated: false,
      property: None,
    },
    property: SubjectSelector {
      evaluated: false,
      subject: last_name.clone(),
      property: None,
    },
    value: SubjectSelector {
      subject: Subject::String {
        value: "Doe".into(),
      },
      evaluated: false,
      property: None,
    },
    operator: System::CORE_OPERATOR_IS.clone(),
    meta: Subject::String {
      value: "default".to_string(),
    },
  });
}
