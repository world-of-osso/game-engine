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
    movement::{RUN_SPEED, SWIM_SPEED, WALK_SPEED},
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
    /// Aura speed multiplier the server applies: its replicated `MovementSpeed` over the
    /// speed it computes without auras for the newest input reported to it.
    speed_modifier: f32,
    /// The newest replicated `MovementSpeed` of the local player.
    server_speed: Option<f32>,
    /// `unmodified_speed` of the newest input reported to the server.
    reported_speed: f32,
    /// Whether the newest reported input moved or jumped, so a release reports one stop.
    reported_motion: bool,
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
            speed_modifier: 1.0,
            server_speed: None,
            // The server spawns players with `MovementSpeed(RUN_SPEED)`.
            reported_speed: RUN_SPEED,
            reported_motion: false,
        }
    }
}

fn turn_animation_id(delta: f32) -> Option<u16> {
    const TURN_THRESHOLD: f32 = 0.02;
    if delta >= TURN_THRESHOLD {
        Some(11)
    } else if delta <= -TURN_THRESHOLD {
        Some(12)
    } else {
        None
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
        turn_animation_id(delta).unwrap_or(locomotion)
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
        MovementFrame {
            direction,
            speed: self.unmodified_speed() * self.speed_modifier,
        }
    }

    /// The server's speed for the current state without auras (game-server
    /// `compute_movement_speed`): swim, run or walk base × the direction multiplier.
    fn unmodified_speed(&self) -> f32 {
        let base = if self.swimming {
            SWIM_SPEED
        } else if self.running {
            RUN_SPEED
        } else {
            WALK_SPEED
        };
        base * movement_speed_multiplier(self.direction)
    }

    /// Adopt the local player's replicated `MovementSpeed` (base × auras × direction of the
    /// input the server applied) as the aura multiplier of the newest reported input. The
    /// server rewrites it only when it applies an input, so an unchanged value is not read
    /// again against a newer input.
    pub fn adopt_server_speed(&mut self, speed: f32) {
        if self.server_speed == Some(speed) {
            return;
        }
        self.server_speed = Some(speed);
        self.speed_modifier = speed / self.reported_speed;
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

    /// The input reporting `position`, where `predict` put the player, for the server to
    /// adopt; `epoch` is the server teleport that position follows. The first idle frame
    /// after movement reports one stop; later idle frames report nothing.
    pub fn network_input(
        &mut self,
        yaw: f32,
        position: glam::Vec3,
        epoch: u32,
    ) -> Option<PlayerInput> {
        let direction = movement_to_direction(self.direction, yaw);
        self.report(direction, self.jumping, yaw, position, epoch)
    }

    /// The one stop input for movement a modal halted, keeping any airborne state local.
    pub fn stop_input(
        &mut self,
        yaw: f32,
        position: glam::Vec3,
        epoch: u32,
    ) -> Option<PlayerInput> {
        self.report([0.0; 3], false, yaw, position, epoch)
    }

    fn report(
        &mut self,
        direction: [f32; 3],
        jumping: bool,
        yaw: f32,
        position: glam::Vec3,
        epoch: u32,
    ) -> Option<PlayerInput> {
        let moving = direction != [0.0; 3] || jumping;
        if !moving && !self.reported_motion {
            return None;
        }
        self.reported_motion = moving;
        self.reported_speed = self.unmodified_speed();
        Some(PlayerInput {
            direction,
            facing_yaw: yaw,
            running: self.running,
            jumping,
            swimming: self.swimming,
            position: position.to_array(),
            epoch,
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
        if self.game_menu_ui.is_some() {
            // Match original modal movement: stop direction/autorun, retain airborne state.
            self.player_movement.autorun = false;
            self.player_movement.direction = MoveDirection::None;
            return Ok(());
        }
        if !self.gameplay_input_allowed() {
            self.player_movement.stop();
            return Ok(());
        }
        self.adopt_server_speed();
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
        let space = player
            .get_world_3d()
            .and_then(|world| world.get_direct_space_state())
            .ok_or("Local player has no physics space")?;
        let walls = |origin, direction, length| {
            crate::wmo::collision::wall_hit(&space, origin, direction, length)
        };
        let ground = crate::ground::TerrainGround {
            terrain: &self.terrain,
            walls: &walls,
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

    pub(super) fn send_player_input(&mut self) -> Result<(), String> {
        if !self.gameplay_input_allowed() || self.world.local_player_controlled() {
            return Ok(());
        }
        let (Some(yaw), Some(node), Some(epoch)) = (
            self.world.local_player_facing(),
            self.world.local_player_node(),
            self.world.local_player_epoch(),
        ) else {
            return Ok(());
        };
        let position = node.get_position();
        let position = glam::Vec3::new(position.x, position.y, position.z);
        let input = if self.game_menu_ui.is_some() {
            self.player_movement.stop_input(yaw, position, epoch)
        } else {
            self.player_movement.network_input(yaw, position, epoch)
        };
        match input {
            Some(input) => self.account.send_player_input(input),
            None => Ok(()),
        }
    }

    /// The local player's replicated `MovementSpeed`, as the aura speed multiplier.
    fn adopt_server_speed(&mut self) {
        if let Some(speed) = self
            .world
            .local_player_id()
            .and_then(|id| self.units.get(&id)?.movement_speed)
        {
            self.player_movement.adopt_server_speed(speed.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PlayerMovement, turn_animation_id};
    use crate::input::PhysicalInput;
    use game_engine_core::input_bindings_data::{
        BindingKey, BindingMouseButton, InputBindingsData,
    };
    use game_engine_core::movement_input_data::MoveDirection;
    use glam::Vec3;

    use crate::ground::TerrainGround;
    use crate::terrain::streaming::StreamedTerrain;

    /// The Stockade entrance (`tdb_world_safe_locs` 3599), world space.
    const FEET: Vec3 = Vec3::new(56.682, -19.269, -0.624);

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
        let packet = movement.network_input(0.0, FEET, 3).unwrap();
        assert_eq!(packet.direction, [0.0, 0.0, 1.0]);
        assert!(packet.running);
        assert!(!packet.jumping);
        assert!(!packet.swimming);
        input.clear();
        movement.resolve(&InputBindingsData::default(), &input, 0.0);
        let stop = movement.network_input(0.0, FEET, 3).unwrap();
        assert_eq!(stop.direction, [0.0; 3]);
        assert!(movement.network_input(0.0, FEET, 3).is_none());
    }

    /// Releasing W reports exactly one stop: no direction, no jump, the final predicted
    /// position and the adopted epoch; idle frames after it send nothing.
    #[test]
    fn release_sends_exactly_one_stop_input_with_final_position_and_epoch() {
        let mut movement = PlayerMovement::default();
        let mut input = PhysicalInput::default();
        assert!(movement.network_input(0.0, FEET, 4).is_none());
        input.set_key(BindingKey::KeyW, true);
        movement.resolve(&InputBindingsData::default(), &input, 0.0);
        let moving = movement.network_input(0.0, FEET, 4).unwrap();
        assert_eq!(moving.direction, [0.0, 0.0, 1.0]);
        input.clear();
        movement.resolve(&InputBindingsData::default(), &input, 0.0);
        let last = FEET + Vec3::new(0.0, 0.0, 0.117);
        let stop = movement.network_input(0.0, last, 4).unwrap();
        assert_eq!(stop.direction, [0.0; 3]);
        assert!(!stop.jumping);
        assert!(stop.running);
        assert_eq!(stop.position, last.to_array());
        assert_eq!(stop.epoch, 4);
        for _ in 0..3 {
            movement.resolve(&InputBindingsData::default(), &input, 0.0);
            assert!(movement.network_input(0.0, last, 4).is_none());
        }
    }

    /// A modal stops a running player: `stop_input` reports the stop once, and nothing
    /// while the player never moved.
    #[test]
    fn stop_input_reports_a_modal_stop_once() {
        let mut movement = PlayerMovement::default();
        assert!(movement.stop_input(0.0, FEET, 2).is_none());
        let mut input = PhysicalInput::default();
        input.set_key(BindingKey::KeyW, true);
        movement.resolve(&InputBindingsData::default(), &input, 0.0);
        assert!(movement.network_input(0.0, FEET, 2).is_some());
        movement.stop();
        let stop = movement.stop_input(0.0, FEET, 2).unwrap();
        assert_eq!(stop.direction, [0.0; 3]);
        assert!(movement.stop_input(0.0, FEET, 2).is_none());
        assert!(movement.network_input(0.0, FEET, 2).is_none());
    }

    fn resolve_speed(movement: &mut PlayerMovement, keys: &[BindingKey]) -> f32 {
        let mut input = PhysicalInput::default();
        for key in keys {
            input.set_key(*key, true);
        }
        movement
            .resolve(&InputBindingsData::default(), &input, 0.0)
            .speed
    }

    /// The server replicates `MovementSpeed` = base × aura × direction for the input it
    /// applied. Running forward, 3.5 yd/s is a 50% snare; it then scales every speed.
    #[test]
    fn replicated_snare_scales_run_walk_backward_strafe_and_swim_speeds() {
        let mut movement = PlayerMovement::default();
        assert_eq!(resolve_speed(&mut movement, &[BindingKey::KeyW]), 7.0);
        movement.network_input(0.0, FEET, 1).unwrap();
        movement.adopt_server_speed(3.5);
        assert_eq!(resolve_speed(&mut movement, &[BindingKey::KeyW]), 3.5);
        assert!((resolve_speed(&mut movement, &[BindingKey::KeyS]) - 2.1).abs() < 1e-5);
        assert!((resolve_speed(&mut movement, &[BindingKey::KeyA]) - 2.8).abs() < 1e-5);
        movement.running = false;
        assert_eq!(resolve_speed(&mut movement, &[BindingKey::KeyW]), 1.25);
        movement.running = true;
        movement.swimming = true;
        let swim = resolve_speed(&mut movement, &[BindingKey::KeyW]);
        assert!(
            (swim - shared::movement::SWIM_SPEED * 0.5).abs() < 1e-5,
            "{swim}"
        );
    }

    /// Unsnared swimming uses the server's swim speed, not run speed.
    #[test]
    fn swimming_predicts_at_swim_speed() {
        let mut movement = PlayerMovement::default();
        movement.swimming = true;
        let swim = resolve_speed(&mut movement, &[BindingKey::KeyW]);
        assert_eq!(swim, shared::movement::SWIM_SPEED);
        let back = resolve_speed(&mut movement, &[BindingKey::KeyS]);
        assert!(
            (back - shared::movement::SWIM_SPEED * 0.6).abs() < 1e-5,
            "{back}"
        );
    }

    /// The replicated speed pairs with the input it was computed for: backpedalling
    /// unsnared replicates 4.2 = 7 × 0.6, which is no snare; the same value for a forward
    /// input is a 40% snare. An unchanged value does not re-derive against a newer input.
    #[test]
    fn replicated_speed_is_read_against_the_newest_reported_input() {
        let mut movement = PlayerMovement::default();
        resolve_speed(&mut movement, &[BindingKey::KeyS]);
        movement.network_input(0.0, FEET, 1).unwrap();
        movement.adopt_server_speed(4.2);
        assert!((resolve_speed(&mut movement, &[BindingKey::KeyW]) - 7.0).abs() < 1e-5);
        movement.network_input(0.0, FEET, 1).unwrap();
        movement.adopt_server_speed(4.2);
        assert!((resolve_speed(&mut movement, &[BindingKey::KeyW]) - 7.0).abs() < 1e-5);
        movement.adopt_server_speed(4.9);
        assert!((resolve_speed(&mut movement, &[BindingKey::KeyW]) - 4.9).abs() < 1e-5);
    }

    /// A root replicates 0: the player predicts no movement.
    #[test]
    fn replicated_root_stops_prediction() {
        let mut movement = PlayerMovement::default();
        resolve_speed(&mut movement, &[BindingKey::KeyW]);
        movement.network_input(0.0, FEET, 1).unwrap();
        movement.adopt_server_speed(0.0);
        assert_eq!(resolve_speed(&mut movement, &[BindingKey::KeyW]), 0.0);
    }

    /// One second of W from the Stockade entrance under a 50% snare predicts 3.5 yd,
    /// what the server grants in that second.
    #[test]
    fn snared_run_predicts_the_server_distance() {
        let data_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let terrain = StreamedTerrain::new(data_root.clone(), data_root.join("cache"));
        let ground = TerrainGround {
            terrain: &terrain,
            walls: &|_, _, _| None,
        };
        let yaw = std::f32::consts::FRAC_PI_2;
        let mut movement = PlayerMovement::default();
        let mut input = PhysicalInput::default();
        input.set_key(BindingKey::KeyW, true);
        movement.resolve(&InputBindingsData::default(), &input, yaw);
        movement.network_input(yaw, FEET, 3).unwrap();
        movement.adopt_server_speed(3.5);
        let mut feet = FEET;
        for _ in 0..60 {
            let frame = movement.resolve(&InputBindingsData::default(), &input, yaw);
            feet = movement.predict(feet, frame, false, &ground, 1.0 / 60.0);
        }
        let ran = (feet - FEET).length();
        assert!((ran - 3.5).abs() < 0.01, "{ran}");
    }

    /// W+D facing east for one second from the Stockade entrance: the input reports where
    /// `predict` ran the player, 7 yd along forward + right, while its wire direction is
    /// forward; the server adopts that position.
    #[test]
    fn strafe_run_reports_the_predicted_diagonal_position() {
        let data_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let terrain = StreamedTerrain::new(data_root.clone(), data_root.join("cache"));
        let ground = TerrainGround {
            terrain: &terrain,
            walls: &|_, _, _| None,
        };
        let yaw = std::f32::consts::FRAC_PI_2;
        let mut movement = PlayerMovement::default();
        let mut input = PhysicalInput::default();
        input.set_key(BindingKey::KeyW, true);
        input.set_key(BindingKey::KeyD, true);
        let mut feet = FEET;
        for _ in 0..60 {
            let frame = movement.resolve(&InputBindingsData::default(), &input, yaw);
            feet = movement.predict(feet, frame, false, &ground, 1.0 / 60.0);
        }
        let packet = movement.network_input(yaw, feet, 3).unwrap();
        assert_eq!(packet.position, feet.to_array());
        assert_eq!(packet.epoch, 3);
        let ran = feet - FEET;
        let diagonal = Vec3::new(1.0, 0.0, 1.0).normalize();
        assert!((ran.length() - 7.0).abs() < 0.01, "{ran}");
        assert!((ran.normalize() - diagonal).length() < 0.001, "{ran}");
        let wire = Vec3::from_array(packet.direction);
        assert!((wire - Vec3::X).length() < 1e-6, "{wire}");
    }

    #[test]
    fn turn_threshold_is_inclusive_for_both_directions() {
        assert_eq!(turn_animation_id(0.019), None);
        assert_eq!(turn_animation_id(0.02), Some(11));
        assert_eq!(turn_animation_id(-0.019), None);
        assert_eq!(turn_animation_id(-0.02), Some(12));
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
        assert!(movement.network_input(0.0, FEET, 3).is_some());
        movement.stop();
        assert!(!movement.running);
        assert!(!movement.autorun);
        assert!(!movement.jumping);
        assert!(movement.network_input(0.0, FEET, 3).unwrap().direction == [0.0; 3]);
        assert!(movement.network_input(0.0, FEET, 3).is_none());
    }
}
