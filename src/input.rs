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
    pub keys: HashMap<Keycode, KeyState>,
    pub left_mouse: KeyState,
    pub right_mouse: KeyState,
    pub middle_mouse: KeyState,
    pub mouse_pos: (u32, u32),
    pub mouse_delta: (i32, i32)
}

#[allow(dead_code)]
impl Input {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            left_mouse: KeyState::Released,
            middle_mouse: KeyState::Released,
            right_mouse: KeyState::Released,
            mouse_pos: (0, 0),
            mouse_delta: (0, 0)
        }
    }

    pub fn update(&mut self) {
        for (_, v) in self.keys.iter_mut() {
            if let KeyState::JustPressed = *v {
                *v = KeyState::Pressed;
            }
        }

        if let KeyState::JustPressed = self.left_mouse { self.left_mouse = KeyState::Pressed; }
        if let KeyState::JustPressed = self.right_mouse { self.right_mouse = KeyState::Pressed; }
        if let KeyState::JustPressed = self.middle_mouse { self.middle_mouse = KeyState::Pressed; }
    }

    /// Notify the input manager that a key has been pressed
    pub fn pressed(&mut self, key: Keycode) {
        self.keys.insert(key, KeyState::JustPressed);
    }

    /// Notify the input manager that a key has been released
    pub fn released(&mut self, key: Keycode) {
        self.keys.insert(key, KeyState::Released);
    }

    pub fn left_mouse_pressed(&mut self) {
        self.left_mouse = KeyState::JustPressed;
    }

    pub fn right_mouse_pressed(&mut self) {
        self.right_mouse = KeyState::JustPressed;
    }

    pub fn middle_mouse_pressed(&mut self) {
        self.middle_mouse = KeyState::JustPressed;
    }

    pub fn left_mouse_released(&mut self) {
        self.left_mouse = KeyState::Released;
    }

    pub fn right_mouse_released(&mut self) {
        self.right_mouse = KeyState::Released;
    }

    pub fn middle_mouse_released(&mut self) {
        self.middle_mouse = KeyState::Released;
    }

    // TODO: Scrolling

    pub fn mouse_moved(&mut self, new_pos: (u32, u32), rel: (i32, i32)) {
        self.mouse_pos = new_pos;
        self.mouse_delta = rel;
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

    pub fn get_left_mouse_just_pressed(&self) -> bool { matches!(self.left_mouse, KeyState::JustPressed) }
    pub fn get_left_mouse_pressed(&self) -> bool { matches!(self.left_mouse, KeyState::JustPressed | KeyState::Pressed) }
    pub fn get_left_mouse_released(&self) -> bool { matches!(self.left_mouse, KeyState::Released) }
    pub fn get_right_mouse_just_pressed(&self) -> bool { matches!(self.right_mouse, KeyState::JustPressed) }
    pub fn get_right_mouse_pressed(&self) -> bool { matches!(self.right_mouse, KeyState::JustPressed | KeyState::Pressed) }
    pub fn get_right_mouse_released(&self) -> bool { matches!(self.right_mouse, KeyState::Released) }
    pub fn get_middle_mouse_just_pressed(&self) -> bool { matches!(self.middle_mouse, KeyState::JustPressed) }
    pub fn get_middle_mouse_pressed(&self) -> bool { matches!(self.middle_mouse, KeyState::JustPressed | KeyState::Pressed) }
    pub fn get_middle_mouse_released(&self) -> bool { matches!(self.middle_mouse, KeyState::Released) }
}