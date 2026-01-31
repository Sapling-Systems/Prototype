mod constraint;
mod optimizer;
mod preset;
mod resolver;

pub use constraint::*;
pub use optimizer::*;
pub use preset::*;
pub use resolver::*;

#[derive(Debug, Clone)]
pub struct ResolvedLayout {
  pub width: f32,
  pub height: f32,
  pub x: f32,
  pub y: f32,
}
