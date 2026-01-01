use sapling_serialization_macro::{SaplingDeserialization, SaplingSerialization};

#[derive(SaplingSerialization, SaplingDeserialization)]
pub struct Operations {
  #[sapling(indexed = true)]
  indexed: Vec<i64>,
}

pub fn std_math_operation_add(input: &Operations) -> i64 {
  input.indexed.iter().sum()
}
