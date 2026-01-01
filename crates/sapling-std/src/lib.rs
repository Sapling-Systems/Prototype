use sapling_app::{AppPlugin, AppPluginInstallContext};

use crate::math::std_math_operation_add;

mod math;

#[derive(Default)]
pub struct StandardLibrary;

impl AppPlugin for StandardLibrary {
  fn install_plugin(&mut self, context: &mut AppPluginInstallContext) {
    context.add_interop_fn("MathSum", "Result", std_math_operation_add);
  }
}
