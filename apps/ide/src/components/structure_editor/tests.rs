#[cfg(test)]
mod tests {
  use sapling_app::App;
  use sapling_data_model::{Fact, Subject, SubjectSelector};
  use sapling_query_engine::System;

  use crate::components::structure_editor::{
    SelectionPathItem, SelectionType, StructureEditorMode, data::SubjectFactCollection,
    move_horizontal, move_vertical,
  };

  struct TestData {
    app: App,
    first_name: Subject,
    last_name: Subject,
    best_friend: Subject,
    age: Subject,
    person1: Subject,
    person2: Subject,
  }

  // Helper function to create a test app with nested data
  fn create_test_data() -> TestData {
    let mut app = App::new(128);

    let first_name = app.create_named_subject("First Name");
    let last_name = app.create_named_subject("Last Name");
    let best_friend = app.create_named_subject("Best Friend");
    let age = app.create_named_subject("Age");

    let person1 = app.create_named_subject("Person 1");
    let person2 = app.create_named_subject("Person 2");

    // Person 1: First Name = "Rene"
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

    // Person 1: Last Name = "Eichhorn"
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

    // Person 1: Best Friend = Person 2 (nested structure)
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

    // Person 1: Age = 31
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

    // Person 2: First Name = "John"
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

    // Person 2: Last Name = "Doe"
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

    TestData {
      app,
      first_name,
      last_name,
      best_friend,
      age,
      person1,
      person2,
    }
  }

  // Helper to match selection state
  fn assert_selection(
    mode: &StructureEditorMode,
    expected_type: SelectionType,
    expected_path_len: usize,
  ) {
    match mode {
      StructureEditorMode::Select {
        selection_type,
        selection_path,
      } => {
        assert_eq!(*selection_type, expected_type);
        assert_eq!(selection_path.len(), expected_path_len);
      }
      _ => panic!("Expected Select mode"),
    }
  }

  #[test]
  fn test_horizontal_subject_to_property() {
    let data = create_test_data();

    let collection = SubjectFactCollection::new(
      SubjectSelector {
        subject: data.person1,
        evaluated: false,
        property: None,
      },
      &data.app,
    );

    // Start at subject, move right to property
    let path = vec![];
    let new_mode = move_horizontal(&collection, &path, SelectionType::Subject, false);

    // Should move to first property (First Name) and extend path
    assert_selection(&new_mode, SelectionType::Property, 1);
  }

  #[test]
  fn test_horizontal_property_to_operator() {
    let data = create_test_data();

    let collection = SubjectFactCollection::new(
      SubjectSelector {
        subject: data.person1,
        evaluated: false,
        property: None,
      },
      &data.app,
    );

    // Start at property, move right to operator
    let path = vec![SelectionPathItem {
      subject: data.first_name,
    }];
    let new_mode = move_horizontal(&collection, &path, SelectionType::Property, false);

    assert_selection(&new_mode, SelectionType::Operator, 1);
  }

  #[test]
  fn test_horizontal_operator_to_value() {
    let data = create_test_data();

    let collection = SubjectFactCollection::new(
      SubjectSelector {
        subject: data.person1,
        evaluated: false,
        property: None,
      },
      &data.app,
    );

    // Start at operator, move right to value
    let path = vec![SelectionPathItem {
      subject: data.first_name,
    }];
    let new_mode = move_horizontal(&collection, &path, SelectionType::Operator, false);

    assert_selection(&new_mode, SelectionType::Value, 1);
  }

  #[test]
  fn test_horizontal_value_primitive_stays() {
    let data = create_test_data();

    let collection = SubjectFactCollection::new(
      SubjectSelector {
        subject: data.person1,
        evaluated: false,
        property: None,
      },
      &data.app,
    );

    // Start at value (primitive string), move right
    let path = vec![SelectionPathItem {
      subject: data.first_name,
    }];
    let new_mode = move_horizontal(&collection, &path, SelectionType::Value, false);

    // Should stay at value since it's a primitive
    assert_selection(&new_mode, SelectionType::Value, 1);
  }

  #[test]
  fn test_horizontal_value_nested_to_subject() {
    let data = create_test_data();

    let collection = SubjectFactCollection::new(
      SubjectSelector {
        subject: data.person1,
        evaluated: false,
        property: None,
      },
      &data.app,
    );

    // Find a nested value (not a primitive) in the collection
    let nested_property = collection
      .facts
      .iter()
      .find(|f| {
        if let Some(value) = &f.value {
          matches!(value.subject.subject, Subject::Static { .. })
        } else {
          false
        }
      })
      .and_then(|f| f.property.as_ref())
      .unwrap()
      .subject
      .clone();

    // Start at value (nested), move right
    let path = vec![SelectionPathItem {
      subject: nested_property,
    }];
    let new_mode = move_horizontal(&collection, &path, SelectionType::Value, false);

    // Should move to subject of nested structure
    assert_selection(&new_mode, SelectionType::Subject, 1);
  }

  #[test]
  fn test_horizontal_left_value_to_operator() {
    let data = create_test_data();

    let collection = SubjectFactCollection::new(
      SubjectSelector {
        subject: data.person1,
        evaluated: false,
        property: None,
      },
      &data.app,
    );

    // Start at value, move left to operator
    let path = vec![SelectionPathItem {
      subject: data.first_name,
    }];
    let new_mode = move_horizontal(&collection, &path, SelectionType::Value, true);

    assert_selection(&new_mode, SelectionType::Operator, 1);
  }

  #[test]
  fn test_horizontal_left_operator_to_property() {
    let data = create_test_data();

    let collection = SubjectFactCollection::new(
      SubjectSelector {
        subject: data.person1,
        evaluated: false,
        property: None,
      },
      &data.app,
    );

    // Start at operator, move left to property
    let path = vec![SelectionPathItem {
      subject: data.first_name,
    }];
    let new_mode = move_horizontal(&collection, &path, SelectionType::Operator, true);

    assert_selection(&new_mode, SelectionType::Property, 1);
  }

  #[test]
  fn test_horizontal_left_property_to_subject() {
    let data = create_test_data();

    let collection = SubjectFactCollection::new(
      SubjectSelector {
        subject: data.person1,
        evaluated: false,
        property: None,
      },
      &data.app,
    );

    // Start at property, move left to subject
    let path = vec![SelectionPathItem {
      subject: data.first_name,
    }];
    let new_mode = move_horizontal(&collection, &path, SelectionType::Property, true);

    // Should pop path and go to parent subject
    assert_selection(&new_mode, SelectionType::Subject, 0);
  }

  #[test]
  fn test_vertical_property_down() {
    let data = create_test_data();

    let collection = SubjectFactCollection::new(
      SubjectSelector {
        subject: data.person1,
        evaluated: false,
        property: None,
      },
      &data.app,
    );

    // Get the first fact's property to start from
    let first_property = collection.facts[0]
      .property
      .as_ref()
      .unwrap()
      .subject
      .clone();

    // Start at first property, move down
    let path = vec![SelectionPathItem {
      subject: first_property.clone(),
    }];
    let new_mode = move_vertical(&collection, &path, SelectionType::Property, true);

    // Should move to second property
    match new_mode {
      StructureEditorMode::Select {
        selection_type,
        selection_path,
      } => {
        assert_eq!(selection_type, SelectionType::Property);
        assert_eq!(selection_path.len(), 1);
        // Should have moved to a different property
        assert!(!selection_path[0].subject.is_same(&first_property));
      }
      _ => panic!("Expected Select mode"),
    }
  }

  #[test]
  fn test_vertical_property_up() {
    let data = create_test_data();

    let collection = SubjectFactCollection::new(
      SubjectSelector {
        subject: data.person1,
        evaluated: false,
        property: None,
      },
      &data.app,
    );

    // Get the second fact's property to start from
    let second_property = collection.facts[1]
      .property
      .as_ref()
      .unwrap()
      .subject
      .clone();
    let first_property = collection.facts[0]
      .property
      .as_ref()
      .unwrap()
      .subject
      .clone();

    // Start at second property, move up
    let path = vec![SelectionPathItem {
      subject: second_property.clone(),
    }];
    let new_mode = move_vertical(&collection, &path, SelectionType::Property, false);

    // Should move to first property
    match new_mode {
      StructureEditorMode::Select {
        selection_type,
        selection_path,
      } => {
        assert_eq!(selection_type, SelectionType::Property);
        assert_eq!(selection_path.len(), 1);
        assert!(selection_path[0].subject.is_same(&first_property));
      }
      _ => panic!("Expected Select mode"),
    }
  }

  #[test]
  fn test_vertical_operator_down() {
    let data = create_test_data();

    let collection = SubjectFactCollection::new(
      SubjectSelector {
        subject: data.person1,
        evaluated: false,
        property: None,
      },
      &data.app,
    );

    // Get the first fact's property to start from
    let first_property = collection.facts[0]
      .property
      .as_ref()
      .unwrap()
      .subject
      .clone();

    // Start at first operator, move down
    let path = vec![SelectionPathItem {
      subject: first_property.clone(),
    }];
    let new_mode = move_vertical(&collection, &path, SelectionType::Operator, true);

    // Should move to second fact's operator
    match new_mode {
      StructureEditorMode::Select {
        selection_type,
        selection_path,
      } => {
        assert_eq!(selection_type, SelectionType::Operator);
        assert_eq!(selection_path.len(), 1);
        // Should have moved to a different property
        assert!(!selection_path[0].subject.is_same(&first_property));
      }
      _ => panic!("Expected Select mode"),
    }
  }

  #[test]
  fn test_vertical_value_primitive_down() {
    let data = create_test_data();

    let collection = SubjectFactCollection::new(
      SubjectSelector {
        subject: data.person1,
        evaluated: false,
        property: None,
      },
      &data.app,
    );

    // Get the first fact's property to start from
    let first_property = collection.facts[0]
      .property
      .as_ref()
      .unwrap()
      .subject
      .clone();

    // Start at first value, move down
    let path = vec![SelectionPathItem {
      subject: first_property.clone(),
    }];
    let new_mode = move_vertical(&collection, &path, SelectionType::Value, true);

    // Should move to next sibling value if there is one
    match new_mode {
      StructureEditorMode::Select {
        selection_type,
        selection_path,
      } => {
        assert_eq!(selection_type, SelectionType::Value);
        assert_eq!(selection_path.len(), 1);
        // Should have moved to a different property
        assert!(!selection_path[0].subject.is_same(&first_property));
      }
      _ => panic!("Expected Select mode"),
    }
  }

  #[test]
  fn test_vertical_value_nested_down() {
    let data = create_test_data();

    let collection = SubjectFactCollection::new(
      SubjectSelector {
        subject: data.person1,
        evaluated: false,
        property: None,
      },
      &data.app,
    );

    // Find a nested value (not a primitive) in the collection
    let nested_property = collection
      .facts
      .iter()
      .find(|f| {
        if let Some(value) = &f.value {
          matches!(value.subject.subject, Subject::Static { .. }) && !value.facts.is_empty()
        } else {
          false
        }
      })
      .and_then(|f| f.property.as_ref())
      .unwrap()
      .subject
      .clone();

    // Start at nested value, move down
    let path = vec![SelectionPathItem {
      subject: nested_property.clone(),
    }];
    let new_mode = move_vertical(&collection, &path, SelectionType::Value, true);

    // Should move into nested structure to first property
    match new_mode {
      StructureEditorMode::Select {
        selection_type,
        selection_path,
      } => {
        assert_eq!(selection_type, SelectionType::Property);
        assert_eq!(selection_path.len(), 2);
        // First path item should be the nested property
        assert!(selection_path[0].subject.is_same(&nested_property));
      }
      _ => panic!("Expected Select mode"),
    }
  }

  #[test]
  fn test_vertical_value_nested_up() {
    let data = create_test_data();

    let collection = SubjectFactCollection::new(
      SubjectSelector {
        subject: data.person1,
        evaluated: false,
        property: None,
      },
      &data.app,
    );

    // Find a nested value (not a primitive) in the collection
    let nested_property = collection
      .facts
      .iter()
      .find(|f| {
        if let Some(value) = &f.value {
          matches!(value.subject.subject, Subject::Static { .. })
        } else {
          false
        }
      })
      .and_then(|f| f.property.as_ref())
      .unwrap()
      .subject
      .clone();

    // Start at nested value, move up
    let path = vec![SelectionPathItem {
      subject: nested_property.clone(),
    }];
    let new_mode = move_vertical(&collection, &path, SelectionType::Value, false);

    // Should move to previous sibling value or stay at current
    match new_mode {
      StructureEditorMode::Select {
        selection_type,
        selection_path,
      } => {
        assert_eq!(selection_type, SelectionType::Value);
        assert_eq!(selection_path.len(), 1);
        // Either moved to a different property or stayed at current (if first)
      }
      _ => panic!("Expected Select mode"),
    }
  }

  #[test]
  fn test_vertical_property_at_boundary_stays() {
    let data = create_test_data();

    let collection = SubjectFactCollection::new(
      SubjectSelector {
        subject: data.person1,
        evaluated: false,
        property: None,
      },
      &data.app,
    );

    // Start at first property, move up (should stay)
    let first_property = collection.facts[0]
      .property
      .as_ref()
      .unwrap()
      .subject
      .clone();
    let path = vec![SelectionPathItem {
      subject: first_property.clone(),
    }];
    let new_mode = move_vertical(&collection, &path, SelectionType::Property, false);

    match new_mode {
      StructureEditorMode::Select {
        selection_type,
        selection_path,
      } => {
        assert_eq!(selection_type, SelectionType::Property);
        assert_eq!(selection_path.len(), 1);
        assert!(selection_path[0].subject.is_same(&first_property));
      }
      _ => panic!("Expected Select mode"),
    }

    // Start at last property, move down (should stay)
    let last_property = collection
      .facts
      .last()
      .unwrap()
      .property
      .as_ref()
      .unwrap()
      .subject
      .clone();
    let path = vec![SelectionPathItem {
      subject: last_property.clone(),
    }];
    let new_mode = move_vertical(&collection, &path, SelectionType::Property, true);

    match new_mode {
      StructureEditorMode::Select {
        selection_type,
        selection_path,
      } => {
        assert_eq!(selection_type, SelectionType::Property);
        assert_eq!(selection_path.len(), 1);
        assert!(selection_path[0].subject.is_same(&last_property));
      }
      _ => panic!("Expected Select mode"),
    }
  }

  #[test]
  fn test_vertical_subject_stays() {
    let data = create_test_data();

    let collection = SubjectFactCollection::new(
      SubjectSelector {
        subject: data.person1,
        evaluated: false,
        property: None,
      },
      &data.app,
    );

    // Subject should not move vertically
    let path = vec![];
    let new_mode_down = move_vertical(&collection, &path, SelectionType::Subject, true);
    assert_selection(&new_mode_down, SelectionType::Subject, 0);

    let new_mode_up = move_vertical(&collection, &path, SelectionType::Subject, false);
    assert_selection(&new_mode_up, SelectionType::Subject, 0);
  }

  #[test]
  fn test_complex_navigation_path() {
    let data = create_test_data();

    let collection = SubjectFactCollection::new(
      SubjectSelector {
        subject: data.person1,
        evaluated: false,
        property: None,
      },
      &data.app,
    );

    // Test complex navigation path
    // Start at subject -> right to property
    let path = vec![];
    let mode = move_horizontal(&collection, &path, SelectionType::Subject, false);
    assert_selection(&mode, SelectionType::Property, 1);

    // Get first property
    let first_property = collection.facts[0]
      .property
      .as_ref()
      .unwrap()
      .subject
      .clone();

    // Move right to operator
    let path = vec![SelectionPathItem {
      subject: first_property.clone(),
    }];
    let mode = move_horizontal(&collection, &path, SelectionType::Property, false);
    assert_selection(&mode, SelectionType::Operator, 1);

    // Move right to value
    let mode = move_horizontal(&collection, &path, SelectionType::Operator, false);
    assert_selection(&mode, SelectionType::Value, 1);

    // Move left back to operator
    let mode = move_horizontal(&collection, &path, SelectionType::Value, true);
    assert_selection(&mode, SelectionType::Operator, 1);

    // Move left to property
    let mode = move_horizontal(&collection, &path, SelectionType::Operator, true);
    assert_selection(&mode, SelectionType::Property, 1);

    // Move down to next property (if exists)
    let mode = move_vertical(&collection, &path, SelectionType::Property, true);
    match mode {
      StructureEditorMode::Select {
        selection_type,
        selection_path,
      } => {
        assert_eq!(selection_type, SelectionType::Property);
        assert_eq!(selection_path.len(), 1);
        // Should have moved to a different property (or stayed if only one fact)
      }
      _ => panic!("Expected Select mode"),
    }
  }
}
