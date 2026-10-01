//! Native local movement state around the shared binding decisions.

use game_engine_core::{
    input_bindings_data::{BindingMouseButton, InputAction, InputBindingsData, InputState},
    movement_animation_data::direction_to_anim_id,
    movement_input_data::{
        MoveDirection, compute_movement_input, has_manual_movement_override,
        movement_speed_multiplier, movement_to_direction, sync_movement_toggles,
    },
    player_physics_data::{
        GroundState, VerticalState, apply_gravity_and_ground_snap, build_proposed_ground_movement,
        update_grounded,
    },
};
use game_engine_network::movement_control::{ScriptedMovement, ScriptedMovementStep};
use game_engine_session::SessionScreen;
use glam::Vec3;
use shared::{
    components::MovementSpeed,
    movement::{RUN_SPEED, SWIM_SPEED, WALK_SPEED, swim_top},
    protocol::PlayerInput,
};

use crate::swim::{at_swim_surface, swim_height};

pub(crate) struct PlayerMovement {
    pub running: bool,
    pub autorun: bool,
    pub jumping: bool,
    pub swimming: bool,
    direction: MoveDirection,
    /// Whether the swimmer floats at the water surface.
    at_surface: bool,
    /// Whether the last swim step changed height, for a vertical-only input.
    swim_rose: bool,
    /// Whether the previous `predict` left the player walking on dry ground.
    wading: bool,
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

/// What an input reports moving: a direction, a jump, or a swim step that changed height.
struct ReportedMotion {
    direction: [f32; 3],
    jumping: bool,
    swimming_vertically: bool,
}

pub(crate) struct MovementFrame {
    /// World direction; its Y is the pitched part of mouse-steered swimming.
    pub direction: [f32; 3],
    pub speed: f32,
    /// Swim ascend (+) or descend (-) speed: `SWIM_SPEED` × the server's aura modifier.
    pub vertical: f32,
}

impl Default for PlayerMovement {
    fn default() -> Self {
        Self {
            running: true,
            autorun: false,
            jumping: false,
            swimming: false,
            direction: MoveDirection::None,
            at_surface: false,
            swim_rose: false,
            wading: false,
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

/// Retail `JumpOrAscendStart` / `SitStandOrDescendStart`: the held swim vertical input.
fn swim_vertical_input(bindings: &InputBindingsData, input: &impl InputState) -> f32 {
    let ascend = bindings.is_pressed(InputAction::Jump, input);
    let descend = bindings.is_pressed(InputAction::SitOrStand, input);
    f32::from(u8::from(ascend)) - f32::from(u8::from(descend))
}

/// Tilt the forward part of `direction` by the camera `pitch` (the mouse-steered swimmer
/// follows the view); the lateral part stays level.
fn pitch_forward(direction: Vec3, yaw: f32, pitch: f32) -> Vec3 {
    let forward = Vec3::new(yaw.sin(), 0.0, yaw.cos());
    let along = direction.dot(forward);
    let lateral = direction - forward * along;
    lateral + (forward * pitch.cos() + Vec3::Y * pitch.sin()) * along
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

    /// `pitch` is the camera pitch, steering a swimmer moved with the right mouse button;
    /// `scripted_forward` is an IPC `ScriptedMovementForward` step.
    pub fn resolve(
        &mut self,
        bindings: &InputBindingsData,
        input: &impl InputState,
        yaw: f32,
        pitch: f32,
        scripted_forward: bool,
    ) -> MovementFrame {
        (self.autorun, self.running) =
            sync_movement_toggles(bindings, input, self.autorun, self.running);
        let (direction, animation) =
            compute_movement_input(bindings, input, self.autorun, scripted_forward, yaw);
        self.direction = animation;
        let mut direction = Vec3::from_array(direction);
        if self.swimming && input.mouse_pressed(BindingMouseButton::Right) {
            direction = pitch_forward(direction, yaw, pitch);
        }
        MovementFrame {
            direction: direction.to_array(),
            speed: self.unmodified_speed() * self.speed_modifier,
            vertical: if self.swimming {
                swim_vertical_input(bindings, input) * SWIM_SPEED * self.speed_modifier
            } else {
                0.0
            },
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
        position: Vec3,
        frame: MovementFrame,
        jump_pressed: bool,
        ground: &crate::ground::TerrainGround<'_>,
        delta: f32,
    ) -> Vec3 {
        self.swim_rose = false;
        let position = if self.swimming {
            self.swim(position, &frame, ground, delta)
        } else {
            self.walk(position, &frame, jump_pressed, ground, delta)
        };
        let was_swimming = self.swimming;
        let wading = self.wading && self.grounded;
        self.swimming = ground.swimming(position);
        self.wading = !self.swimming && self.grounded;
        if !self.swimming {
            return position;
        }
        self.jumping = false;
        self.vertical_velocity = 0.0;
        if was_swimming {
            return position;
        }
        self.enter_water(position, wading, ground)
    }

    /// Wading in turns into swimming at the floating height: a frame's walk can cross the
    /// threshold past it, onto a seabed below it. A fall or a spawn keeps its depth.
    fn enter_water(
        &mut self,
        feet: Vec3,
        wading: bool,
        ground: &crate::ground::TerrainGround<'_>,
    ) -> Vec3 {
        let Some(surface) = ground.water_surface(feet) else {
            return feet;
        };
        let feet = if wading {
            feet.with_y(feet.y.max(swim_top(surface)))
        } else {
            feet
        };
        self.at_surface = at_swim_surface(feet.y, surface);
        feet
    }

    fn walk(
        &mut self,
        mut position: Vec3,
        frame: &MovementFrame,
        jump_pressed: bool,
        ground: &crate::ground::TerrainGround<'_>,
        delta: f32,
    ) -> Vec3 {
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
        position
    }

    /// Swim along `frame` and the held ascend/descend, floating under the water surface.
    fn swim(
        &mut self,
        current: Vec3,
        frame: &MovementFrame,
        ground: &crate::ground::TerrainGround<'_>,
        delta: f32,
    ) -> Vec3 {
        let proposed =
            build_proposed_ground_movement(current, frame.direction.into(), frame.speed, delta)
                .unwrap_or(current);
        let moved = ground.validate_swim_move(current, proposed.with_y(current.y));
        // The server grants a swimmer `SWIM_SPEED` × the aura modifier of vertical travel
        // (pitched movement and ascend/descend together), and cuts a faster report short.
        let limit = SWIM_SPEED * self.speed_modifier * delta;
        let rise = (proposed.y - current.y + frame.vertical * delta).clamp(-limit, limit);
        let Some(surface) = ground.water_surface(moved) else {
            return moved;
        };
        let floor = match ground.probe(moved) {
            GroundState::Supported(height) => Some(height),
            _ => None,
        };
        let y = swim_height(moved.y, rise, surface, floor, self.at_surface);
        self.swim_rose = y != current.y;
        self.at_surface = at_swim_surface(y, surface);
        self.grounded = floor.is_some_and(|floor| y <= floor + 0.05);
        moved.with_y(y)
    }

    fn update_jump(
        &mut self,
        position: Vec3,
        pressed: bool,
        ground: &crate::ground::TerrainGround<'_>,
    ) {
        if pressed && self.grounded && !self.jumping {
            self.jumping = true;
            self.vertical_velocity = 7.0;
        }
        let landed = match ground.probe(position) {
            GroundState::Supported(height) => position.y <= height + 0.05,
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
        let swimming_vertically = self.swimming && self.swim_rose;
        let motion = ReportedMotion {
            direction,
            jumping: self.jumping,
            swimming_vertically,
        };
        self.report(motion, yaw, position, epoch)
    }

    /// The one stop input for movement a modal halted, keeping any airborne state local.
    pub fn stop_input(
        &mut self,
        yaw: f32,
        position: glam::Vec3,
        epoch: u32,
    ) -> Option<PlayerInput> {
        let halted = ReportedMotion {
            direction: [0.0; 3],
            jumping: false,
            swimming_vertically: false,
        };
        self.report(halted, yaw, position, epoch)
    }

    fn report(
        &mut self,
        motion: ReportedMotion,
        yaw: f32,
        position: glam::Vec3,
        epoch: u32,
    ) -> Option<PlayerInput> {
        let ReportedMotion {
            direction,
            jumping,
            swimming_vertically,
        } = motion;
        let moving = direction != [0.0; 3] || jumping || swimming_vertically;
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
        // World entry now loads unit visuals asynchronously. Local gameplay animation
        // starts at the same Loading → InWorld barrier as input and footsteps.
        if self.account.session.screen != SessionScreen::InWorld {
            return Ok(());
        }
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
            self.scripted_movement.stop();
            return Ok(());
        }
        if !self.gameplay_input_allowed() {
            self.halt_player_movement();
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
            self.halt_player_movement();
            return Ok(());
        }
        let keyboard = !viewport.gui_is_dragging()
            && !viewport
                .gui_get_focus_owner()
                .is_some_and(|focus| focus.is_class("LineEdit") || focus.is_class("TextEdit"));
        let Some(facing) = self.world.local_player_facing() else {
            self.halt_player_movement();
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
        let step = scripted_step(
            &mut self.scripted_movement,
            &mut self.player_movement,
            &self.client_options.bindings,
            &input,
            delta,
        );
        let yaw = step.and_then(|step| step.facing_yaw).unwrap_or(yaw);
        self.world.set_local_player_facing(yaw);
        if self.world.local_player_controlled() {
            self.halt_player_movement();
            return Ok(());
        }
        let frame = self.player_movement.resolve(
            &self.client_options.bindings,
            &input,
            yaw,
            self.world_camera.pitch(),
            step.is_some(),
        );
        let jump = self
            .client_options
            .bindings
            .is_just_pressed(InputAction::Jump, &input);
        if !self.player_movement.swimming
            && self
                .client_options
                .bindings
                .is_just_pressed(InputAction::SitOrStand, &input)
            && let Err(error) = self.sit_or_stand()
        {
            godot_error!("Sit or stand: {error}");
        }
        let delta = step.map_or(delta, |step| step.duration_secs);
        self.predict_player(frame, jump, yaw, delta)
    }

    /// Retail `SitStandOrDescendStart` on land: toggle sitting.
    fn sit_or_stand(&self) -> Result<(), String> {
        let current = self
            .world
            .local_player_stand_state()
            .ok_or("Local player has no replicated stand state")?;
        self.account
            .send_stand_state(crate::world::stand::sit_or_stand(current))
            .map_err(|error| error.0)
    }

    fn halt_player_movement(&mut self) {
        self.player_movement.stop();
        self.scripted_movement.stop();
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

    pub(super) fn send_player_input(&mut self) -> Result<(), crate::frame_error::FrameError> {
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
            Some(input) => Ok(self.account.send_player_input(input)?),
            None => Ok(()),
        }
    }

    /// The local player's replicated `MovementSpeed`, as the aura speed multiplier.
    fn adopt_server_speed(&mut self) {
        if let Some(speed) = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id)?.get::<MovementSpeed>())
        {
            self.player_movement.adopt_server_speed(speed.0);
        }
    }
}

/// This frame's IPC scripted movement (Bevy `advance_scripted_movement`): manual
/// movement input cancels it; a step turns autorun off.
fn scripted_step(
    scripted: &mut ScriptedMovement,
    movement: &mut PlayerMovement,
    bindings: &InputBindingsData,
    input: &impl InputState,
    delta: f32,
) -> Option<ScriptedMovementStep> {
    if has_manual_movement_override(bindings, input) {
        scripted.stop();
        return None;
    }
    let step = scripted.next_step(delta)?;
    movement.autorun = false;
    Some(step)
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
    use shared::movement::{SWIM_SPEED, swim_top};

    use crate::ground::TerrainGround;
    use crate::terrain::streaming::StreamedTerrain;

    /// The Stockade entrance (`tdb_world_safe_locs` 3599), world space.
    const FEET: Vec3 = Vec3::new(56.682, -19.269, -0.624);

    /// A scripted step runs forward along the facing with no key held and reports it
    /// as forward running on the wire.
    #[test]
    fn scripted_forward_runs_along_the_facing_without_keys() {
        let mut movement = PlayerMovement::default();
        let input = PhysicalInput::default();
        let yaw = std::f32::consts::FRAC_PI_2;
        let frame = movement.resolve(&InputBindingsData::default(), &input, yaw, 0.0, true);
        assert!((frame.direction[0] - 1.0).abs() < 1e-6 && frame.direction[2].abs() < 1e-6);
        assert_eq!(frame.speed, 7.0);
        let packet = movement.network_input(yaw, FEET, 2).unwrap();
        assert!((packet.direction[0] - 1.0).abs() < 1e-6);
        assert_eq!(packet.facing_yaw, yaw);
        assert!(packet.running);
        movement.resolve(&InputBindingsData::default(), &input, yaw, 0.0, false);
        assert_eq!(
            movement.network_input(yaw, FEET, 2).unwrap().direction,
            [0.0; 3]
        );
    }

    #[test]
    fn diagonal_prediction_keeps_original_forward_wire_priority() {
        let mut movement = PlayerMovement::default();
        let mut input = PhysicalInput::default();
        input.set_key(BindingKey::KeyW, true);
        input.set_key(BindingKey::KeyD, true);
        input.set_mouse(BindingMouseButton::Left, true);
        input.set_mouse(BindingMouseButton::Right, true);
        let frame = movement.resolve(&InputBindingsData::default(), &input, 0.0, 0.0, false);
        assert_eq!(frame.direction, [-1.0, 0.0, 2.0]);
        assert_eq!(frame.speed, 7.0);
        let packet = movement.network_input(0.0, FEET, 3).unwrap();
        assert_eq!(packet.direction, [0.0, 0.0, 1.0]);
        assert!(packet.running);
        assert!(!packet.jumping);
        assert!(!packet.swimming);
        input.clear();
        movement.resolve(&InputBindingsData::default(), &input, 0.0, 0.0, false);
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
        movement.resolve(&InputBindingsData::default(), &input, 0.0, 0.0, false);
        let moving = movement.network_input(0.0, FEET, 4).unwrap();
        assert_eq!(moving.direction, [0.0, 0.0, 1.0]);
        input.clear();
        movement.resolve(&InputBindingsData::default(), &input, 0.0, 0.0, false);
        let last = FEET + Vec3::new(0.0, 0.0, 0.117);
        let stop = movement.network_input(0.0, last, 4).unwrap();
        assert_eq!(stop.direction, [0.0; 3]);
        assert!(!stop.jumping);
        assert!(stop.running);
        assert_eq!(stop.position, last.to_array());
        assert_eq!(stop.epoch, 4);
        for _ in 0..3 {
            movement.resolve(&InputBindingsData::default(), &input, 0.0, 0.0, false);
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
        movement.resolve(&InputBindingsData::default(), &input, 0.0, 0.0, false);
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
            .resolve(&InputBindingsData::default(), &input, 0.0, 0.0, false)
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
        let terrain = StreamedTerrain::new(data_root.clone());
        let ground = TerrainGround {
            terrain: &terrain,
            walls: &|_, _, _| None,
        };
        let yaw = std::f32::consts::FRAC_PI_2;
        let mut movement = PlayerMovement::default();
        let mut input = PhysicalInput::default();
        input.set_key(BindingKey::KeyW, true);
        movement.resolve(&InputBindingsData::default(), &input, yaw, 0.0, false);
        movement.network_input(yaw, FEET, 3).unwrap();
        movement.adopt_server_speed(3.5);
        let mut feet = FEET;
        for _ in 0..60 {
            let frame = movement.resolve(&InputBindingsData::default(), &input, yaw, 0.0, false);
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
        let terrain = StreamedTerrain::new(data_root.clone());
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
            let frame = movement.resolve(&InputBindingsData::default(), &input, yaw, 0.0, false);
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
            0.0,
            false,
        );
        assert!(movement.network_input(0.0, FEET, 3).is_some());
        movement.stop();
        assert!(!movement.running);
        assert!(!movement.autorun);
        assert!(!movement.jumping);
        assert!(movement.network_input(0.0, FEET, 3).unwrap().direction == [0.0; 3]);
        assert!(movement.network_input(0.0, FEET, 3).is_none());
    }

    /// Cached `azeroth_32_48` at the swimming fixture's X=-8558 line: dry shore at Z522,
    /// water standing at least 3.76 yd deep over the seabed at Z500.
    const SHORE: Vec3 = Vec3::new(-8558.0, 144.960_08, 522.0);
    const DEEP_Z: f32 = 500.0;
    const WATER: f32 = 143.988_92;
    const DT: f32 = 1.0 / 60.0;
    /// Facing -Z (toward deep water): forward is `(sin yaw, 0, cos yaw)`.
    const INTO_WATER: f32 = std::f32::consts::PI;

    fn swimming_terrain() -> StreamedTerrain {
        let data_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let mut terrain = StreamedTerrain::new(data_root.clone());
        terrain.request_map("azeroth".into(), (32, 48)).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while !terrain.parsed_tiles.contains_key(&(32, 48)) {
            terrain.poll().expect("terrain worker alive");
            assert!(
                std::time::Instant::now() < deadline,
                "azeroth_32_48 timed out: {:?}",
                terrain.state().map_error
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        terrain
    }

    /// Hold `input` for `frames` frames at `yaw` and camera `pitch`.
    fn hold(
        movement: &mut PlayerMovement,
        ground: &TerrainGround<'_>,
        input: &PhysicalInput,
        feet: Vec3,
        view: (f32, f32),
        frames: usize,
    ) -> Vec3 {
        hold_at(movement, ground, input, feet, view, frames, DT)
    }

    fn hold_at(
        movement: &mut PlayerMovement,
        ground: &TerrainGround<'_>,
        input: &PhysicalInput,
        mut feet: Vec3,
        (yaw, pitch): (f32, f32),
        frames: usize,
        dt: f32,
    ) -> Vec3 {
        for _ in 0..frames {
            let frame = movement.resolve(&InputBindingsData::default(), input, yaw, pitch, false);
            feet = movement.predict(feet, frame, false, ground, dt);
        }
        feet
    }

    fn seabed(terrain: &StreamedTerrain) -> Vec3 {
        let y = terrain.height_at(SHORE.x, DEEP_Z).expect("seabed height");
        assert!(WATER - y > 3.0, "fixture water too shallow: {y}");
        Vec3::new(SHORE.x, y, DEEP_Z)
    }

    /// Retail `JUMP` runs `JumpOrAscendStart`, `SITORSTAND` `SitStandOrDescendStart`
    /// (Bindings_Standard.xml): in water Space ascends and X descends at swim speed. The
    /// swimmer floats with its feet `SWIM_DEPTH` under the surface and rises no higher.
    #[test]
    fn space_ascends_to_the_surface_and_x_descends_to_the_seabed() {
        let terrain = swimming_terrain();
        let ground = TerrainGround {
            terrain: &terrain,
            walls: &|_, _, _| None,
        };
        let bed = seabed(&terrain);
        let top = swim_top(WATER);
        let mut movement = PlayerMovement::default();
        let mut input = PhysicalInput::default();
        let idle = hold(&mut movement, &ground, &input, bed, (INTO_WATER, 0.0), 30);
        assert!(movement.swimming);
        assert!((idle - bed).length() < 1e-4, "seabed idle drifted: {idle}");

        input.set_key(BindingKey::Space, true);
        let rising = hold(&mut movement, &ground, &input, idle, (INTO_WATER, 0.0), 12);
        let rate = (rising.y - idle.y) / (12.0 * DT);
        assert!((rate - SWIM_SPEED).abs() < 0.01, "ascend rate {rate}");
        let packet = movement
            .network_input(INTO_WATER, rising, 4)
            .expect("ascend input");
        assert!(packet.swimming && !packet.jumping, "{packet:?}");
        assert_eq!(packet.position, rising.to_array());
        assert_eq!(packet.direction, [0.0; 3]);
        let surfaced = hold(
            &mut movement,
            &ground,
            &input,
            rising,
            (INTO_WATER, 0.0),
            120,
        );
        assert!(
            (surfaced.y - top).abs() < 1e-3,
            "not bobbing at the surface: {surfaced}"
        );
        assert!(movement.swimming && !movement.jumping);
        assert_eq!((surfaced.x, surfaced.z), (bed.x, bed.z));
        // Reaching the surface ends the vertical motion: exactly one stop, then quiet
        // while Space stays held there.
        let stop = movement
            .network_input(INTO_WATER, surfaced, 4)
            .expect("one stop at the surface");
        assert_eq!(stop.direction, [0.0; 3]);
        assert_eq!(stop.position, surfaced.to_array());
        assert!(stop.swimming && !stop.jumping);
        assert!(
            movement.network_input(INTO_WATER, surfaced, 4).is_none(),
            "Space held at the surface moves nothing and sends nothing"
        );

        input.set_key(BindingKey::Space, false);
        let floating = hold(
            &mut movement,
            &ground,
            &input,
            surfaced,
            (INTO_WATER, 0.0),
            60,
        );
        assert!(
            (floating - surfaced).length() < 1e-4,
            "surface idle drifted: {floating}"
        );
        assert!(movement.network_input(INTO_WATER, floating, 4).is_none());

        input.set_key(BindingKey::KeyX, true);
        let sinking = hold(
            &mut movement,
            &ground,
            &input,
            floating,
            (INTO_WATER, 0.0),
            12,
        );
        let rate = (floating.y - sinking.y) / (12.0 * DT);
        assert!((rate - SWIM_SPEED).abs() < 0.01, "descend rate {rate}");
        let descent = movement.network_input(INTO_WATER, sinking, 4).unwrap();
        assert_eq!(descent.position, sinking.to_array());
        // Releasing X mid-water sends exactly one stop.
        input.set_key(BindingKey::KeyX, false);
        let paused = hold(
            &mut movement,
            &ground,
            &input,
            sinking,
            (INTO_WATER, 0.0),
            1,
        );
        assert_eq!(paused, sinking, "released X kept sinking");
        let stop = movement.network_input(INTO_WATER, paused, 4).unwrap();
        assert_eq!(stop.direction, [0.0; 3]);
        assert!(movement.network_input(INTO_WATER, paused, 4).is_none());
        input.set_key(BindingKey::KeyX, true);
        let sunk = hold(
            &mut movement,
            &ground,
            &input,
            sinking,
            (INTO_WATER, 0.0),
            120,
        );
        assert!(
            (sunk.y - bed.y).abs() < 1e-3,
            "did not reach the seabed: {sunk}"
        );
        assert!(movement.swimming);

        input.set_key(BindingKey::KeyX, false);
        input.set_key(BindingKey::Space, true);
        input.set_key(BindingKey::KeyX, true);
        let both = hold(&mut movement, &ground, &input, sunk, (INTO_WATER, 0.0), 30);
        assert!(
            (both - sunk).length() < 1e-4,
            "Space+X should cancel: {both}"
        );
    }

    /// Vertical swim speed carries the server's aura modifier: half speed ascends at
    /// half `SWIM_SPEED`, a root (replicated speed 0) holds the swimmer in place.
    #[test]
    fn vertical_swim_speed_follows_the_server_speed_modifier() {
        let terrain = swimming_terrain();
        let ground = TerrainGround {
            terrain: &terrain,
            walls: &|_, _, _| None,
        };
        let bed = seabed(&terrain);
        let mut movement = PlayerMovement::default();
        let mut input = PhysicalInput::default();
        let idle = hold(&mut movement, &ground, &input, bed, (INTO_WATER, 0.0), 1);
        input.set_key(BindingKey::Space, true);
        let rising = hold(&mut movement, &ground, &input, idle, (INTO_WATER, 0.0), 1);
        movement.network_input(INTO_WATER, rising, 4).unwrap();
        movement.adopt_server_speed(SWIM_SPEED / 2.0);
        let slowed = hold(
            &mut movement,
            &ground,
            &input,
            rising,
            (INTO_WATER, 0.0),
            12,
        );
        let rate = (slowed.y - rising.y) / (12.0 * DT);
        assert!(
            (rate - SWIM_SPEED / 2.0).abs() < 0.01,
            "slowed ascend {rate}"
        );
        movement.network_input(INTO_WATER, slowed, 4).unwrap();
        movement.adopt_server_speed(0.0);
        let rooted = hold(
            &mut movement,
            &ground,
            &input,
            slowed,
            (INTO_WATER, 0.0),
            12,
        );
        assert_eq!(rooted, slowed, "rooted swimmer ascended");
    }

    /// Walking off the shore turns into swimming where the water reaches `SWIM_DEPTH` over
    /// the feet; from there the swimmer stays at the surface over deeper water, at the
    /// swim speed the server grants (`input.swimming` → `SWIM_SPEED`).
    #[test]
    fn walking_into_deep_water_floats_at_the_surface_at_swim_speed() {
        let terrain = swimming_terrain();
        let ground = TerrainGround {
            terrain: &terrain,
            walls: &|_, _, _| None,
        };
        let top = swim_top(WATER);
        let mut movement = PlayerMovement::default();
        let mut input = PhysicalInput::default();
        input.set_key(BindingKey::KeyW, true);
        let mut feet = SHORE;
        let mut frames = 0;
        while feet.z > DEEP_Z + 2.0 {
            feet = hold(&mut movement, &ground, &input, feet, (INTO_WATER, 0.0), 1);
            frames += 1;
            assert!(frames < 600, "never reached deep water: {feet}");
        }
        assert!(movement.swimming);
        // Wading turns into swimming at the floating height: no per-frame overshoot below
        // the surface, so Space there moves nothing and sends nothing.
        assert!(
            (feet.y - top).abs() < 1e-4,
            "not floating at the surface: {feet}"
        );
        input.set_key(BindingKey::KeyW, false);
        input.set_key(BindingKey::Space, true);
        let held = hold(&mut movement, &ground, &input, feet, (INTO_WATER, 0.0), 10);
        assert_eq!(held, feet, "Space at the surface moved the swimmer");
        assert!(movement.network_input(INTO_WATER, held, 4).is_none());
        input.set_key(BindingKey::Space, false);
        input.set_key(BindingKey::KeyW, true);
        let start = feet;
        let swum = hold(&mut movement, &ground, &input, start, (INTO_WATER, 0.0), 30);
        let speed = (start.z - swum.z) / (30.0 * DT);
        assert!((speed - SWIM_SPEED).abs() < 0.01, "swim speed {speed}");
        assert!((swum.y - top).abs() < 0.02, "left the surface: {swum}");
        let packet = movement.network_input(INTO_WATER, swum, 4).unwrap();
        assert!(packet.swimming && !packet.jumping);

        // Back to the shore: the rising seabed lifts the swimmer onto it, and it walks.
        input.set_key(BindingKey::KeyW, false);
        input.set_key(BindingKey::KeyS, true);
        let mut feet = swum;
        let mut frames = 0;
        while feet.z < SHORE.z {
            feet = hold(&mut movement, &ground, &input, feet, (INTO_WATER, 0.0), 1);
            frames += 1;
            assert!(frames < 1200, "never returned to the shore: {feet}");
        }
        assert!(!movement.swimming);
        let shore = terrain.height_at(feet.x, feet.z).unwrap();
        assert!(
            (feet.y - shore).abs() < 0.05,
            "not walking on the shore: {feet}"
        );
    }

    /// At 20 fps a frame wades 0.35 yd, far enough for the seabed to drop past the swim
    /// threshold: the swimmer still starts at the floating height, and Space there is quiet.
    #[test]
    fn slow_frames_wade_in_to_the_floating_height() {
        let terrain = swimming_terrain();
        let ground = TerrainGround {
            terrain: &terrain,
            walls: &|_, _, _| None,
        };
        let top = swim_top(WATER);
        let mut movement = PlayerMovement::default();
        let mut input = PhysicalInput::default();
        input.set_key(BindingKey::KeyW, true);
        let mut feet = SHORE;
        let mut frames = 0;
        while !movement.swimming {
            feet = hold_at(
                &mut movement,
                &ground,
                &input,
                feet,
                (INTO_WATER, 0.0),
                1,
                0.05,
            );
            frames += 1;
            assert!(frames < 200, "never started swimming: {feet}");
        }
        assert!(
            (top..top + 0.02).contains(&feet.y),
            "not wading in at the floating height: {feet}"
        );
        input.set_key(BindingKey::KeyW, false);
        input.set_key(BindingKey::Space, true);
        let held = hold_at(
            &mut movement,
            &ground,
            &input,
            feet,
            (INTO_WATER, 0.0),
            10,
            0.05,
        );
        assert_eq!(held, feet, "Space at the surface moved the swimmer");
        assert!(movement.network_input(INTO_WATER, held, 4).is_none());
    }

    /// Mouse-steered (right button) forward swimming follows the camera pitch; strafing
    /// and keyboard-only forward stay level.
    #[test]
    fn mouse_steered_forward_swim_follows_camera_pitch() {
        let terrain = swimming_terrain();
        let ground = TerrainGround {
            terrain: &terrain,
            walls: &|_, _, _| None,
        };
        let top = swim_top(WATER);
        let start = Vec3::new(SHORE.x, top - 1.0, DEEP_Z);
        let pitch = -0.5_f32;
        let mut movement = PlayerMovement::default();
        movement.swimming = true;
        let mut input = PhysicalInput::default();
        input.set_key(BindingKey::KeyW, true);
        let level = hold(
            &mut movement,
            &ground,
            &input,
            start,
            (INTO_WATER, pitch),
            30,
        );
        assert!(
            (level.y - start.y).abs() < 1e-4,
            "keyboard forward pitched: {level}"
        );
        input.set_mouse(BindingMouseButton::Right, true);
        let dove = hold(
            &mut movement,
            &ground,
            &input,
            level,
            (INTO_WATER, pitch),
            30,
        );
        let travel = SWIM_SPEED * 30.0 * DT;
        assert!(
            (level.y - dove.y - travel * 0.5_f32.sin()).abs() < 0.01,
            "dive depth {}",
            level.y - dove.y
        );
        assert!(
            (level.z - dove.z - travel * 0.5_f32.cos()).abs() < 0.01,
            "dive run {}",
            level.z - dove.z
        );
        input.set_key(BindingKey::KeyW, false);
        input.set_key(BindingKey::KeyA, true);
        let strafed = hold(
            &mut movement,
            &ground,
            &input,
            dove,
            (INTO_WATER, pitch),
            30,
        );
        assert!(
            (strafed.y - dove.y).abs() < 1e-4,
            "strafe pitched: {strafed}"
        );
    }

    /// The server grants a swimmer `SWIM_SPEED` × its aura modifier of vertical travel
    /// (game-server `move_toward_reported`), so a mouse-steered climb or dive with Space or
    /// X held changes height no faster than that; the forward part is unchanged.
    #[test]
    fn pitched_swim_with_ascend_or_descend_changes_height_at_most_at_swim_speed() {
        let terrain = swimming_terrain();
        let ground = TerrainGround {
            terrain: &terrain,
            walls: &|_, _, _| None,
        };
        let bed = seabed(&terrain);
        let pitch = 0.5_f32;
        let mut movement = PlayerMovement::default();
        let mut input = PhysicalInput::default();
        let start = hold(&mut movement, &ground, &input, bed, (INTO_WATER, pitch), 1);
        assert!(movement.swimming);
        input.set_key(BindingKey::KeyW, true);
        input.set_mouse(BindingMouseButton::Right, true);
        input.set_key(BindingKey::Space, true);
        let climbed = hold(
            &mut movement,
            &ground,
            &input,
            start,
            (INTO_WATER, pitch),
            24,
        );
        let seconds = 24.0 * DT;
        let rate = (climbed.y - start.y) / seconds;
        assert!((rate - SWIM_SPEED).abs() < 0.01, "climb rate {rate}");
        let run = (start.z - climbed.z) / seconds;
        assert!(
            (run - SWIM_SPEED * pitch.cos()).abs() < 0.01,
            "climb run {run}"
        );
        let report = movement.network_input(INTO_WATER, climbed, 4).unwrap();
        assert_eq!(report.position, climbed.to_array());

        input.set_key(BindingKey::Space, false);
        input.set_key(BindingKey::KeyX, true);
        let dove = hold(
            &mut movement,
            &ground,
            &input,
            climbed,
            (INTO_WATER, -pitch),
            12,
        );
        let rate = (climbed.y - dove.y) / (12.0 * DT);
        assert!((rate - SWIM_SPEED).abs() < 0.01, "dive rate {rate}");
    }
}
