pub struct QueryMeta {
  pub include_system_meta: bool,
}

impl Default for QueryMeta {
  fn default() -> Self {
    Self {
      include_system_meta: false,
    }
  }
}
