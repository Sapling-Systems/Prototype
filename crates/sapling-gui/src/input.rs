use raylib::{RaylibHandle, math::Vector2};

#[derive(Default)]
pub struct InputState {
  pub mouse_press: Option<Vector2>,
}

impl InputState {
  pub fn from_raylib(handle: &mut RaylibHandle) -> Self {
    let mut state = Self::default();

    if handle.is_mouse_button_released(raylib::ffi::MouseButton::MOUSE_BUTTON_LEFT) {
      state.mouse_press = Some(handle.get_mouse_position());
    }

    state
  }
}
