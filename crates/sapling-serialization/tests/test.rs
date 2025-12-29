use sapling_data_model::Fact;
use sapling_query_engine::{
  Database, QueryEngine, SharedVariableAllocator, SharedVariableBank, System,
};
use sapling_serialization::{
  DeserializerContext, SaplingDeserializable, SaplingSerializable, SerializerContext,
};
use sapling_serialization_macro::{SaplingDeserialization, SaplingSerialization};

#[test]
fn test_struct_serialization() {
  #[derive(SaplingSerialization, SaplingDeserialization)]
  struct TestStruct {
    a: i64,
    b: i64,
    #[sapling(rename = "c")]
    something: i64,
    #[sapling(rename = "INDEX", indexed = true)]
    indexed: Vec<i64>,
  }

  let test_struct = TestStruct {
    a: 1,
    b: 2,
    something: 3,
    indexed: vec![1, 2, 3],
  };

  let mut database = Database::new();

  struct TestSerializerContext<'a> {
    database: &'a mut Database,
    output: Vec<Fact>,
  }

  impl<'a> SerializerContext for TestSerializerContext<'a> {
    fn new_static_subject(&mut self, name: &str) -> sapling_data_model::Subject {
      System::new_named_static(self.database, name)
    }
    fn add_fact(&mut self, fact: Fact) {
      self.output.push(fact.clone());
      self.database.add_fact(fact);
    }
  }

  let mut context = TestSerializerContext {
    database: &mut database,
    output: Vec::new(),
  };

  let test_name = "test_name";
  let test_subject = test_struct.serialize_to_facts(&mut context, test_name);

  assert_eq!(context.output.len(), 7);
  assert_eq!(
    System::get_human_readable_fact(context.database, &context.output[0]),
    "test_name/SystemSerializationSource = sapling-serialization::test::TestStruct"
  );
  assert_eq!(
    System::get_human_readable_fact(context.database, &context.output[1]),
    "test_name/a = 1"
  );
  assert_eq!(
    System::get_human_readable_fact(context.database, &context.output[2]),
    "test_name/b = 2"
  );
  assert_eq!(
    System::get_human_readable_fact(context.database, &context.output[3]),
    "test_name/c = 3"
  );
  assert_eq!(
    System::get_human_readable_fact(context.database, &context.output[4]),
    "test_name/0 = 1"
  );
  assert_eq!(
    System::get_human_readable_fact(context.database, &context.output[5]),
    "test_name/1 = 2"
  );
  assert_eq!(
    System::get_human_readable_fact(context.database, &context.output[6]),
    "test_name/2 = 3"
  );

  struct TestDeserializerContext {
    database: Database,
  }

  impl DeserializerContext for TestDeserializerContext {
    fn new_static_subject(&mut self, name: &str) -> sapling_data_model::Subject {
      System::new_named_static(&mut self.database, name)
    }
    fn get_subject_name(&mut self, subject: &sapling_data_model::Subject) -> String {
      System::get_subject_name(&self.database, subject).unwrap_or_else(|| "unknown".to_string())
    }
    fn query(&mut self, query: &sapling_data_model::Query) -> Vec<&Fact> {
      let query_engine = QueryEngine::new();
      let bank = SharedVariableBank::new(128);
      let allocator = SharedVariableAllocator::new();
      query_engine
        .query(&self.database, query, bank, allocator)
        .map(|fact| fact.fact)
        .collect::<Vec<_>>()
    }
  }

  let result =
    TestStruct::deserialize_subject(&test_subject, &mut TestDeserializerContext { database })
      .unwrap();

  assert_eq!(result.a, 1);
  assert_eq!(result.b, 2);
  assert_eq!(result.something, 3);
  assert_eq!(result.indexed, vec![1, 2, 3]);
}
