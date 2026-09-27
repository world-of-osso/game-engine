//! Native local movement state around the shared binding decisions.

use game_engine_core::{
    input_bindings_data::{InputAction, InputBindingsData, InputState},
    movement_animation_data::direction_to_anim_id,
    movement_input_data::{
        MoveDirection, compute_movement_input, movement_speed_multiplier, movement_to_direction,
        sync_movement_toggles,
    },
    player_physics_data::{
        VerticalState, apply_gravity_and_ground_snap, build_proposed_ground_movement,
        update_grounded,
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
    previous_facing: Option<f32>,
    vertical_velocity: f32,
    grounded: bool,
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
            previous_facing: None,
            vertical_velocity: 0.0,
            grounded: true,
        }
    }
}

impl PlayerMovement {
    fn animation_id(&mut self, facing: f32) -> u16 {
        let previous = self.previous_facing.replace(facing);
        let locomotion = direction_to_anim_id(self.direction, self.running, self.swimming);
        if self.direction != MoveDirection::None || self.jumping || self.swimming {
            return locomotion;
        }
        let Some(previous) = previous else {
            return locomotion;
        };
        let delta = (facing - previous + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
            - std::f32::consts::PI;
        const TURN_THRESHOLD: f32 = 0.02;
        if delta >= TURN_THRESHOLD {
            11
        } else if delta <= -TURN_THRESHOLD {
            12
        } else {
            locomotion
        }
    }

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

    pub fn predict(
        &mut self,
        mut position: glam::Vec3,
        frame: MovementFrame,
        jump_pressed: bool,
        ground: &crate::ground::TerrainGround<'_>,
        delta: f32,
    ) -> glam::Vec3 {
        self.grounded = update_grounded(
            position.y,
            ground.probe(position),
            shared::movement::GROUND_SNAP_THRESHOLD,
        );
        if let Some(proposed) =
            build_proposed_ground_movement(position, frame.direction.into(), frame.speed, delta)
        {
            position = ground.validate_move(position, proposed, self.grounded && !self.jumping);
        }
        self.update_jump(position, jump_pressed, ground);
        let vertical = apply_gravity_and_ground_snap(
            VerticalState {
                y: position.y,
                vertical_velocity: self.vertical_velocity,
                grounded: self.grounded,
            },
            ground.probe(position),
            delta,
            shared::movement::GRAVITY,
        );
        self.vertical_velocity = vertical.vertical_velocity;
        self.grounded = vertical.grounded;
        position.y = vertical.y;
        self.swimming = ground.swimming(position);
        position
    }

    fn update_jump(
        &mut self,
        position: glam::Vec3,
        pressed: bool,
        ground: &crate::ground::TerrainGround<'_>,
    ) {
        self.swimming = ground.swimming(position);
        if self.swimming {
            self.jumping = false;
            return;
        }
        if pressed && self.grounded && !self.jumping {
            self.jumping = true;
            self.vertical_velocity = 7.0;
        }
        let landed = match ground.probe(position) {
            game_engine_core::player_physics_data::GroundState::Supported(height) => {
                position.y <= height + 0.05
            }
            _ => true,
        };
        if self.jumping && self.vertical_velocity <= 0.0 && self.grounded && landed {
            self.jumping = false;
        }
    }

    pub fn network_input(&self, yaw: f32, elapsed_secs: f32) -> Option<PlayerInput> {
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
            elapsed_secs,
        })
    }

    pub fn stop(&mut self) {
        self.autorun = false;
        self.direction = MoveDirection::None;
        self.jumping = false;
    }
}

impl crate::GameClient {
    pub(super) fn update_player_animation(&mut self) -> Result<(), String> {
        let Some(facing) = self.world.local_player_facing() else {
            return Ok(());
        };
        let animation_id = self.player_movement.animation_id(facing);
        let movement = &self.player_movement;
        self.world.update_local_locomotion(
            animation_id,
            movement.jumping,
            movement.running && movement.direction == MoveDirection::Forward,
        )
    }

    pub(super) fn update_player_input(&mut self, delta: f32) -> Result<(), String> {
        use game_engine_core::camera_input_data::CameraInput;
        use godot::prelude::*;
        if !self.gameplay_input_allowed() {
            self.player_movement.stop();
            self.physical_input.clear();
            return Ok(());
        }
        let viewport = self
            .base()
            .get_viewport()
            .ok_or("Gameplay root has no viewport")?;
        if viewport
            .get_embedded_subwindows()
            .iter_shared()
            .any(|window| window.is_visible() && window.is_exclusive())
        {
            self.player_movement.stop();
            return Ok(());
        }
        let keyboard = !viewport.gui_is_dragging()
            && !viewport
                .gui_get_focus_owner()
                .is_some_and(|focus| focus.is_class("LineEdit") || focus.is_class("TextEdit"));
        let Some(facing) = self.world.local_player_facing() else {
            self.player_movement.stop();
            return Ok(());
        };
        let input = self.physical_input.gameplay_state(keyboard);
        let [delta_x, delta_y] = self.physical_input.motion();
        let options = &self.client_options.camera;
        self.world_camera.configure(options);
        let yaw = self.world_camera.apply_input(
            facing,
            &self.client_options.bindings,
            &input,
            CameraInput {
                delta_x,
                delta_y,
                scroll_y: self.physical_input.scroll(),
                dt: delta,
                look_sensitivity: options.look_sensitivity,
                invert_y: options.invert_y,
            },
        );
        self.world.set_local_player_facing(yaw);
        if self.world.local_player_controlled() {
            self.player_movement.stop();
            return Ok(());
        }
        let frame = self
            .player_movement
            .resolve(&self.client_options.bindings, &input, yaw);
        let jump = self
            .client_options
            .bindings
            .is_just_pressed(InputAction::Jump, &input);
        self.predict_player(frame, jump, yaw, delta)
    }

    fn predict_player(
        &mut self,
        frame: MovementFrame,
        jump: bool,
        yaw: f32,
        delta: f32,
    ) -> Result<(), String> {
        use godot::prelude::*;
        let mut player = self
            .world
            .local_player_node()
            .ok_or("Selected player vanished during input")?;
        let current = player.get_position();
        let ground = crate::ground::TerrainGround {
            terrain: &self.terrain,
        };
        let next = self.player_movement.predict(
            glam::Vec3::new(current.x, current.y, current.z),
            frame,
            jump,
            &ground,
            delta,
        );
        player.set_position(Vector3::new(next.x, next.y, next.z));
        player.set_rotation(Vector3::new(0.0, yaw - std::f32::consts::FRAC_PI_2, 0.0));
        Ok(())
    }

    fn gameplay_input_allowed(&self) -> bool {
        self.account.session.screen == game_engine_session::SessionScreen::InWorld
            && self.account.session.gameplay_input_allowed()
    }

    pub(super) fn send_player_input(&self, elapsed_secs: f32) -> Result<(), String> {
        if !self.gameplay_input_allowed() || self.world.local_player_controlled() {
            return Ok(());
        }
        if let Some(yaw) = self.world.local_player_facing()
            && let Some(input) = self.player_movement.network_input(yaw, elapsed_secs)
        {
            self.account.send_player_input(input)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::PlayerMovement;
    use crate::input::PhysicalInput;
    use game_engine_core::input_bindings_data::{
        BindingKey, BindingMouseButton, InputBindingsData,
    };
    use game_engine_core::movement_input_data::MoveDirection;

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
        let packet = movement.network_input(0.0, 1.0 / 60.0).unwrap();
        assert_eq!(packet.elapsed_secs, 1.0 / 60.0);
        assert_eq!(
            movement
                .network_input(0.0, 1.0 / 30.0)
                .unwrap()
                .elapsed_secs,
            1.0 / 30.0
        );
        assert_eq!(packet.direction, [0.0, 0.0, 1.0]);
        assert!(packet.running);
        assert!(!packet.jumping);
        assert!(!packet.swimming);
        input.clear();
        movement.resolve(&InputBindingsData::default(), &input, 0.0);
        assert!(movement.network_input(0.0, 1.0 / 60.0).is_none());
    }

    #[test]
    fn idle_turn_samples_consecutive_facing_without_hysteresis() {
        let mut movement = PlayerMovement::default();
        assert_eq!(movement.animation_id(0.0), 0);
        assert_eq!(movement.animation_id(0.019), 0);
        assert_eq!(movement.animation_id(0.040), 11);
        assert_eq!(movement.animation_id(-0.080), 12);
        assert_eq!(movement.animation_id(-0.080), 0);
        assert_eq!(movement.animation_id(-0.101), 12);
    }

    #[test]
    fn idle_turn_normalizes_wraparound_and_ignores_subthreshold_motion() {
        let mut movement = PlayerMovement::default();
        let pi = std::f32::consts::PI;
        assert_eq!(movement.animation_id(pi - 0.01), 0);
        assert_eq!(movement.animation_id(-pi + 0.02), 11);
        assert_eq!(movement.animation_id(pi - 0.01), 12);
        assert_eq!(movement.animation_id(pi - 0.005), 0);
    }

    #[test]
    fn ineligible_frames_advance_facing_without_selecting_turn() {
        let mut movement = PlayerMovement::default();
        assert_eq!(movement.animation_id(0.0), 0);
        movement.direction = MoveDirection::Forward;
        assert_eq!(movement.animation_id(0.5), 5);
        movement.direction = MoveDirection::None;
        assert_eq!(movement.animation_id(0.5), 0);
        movement.jumping = true;
        assert_eq!(movement.animation_id(1.0), 0);
        movement.jumping = false;
        assert_eq!(movement.animation_id(1.0), 0);
        movement.swimming = true;
        assert_eq!(movement.animation_id(1.5), 41);
        movement.swimming = false;
        assert_eq!(movement.animation_id(1.5), 0);
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
        assert!(movement.network_input(0.0, 1.0 / 60.0).is_some());
        movement.stop();
        assert!(!movement.running);
        assert!(!movement.autorun);
        assert!(!movement.jumping);
        assert!(movement.network_input(0.0, 1.0 / 60.0).is_none());
    }
}
