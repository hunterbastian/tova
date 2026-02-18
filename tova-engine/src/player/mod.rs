use std::collections::HashSet;
use winit::keyboard::KeyCode;

/// Tracks which keys are currently held down.
pub struct Input {
    pressed: HashSet<KeyCode>,
}

impl Input {
    pub fn new() -> Self {
        Self {
            pressed: HashSet::new(),
        }
    }

    pub fn key_down(&mut self, key: KeyCode) {
        self.pressed.insert(key);
    }

    pub fn key_up(&mut self, key: KeyCode) {
        self.pressed.remove(&key);
    }

    pub fn is_pressed(&self, key: KeyCode) -> bool {
        self.pressed.contains(&key)
    }

    pub fn forward(&self) -> bool {
        self.is_pressed(KeyCode::KeyW) || self.is_pressed(KeyCode::ArrowUp)
    }

    pub fn back(&self) -> bool {
        self.is_pressed(KeyCode::KeyS) || self.is_pressed(KeyCode::ArrowDown)
    }

    pub fn left(&self) -> bool {
        self.is_pressed(KeyCode::KeyA) || self.is_pressed(KeyCode::ArrowLeft)
    }

    pub fn right(&self) -> bool {
        self.is_pressed(KeyCode::KeyD) || self.is_pressed(KeyCode::ArrowRight)
    }

    pub fn up(&self) -> bool {
        self.is_pressed(KeyCode::Space)
    }

    pub fn down(&self) -> bool {
        self.is_pressed(KeyCode::ShiftLeft) || self.is_pressed(KeyCode::ShiftRight)
    }
}
