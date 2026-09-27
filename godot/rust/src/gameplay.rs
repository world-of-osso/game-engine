//! Native local movement state around the shared binding decisions.

use game_engine_core::{
    input_bindings_data::{InputBindingsData, InputState},
    movement_input_data::{
        MoveDirection, compute_movement_input, movement_speed_multiplier, movement_to_direction,
        sync_movement_toggles,
    },
};
use shared::{
    movement::{RUN_SPEED, WALK_SPEED},
    protocol::PlayerInput,
};

pub(crate) struct PlayerMovement {
    pub running: bool,
    pub autorun: bool,
    pub jumping: bool,
    pub swimming: bool,
    direction: MoveDirection,
}

pub(crate) struct MovementFrame {
    pub direction: [f32; 3],
    pub speed: f32,
}

impl Default for PlayerMovement {
    fn default() -> Self {
        Self {
            running: true,
            autorun: false,
            jumping: false,
            swimming: false,
            direction: MoveDirection::None,
        }
    }
}

impl PlayerMovement {
    pub fn resolve(
        &mut self,
        bindings: &InputBindingsData,
        input: &impl InputState,
        yaw: f32,
    ) -> MovementFrame {
        (self.autorun, self.running) =
            sync_movement_toggles(bindings, input, self.autorun, self.running);
        let (direction, animation) =
            compute_movement_input(bindings, input, self.autorun, false, yaw);
        self.direction = animation;
        let speed = if self.running { RUN_SPEED } else { WALK_SPEED };
        MovementFrame {
            direction,
            speed: speed * movement_speed_multiplier(animation),
        }
    }

    pub fn network_input(&self, yaw: f32) -> Option<PlayerInput> {
        let direction = movement_to_direction(self.direction, yaw);
        if direction == [0.0; 3] && !self.jumping {
            return None;
        }
        Some(PlayerInput {
            direction,
            facing_yaw: yaw,
            running: self.running,
            jumping: self.jumping,
            swimming: self.swimming,
        })
    }

    pub fn stop(&mut self) {
        self.autorun = false;
        self.direction = MoveDirection::None;
        self.jumping = false;
    }
}

#[cfg(test)]
mod tests {
    use super::PlayerMovement;
    use crate::input::PhysicalInput;
    use game_engine_core::input_bindings_data::{
        BindingKey, BindingMouseButton, InputBindingsData,
    };

    #[test]
    fn diagonal_prediction_keeps_original_forward_wire_priority() {
        let mut movement = PlayerMovement::default();
        let mut input = PhysicalInput::default();
        input.set_key(BindingKey::KeyW, true);
        input.set_key(BindingKey::KeyD, true);
        input.set_mouse(BindingMouseButton::Left, true);
        input.set_mouse(BindingMouseButton::Right, true);
        let frame = movement.resolve(&InputBindingsData::default(), &input, 0.0);
        assert_eq!(frame.direction, [-1.0, 0.0, 2.0]);
        assert_eq!(frame.speed, 7.0);
        let packet = movement.network_input(0.0).unwrap();
        assert_eq!(packet.direction, [0.0, 0.0, 1.0]);
        assert!(packet.running);
        assert!(!packet.jumping);
        assert!(!packet.swimming);
        input.clear();
        movement.resolve(&InputBindingsData::default(), &input, 0.0);
        assert!(movement.network_input(0.0).is_none());
    }

    #[test]
    fn stopping_movement_clears_autorun_direction_and_jump_but_keeps_run_preference() {
        let mut movement = PlayerMovement::default();
        movement.running = false;
        movement.autorun = true;
        movement.jumping = true;
        movement.resolve(
            &InputBindingsData::default(),
            &PhysicalInput::default(),
            0.0,
        );
        assert!(movement.network_input(0.0).is_some());
        movement.stop();
        assert!(!movement.running);
        assert!(!movement.autorun);
        assert!(!movement.jumping);
        assert!(movement.network_input(0.0).is_none());
    }
}
