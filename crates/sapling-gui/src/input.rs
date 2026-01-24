use std::{
  collections::HashMap,
  hash::{DefaultHasher, Hash, Hasher},
};

use raylib::{RaylibHandle, ffi::KeyboardKey, math::Vector2};

#[derive(Default)]
pub struct InputState {
  pub mouse_press: Option<Vector2>,
  action_map: ActionMap,
  key_presses: HashMap<u64, bool>,
}

impl InputState {
  pub fn from_raylib(handle: &mut RaylibHandle, action_map: ActionMap) -> Self {
    let mut state = Self::default();

    if handle.is_mouse_button_released(raylib::ffi::MouseButton::MOUSE_BUTTON_LEFT) {
      state.mouse_press = Some(handle.get_mouse_position());
    }

    for (hash, key) in &action_map.keys {
      state.key_presses.insert(*hash, handle.is_key_pressed(*key));
    }

    state.action_map = action_map;
    state
  }

  pub fn is_action_pressed(&self, action: u64) -> bool {
    *self.key_presses.get(&action).unwrap_or(&false)
  }
}

#[derive(Default, Clone)]
pub struct ActionMap {
  keys: HashMap<u64, KeyboardKey>,
}

impl ActionMap {
  pub fn new() -> Self {
    ActionMap {
      keys: HashMap::new(),
    }
  }

  pub fn add_action(&mut self, action: impl Hash, key: KeyboardKey) {
    let mut hasher = DefaultHasher::new();
    action.hash(&mut hasher);
    let hash = hasher.finish();

    self.keys.insert(hash, key);
  }
}
