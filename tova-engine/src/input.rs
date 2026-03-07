use std::collections::HashSet;

use winit::keyboard::KeyCode;

use crate::camera::MoveIntent;

#[derive(Default)]
pub struct InputState {
    pressed: HashSet<KeyCode>,
}

impl InputState {
    pub fn key_down(&mut self, key: KeyCode) {
        self.pressed.insert(key);
    }

    pub fn key_up(&mut self, key: KeyCode) {
        self.pressed.remove(&key);
    }

    pub fn clear(&mut self) {
        self.pressed.clear();
    }

    fn is_pressed(&self, key: KeyCode) -> bool {
        self.pressed.contains(&key)
    }

    fn is_pressed_either(&self, primary: KeyCode, alternate: KeyCode) -> bool {
        self.is_pressed(primary) || self.is_pressed(alternate)
    }

    pub fn move_intent(&self) -> MoveIntent {
        MoveIntent {
            forward: self.is_pressed_either(KeyCode::KeyW, KeyCode::ArrowUp),
            back: self.is_pressed_either(KeyCode::KeyS, KeyCode::ArrowDown),
            left: self.is_pressed_either(KeyCode::KeyA, KeyCode::ArrowLeft),
            right: self.is_pressed_either(KeyCode::KeyD, KeyCode::ArrowRight),
            jump: self.is_pressed(KeyCode::Space),
            cautious: self.is_pressed_either(KeyCode::ShiftLeft, KeyCode::ShiftRight),
        }
    }
}

#[cfg(test)]
mod tests {
    use winit::keyboard::KeyCode;

    use super::InputState;

    #[test]
    fn pressing_w_moves_forward() {
        let mut input = InputState::default();
        input.key_down(KeyCode::KeyW);
        assert!(input.move_intent().forward);
    }
}
