use std::collections::HashMap;

use sdl2::keyboard::Keycode;

// from yume
#[derive(Clone, Copy)]
pub enum KeyState {
    JustPressed,
    Pressed,
    Released
}

pub struct Input {
    pub keys: HashMap<Keycode, KeyState>
}

#[allow(dead_code)]
impl Input {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new()
        }
    }

    pub fn update(&mut self) {
        for (_, v) in self.keys.iter_mut() {
            if let KeyState::JustPressed = *v {
                *v = KeyState::Pressed;
            }
        }
    }

    /// Notify the input manager that a key has been pressed
    pub fn pressed(&mut self, key: Keycode) {
        self.keys.insert(key, KeyState::JustPressed);
    }

    /// Notify the input manager that a key has been released
    pub fn released(&mut self, key: Keycode) {
        self.keys.insert(key, KeyState::Released);
    }

    /// Returns true if `key` is pressed
    pub fn get_pressed(&self, key: Keycode) -> bool {
        matches!(self.keys.get(&key).unwrap_or(&KeyState::Released), KeyState::Pressed | KeyState::JustPressed)
    }

    /// Returns true if `key` has just been pressed
    pub fn get_just_pressed(&self, key: Keycode) -> bool {
        matches!(self.keys.get(&key).unwrap_or(&KeyState::Released), KeyState::JustPressed)
    }

    /// Returns true if `key` is released
    pub fn get_released(&self, key: Keycode) -> bool {
        matches!(self.keys.get(&key).unwrap_or(&KeyState::Released), KeyState::Released)
    }

    /// Returns the keystate of `key`
    pub fn get_keystate(&self, key: Keycode) -> KeyState {
        *self.keys.get(&key).unwrap_or(&KeyState::Released)
    }
}