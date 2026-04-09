use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::audio::SpatialListener;
use bevy::core_pipeline::prepass::{DepthPrepass, NormalPrepass};
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::light::ShadowFilteringMethod;
use bevy::pbr::ScreenSpaceAmbientOcclusion;
use bevy::picking::mesh_picking::ray_cast::MeshRayCast;
use bevy::prelude::*;
use std::collections::HashSet;

use crate::collision::{self, CharacterPhysics};
use crate::game_state::GameState;
use crate::pathing::PathingState;
use crate::taxi::TaxiState;
use crate::terrain_heightmap::TerrainHeightmap;
use game_engine::input_bindings::{InputAction, InputBindings};

#[path = "camera_controls.rs"]
mod camera_controls;
#[path = "camera_follow.rs"]
mod camera_follow;
#[path = "camera_post_process.rs"]
mod camera_post_process;

use camera_controls::{apply_keyboard_camera, camera_pitch_delta};
use camera_follow::camera_follow;
pub(crate) use camera_post_process::additive_particle_glow_tonemapping;
use camera_post_process::sync_camera_graphics_post_process;

pub struct WowCameraPlugin;

impl Plugin for WowCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<crate::client_options::CameraOptions>();
        app.init_resource::<PathingState>();
        app.add_systems(Update, sync_camera_graphics_post_process);
        app.add_systems(
            Update,
            (
                sync_camera_options,
                camera_input,
                cursor_grab,
                player_movement,
                camera_follow,
            )
                .chain()
                .run_if(in_state(GameState::InWorld)),
        );
    }
}

/// Marker for the player entity the camera orbits around.
#[derive(Component)]
pub struct Player;

/// Movement direction relative to the character's facing.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveDirection {
    #[default]
    None,
    Forward,
    Backward,
    Left,
    Right,
}

/// Base Y position (ground level) for the player (when no terrain is loaded).
pub(super) const GROUND_Y: f32 = 0.0;

/// Signals current movement direction, run/walk toggle, and jump state.
#[derive(Component)]
pub struct MovementState {
    pub direction: MoveDirection,
    pub running: bool,
    pub jumping: bool,
    pub autorun: bool,
    pub swimming: bool,
}

impl Default for MovementState {
    fn default() -> Self {
        Self {
            direction: MoveDirection::None,
            running: true, // WoW defaults to running
            jumping: false,
            autorun: false,
            swimming: false,
        }
    }
}

/// Character facing yaw (radians). RMB rotates this; the model entity rotation follows.
#[derive(Component)]
pub struct CharacterFacing {
    pub yaw: f32,
}

impl Default for CharacterFacing {
    fn default() -> Self {
        Self {
            yaw: std::f32::consts::PI,
        }
    }
}

#[derive(Component)]
pub struct WowCamera {
    pub pitch: f32,
    pub yaw: f32,
    pub distance: f32,
    pub target_distance: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    /// How fast the camera follows the player position (lerp speed).
    pub follow_speed: f32,
    /// How fast the camera zooms toward target_distance (lerp speed).
    pub zoom_speed: f32,
    /// Whether the camera is currently pulled in due to collision.
    pub collided: bool,
}

impl Default for WowCamera {
    fn default() -> Self {
        Self {
            pitch: -0.3,
            yaw: 0.0,
            distance: 15.0,
            target_distance: 15.0,
            min_distance: 2.0,
            max_distance: 40.0,
            follow_speed: 10.0,
            zoom_speed: 8.0,
            collided: false,
        }
    }
}

pub(crate) fn spawn_wow_camera(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Camera3d::default(),
            Msaa::Off,
            DepthPrepass,
            NormalPrepass,
            ScreenSpaceAmbientOcclusion::default(),
            TemporalAntiAliasing::default(),
            additive_particle_glow_tonemapping(),
            Transform::default(),
            WowCamera::default(),
            SpatialListener::new(0.3),
            ShadowFilteringMethod::Gaussian,
        ))
        .id()
}

const WALK_SPEED: f32 = 2.5; // M2 Walk movespeed (2.5 yards/sec)
const RUN_SPEED: f32 = 7.0; // M2 Run movespeed (7.0 yards/sec)
const ZOOM_STEP: f32 = 2.0;
const KEY_ROTATE_SPEED: f32 = 2.5; // radians/sec for arrow key rotation
const KEY_ZOOM_SPEED: f32 = 15.0; // units/sec for page up/down zoom
pub(super) const COLLISION_OFFSET: f32 = 0.3;
pub(super) const EYE_HEIGHT: f32 = 1.8;
/// Speed at which camera recovers (lerps back out) after collision clears.
pub(super) const COLLISION_RECOVERY_SPEED: f32 = 5.0;
const PITCH_LIMIT: f32 = 88.0_f32 * std::f32::consts::PI / 180.0;
const LANDING_EPSILON: f32 = 0.05;
const SWIM_DEPTH_THRESHOLD: f32 = 1.25;

fn camera_input(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    taxi: Option<Res<TaxiState>>,
    options: Res<crate::client_options::CameraOptions>,
    bindings: Res<InputBindings>,
    mut camera_q: Query<&mut WowCamera>,
    mut facing_q: Query<&mut CharacterFacing, With<Player>>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect)
        || modal_open.is_some()
        || taxi.as_deref().is_some_and(TaxiState::is_active)
    {
        return;
    }
    let Ok(mut cam) = camera_q.single_mut() else {
        return;
    };

    let rmb = mouse_buttons.pressed(MouseButton::Right);
    let lmb = mouse_buttons.pressed(MouseButton::Left);
    let delta = mouse_motion.delta;
    let dt = time.delta_secs();

    if rmb {
        // RMB: character snaps to face camera direction, then both rotate together
        let yaw_delta = -delta.x * options.look_sensitivity;
        cam.yaw += yaw_delta;
        cam.pitch += camera_pitch_delta(delta.y, &options);
        cam.pitch = cam.pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT);
        if let Ok(mut facing) = facing_q.single_mut() {
            facing.yaw = cam.yaw + std::f32::consts::PI;
        }
    } else if lmb {
        // LMB: orbit camera only (character doesn't turn)
        cam.yaw -= delta.x * options.look_sensitivity;
        cam.pitch += camera_pitch_delta(delta.y, &options);
        cam.pitch = cam.pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }

    apply_keyboard_camera(
        &keys,
        &mouse_buttons,
        &bindings,
        dt,
        &mut cam,
        &mut facing_q,
    );

    if mouse_scroll.delta.y != 0.0 {
        cam.target_distance -= mouse_scroll.delta.y * ZOOM_STEP;
        cam.target_distance = cam
            .target_distance
            .clamp(cam.min_distance, cam.max_distance);
    }
}

fn sync_camera_options(
    options: Res<crate::client_options::CameraOptions>,
    mut camera_q: Query<&mut WowCamera>,
) {
    if !options.is_changed() {
        return;
    }
    for mut camera in &mut camera_q {
        camera.follow_speed = options.follow_speed;
        camera.zoom_speed = options.zoom_speed;
        camera.min_distance = options.min_distance;
        camera.max_distance = options.max_distance.max(options.min_distance + 1.0);
        camera.target_distance = camera
            .target_distance
            .clamp(camera.min_distance, camera.max_distance);
        camera.distance = camera
            .distance
            .clamp(camera.min_distance, camera.max_distance);
    }
}

/// Compute movement direction vector and animation direction from input.
fn compute_movement_input(
    keys: &ButtonInput<KeyCode>,
    mouse_buttons: &ButtonInput<MouseButton>,
    bindings: &InputBindings,
    autorun: bool,
    scripted_forward: bool,
    facing: &CharacterFacing,
) -> (Vec3, MoveDirection) {
    let forward = Vec3::new(facing.yaw.sin(), 0.0, facing.yaw.cos());
    let right = Vec3::new(-forward.z, 0.0, forward.x);

    let mut direction = Vec3::ZERO;
    if bindings.is_pressed(InputAction::MoveForward, keys, mouse_buttons)
        || autorun
        || scripted_forward
    {
        direction += forward;
    }
    if bindings.is_pressed(InputAction::MoveBackward, keys, mouse_buttons) {
        direction -= forward;
    }
    if bindings.is_pressed(InputAction::StrafeLeft, keys, mouse_buttons) {
        direction -= right;
    }
    if bindings.is_pressed(InputAction::StrafeRight, keys, mouse_buttons) {
        direction += right;
    }
    if mouse_buttons.pressed(MouseButton::Left) && mouse_buttons.pressed(MouseButton::Right) {
        direction += forward;
    }

    let fwd = bindings.is_pressed(InputAction::MoveForward, keys, mouse_buttons)
        || autorun
        || scripted_forward
        || (mouse_buttons.pressed(MouseButton::Left) && mouse_buttons.pressed(MouseButton::Right));
    let anim_dir = if fwd {
        MoveDirection::Forward
    } else if bindings.is_pressed(InputAction::MoveBackward, keys, mouse_buttons) {
        MoveDirection::Backward
    } else if bindings.is_pressed(InputAction::StrafeLeft, keys, mouse_buttons) {
        MoveDirection::Left
    } else if bindings.is_pressed(InputAction::StrafeRight, keys, mouse_buttons) {
        MoveDirection::Right
    } else {
        MoveDirection::None
    };

    (direction, anim_dir)
}

fn player_movement(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    terrain: Option<Res<TerrainHeightmap>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    taxi: Option<Res<TaxiState>>,
    mut map_status: ResMut<game_engine::status::MapStatusSnapshot>,
    bindings: Res<InputBindings>,
    mut pathing: ResMut<PathingState>,
    mut ray_cast: MeshRayCast,
    wmo_collision_meshes_q: Query<Entity, With<collision::WmoCollisionMesh>>,
    doodad_collider_q: Query<&game_engine::culling::DoodadCollider>,
    mut player_q: Query<
        (
            &mut Transform,
            &mut MovementState,
            &mut CharacterFacing,
            &mut CharacterPhysics,
        ),
        With<Player>,
    >,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) {
        return;
    }
    let Ok((mut transform, mut movement, mut facing, mut physics)) = player_q.single_mut() else {
        return;
    };
    if taxi.as_deref().is_some_and(TaxiState::is_active) {
        movement.autorun = false;
        movement.direction = MoveDirection::None;
        return;
    }

    if close_player_movement_for_modal(modal_open.as_deref(), &mut movement) {
        return;
    }

    sync_swimming_state(transform.translation, terrain.as_deref(), &mut movement);

    sync_player_movement_toggles(&keys, &mouse_buttons, &bindings, &mut movement);
    let manual_override =
        has_manual_movement_override(&keys, &mouse_buttons, &bindings, modal_open.as_deref());
    let current_position = transform.translation;
    let collision_meshes = collect_collision_meshes(&wmo_collision_meshes_q);
    let doodad_colliders = collect_doodad_colliders(&doodad_collider_q);
    let scripted_forward = crate::pathing::update_waypoint_pathing(
        &mut pathing,
        &mut map_status,
        current_position,
        terrain.as_deref(),
        &mut ray_cast,
        &collision_meshes,
        &doodad_colliders,
        manual_override,
    )
    .map(|directive| {
        facing.yaw = directive.facing_yaw;
        true
    })
    .unwrap_or(false);
    let (direction, speed) = resolve_player_movement_state(
        &keys,
        &mouse_buttons,
        &bindings,
        &mut movement,
        scripted_forward,
        &facing,
    );
    let proposed =
        build_proposed_ground_movement(current_position, direction, speed, time.delta_secs());
    apply_horizontal_movement(HorizontalMovementContext {
        transform: &mut transform,
        movement: &mut movement,
        physics: &mut physics,
        keys: &keys,
        mouse_buttons: &mouse_buttons,
        bindings: &bindings,
        terrain: terrain.as_deref(),
        proposed: proposed.map(|proposed| {
            let after_wmo = collision::clamp_movement_against_wmo_meshes(
                current_position,
                proposed,
                &mut ray_cast,
                &collision_meshes,
            );
            collision::clamp_movement_against_doodad_colliders(
                current_position,
                after_wmo,
                &doodad_colliders,
            )
        }),
    });

    sync_swimming_state(transform.translation, terrain.as_deref(), &mut movement);

    transform.rotation = Quat::from_rotation_y(facing.yaw - std::f32::consts::FRAC_PI_2);
}

fn collect_collision_meshes(
    collision_meshes: &Query<Entity, With<collision::WmoCollisionMesh>>,
) -> HashSet<Entity> {
    collision_meshes.iter().collect()
}

fn collect_doodad_colliders(
    collider_q: &Query<&game_engine::culling::DoodadCollider>,
) -> Vec<(Vec3, Vec3)> {
    collider_q
        .iter()
        .map(|c| (c.world_min, c.world_max))
        .collect()
}

fn build_proposed_ground_movement(
    current: Vec3,
    direction: Vec3,
    speed: f32,
    dt: f32,
) -> Option<Vec3> {
    if direction.length_squared() == 0.0 {
        return None;
    }
    Some(current + direction.normalize() * speed * dt)
}

fn close_player_movement_for_modal(
    modal_open: Option<&crate::scenes::game_menu::UiModalOpen>,
    movement: &mut MovementState,
) -> bool {
    if modal_open.is_none() {
        return false;
    }
    movement.autorun = false;
    movement.direction = MoveDirection::None;
    true
}

fn sync_player_movement_toggles(
    keys: &ButtonInput<KeyCode>,
    mouse_buttons: &ButtonInput<MouseButton>,
    bindings: &InputBindings,
    movement: &mut MovementState,
) {
    if bindings.is_just_pressed(InputAction::AutoRun, keys, mouse_buttons) {
        movement.autorun = !movement.autorun;
    }
    if bindings.is_pressed(InputAction::MoveBackward, keys, mouse_buttons) {
        movement.autorun = false;
    }
    if bindings.is_just_pressed(InputAction::RunToggle, keys, mouse_buttons) {
        movement.running = !movement.running;
    }
}

fn resolve_player_movement_state(
    keys: &ButtonInput<KeyCode>,
    mouse_buttons: &ButtonInput<MouseButton>,
    bindings: &InputBindings,
    movement: &mut MovementState,
    scripted_forward: bool,
    facing: &CharacterFacing,
) -> (Vec3, f32) {
    let (direction, anim_dir) = compute_movement_input(
        keys,
        mouse_buttons,
        bindings,
        movement.autorun,
        scripted_forward,
        facing,
    );
    movement.direction = anim_dir;
    let base_speed = if movement.running {
        RUN_SPEED
    } else {
        WALK_SPEED
    };
    let speed = base_speed * movement_speed_multiplier(anim_dir);
    (direction, speed)
}

fn has_manual_movement_override(
    keys: &ButtonInput<KeyCode>,
    mouse_buttons: &ButtonInput<MouseButton>,
    bindings: &InputBindings,
    modal_open: Option<&crate::scenes::game_menu::UiModalOpen>,
) -> bool {
    modal_open.is_some()
        || bindings.is_pressed(InputAction::MoveForward, keys, mouse_buttons)
        || bindings.is_pressed(InputAction::MoveBackward, keys, mouse_buttons)
        || bindings.is_pressed(InputAction::StrafeLeft, keys, mouse_buttons)
        || bindings.is_pressed(InputAction::StrafeRight, keys, mouse_buttons)
        || bindings.is_just_pressed(InputAction::Jump, keys, mouse_buttons)
        || bindings.is_just_pressed(InputAction::AutoRun, keys, mouse_buttons)
        || (mouse_buttons.pressed(MouseButton::Left) && mouse_buttons.pressed(MouseButton::Right))
}

fn movement_speed_multiplier(direction: MoveDirection) -> f32 {
    match direction {
        MoveDirection::Backward => shared::movement::BACKPEDAL_MULTIPLIER,
        MoveDirection::Left | MoveDirection::Right => shared::movement::STRAFE_MULTIPLIER,
        MoveDirection::None | MoveDirection::Forward => 1.0,
    }
}

struct HorizontalMovementContext<'a> {
    transform: &'a mut Transform,
    movement: &'a mut MovementState,
    physics: &'a mut CharacterPhysics,
    keys: &'a ButtonInput<KeyCode>,
    mouse_buttons: &'a ButtonInput<MouseButton>,
    bindings: &'a InputBindings,
    terrain: Option<&'a TerrainHeightmap>,
    proposed: Option<Vec3>,
}

fn apply_horizontal_movement(ctx: HorizontalMovementContext<'_>) {
    let HorizontalMovementContext {
        transform,
        movement,
        physics,
        keys,
        mouse_buttons,
        bindings,
        terrain,
        proposed,
    } = ctx;
    apply_ground_movement(transform, movement, physics, proposed, terrain);
    apply_jump_input(movement, physics, bindings, keys, mouse_buttons);
    finish_jump_if_landed(transform, movement, physics, terrain);
}

fn apply_ground_movement(
    transform: &mut Transform,
    movement: &MovementState,
    physics: &CharacterPhysics,
    proposed: Option<Vec3>,
    terrain: Option<&TerrainHeightmap>,
) {
    let Some(proposed) = proposed else { return };
    transform.translation = match terrain {
        Some(t) => collision::validate_movement_slope(
            transform.translation,
            proposed,
            t,
            physics.grounded && !movement.jumping,
        ),
        None => proposed,
    };
}

fn apply_jump_input(
    movement: &mut MovementState,
    physics: &mut CharacterPhysics,
    bindings: &InputBindings,
    keys: &ButtonInput<KeyCode>,
    mouse_buttons: &ButtonInput<MouseButton>,
) {
    if movement.swimming {
        movement.jumping = false;
        return;
    }
    if bindings.is_just_pressed(InputAction::Jump, keys, mouse_buttons)
        && physics.grounded
        && !movement.jumping
    {
        movement.jumping = true;
        physics.vertical_velocity = collision::JUMP_IMPULSE;
    }
}

fn sync_swimming_state(
    position: Vec3,
    terrain: Option<&TerrainHeightmap>,
    movement: &mut MovementState,
) {
    movement.swimming = terrain.is_some_and(|terrain| is_swimming(position, terrain));
    if movement.swimming {
        movement.jumping = false;
    }
}

fn is_swimming(position: Vec3, terrain: &TerrainHeightmap) -> bool {
    let Some(ground_y) = terrain.height_at(position.x, position.z) else {
        return false;
    };
    let depth = terrain
        .water_surface_at(position.x, position.z)
        .filter(|surface_y| *surface_y > ground_y)
        .map(|surface_y| surface_y - ground_y)
        .unwrap_or(0.0);
    depth >= SWIM_DEPTH_THRESHOLD
}

fn finish_jump_if_landed(
    transform: &Transform,
    movement: &mut MovementState,
    physics: &CharacterPhysics,
    terrain: Option<&TerrainHeightmap>,
) {
    if movement.jumping
        && physics.vertical_velocity <= 0.0
        && should_end_jump(transform, physics, terrain)
    {
        movement.jumping = false;
    }
}

fn should_end_jump(
    transform: &Transform,
    physics: &CharacterPhysics,
    terrain: Option<&TerrainHeightmap>,
) -> bool {
    if !physics.grounded {
        return false;
    }

    match terrain.and_then(|t| t.height_at(transform.translation.x, transform.translation.z)) {
        Some(ground_y) => transform.translation.y <= ground_y + LANDING_EPSILON,
        None => true,
    }
}

fn cursor_grab(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    mut cursor_q: Query<&mut bevy::window::CursorOptions>,
) {
    let Ok(mut cursor) = cursor_q.single_mut() else {
        return;
    };
    if !crate::networking::gameplay_input_allowed(reconnect) {
        cursor.grab_mode = bevy::window::CursorGrabMode::None;
        cursor.visible = true;
        return;
    }
    let held =
        mouse_buttons.pressed(MouseButton::Left) || mouse_buttons.pressed(MouseButton::Right);
    if held {
        cursor.grab_mode = bevy::window::CursorGrabMode::Locked;
        cursor.visible = false;
    } else {
        cursor.grab_mode = bevy::window::CursorGrabMode::None;
        cursor.visible = true;
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/camera_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../tests/unit/camera_post_process_tests.rs"]
mod post_process_tests;
