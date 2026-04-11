use std::f32::consts::FRAC_PI_8;

use bevy::camera::ClearColorConfig;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use game_engine::customization_data::ModelPresentation;

use crate::camera::additive_particle_glow_tonemapping;
use crate::orbit_camera::scaled_orbit_delta;
use crate::terrain_heightmap::TerrainHeightmap;

use super::{CharSelectModelRoot, CharSelectScene};

pub(super) type SceneEntry = crate::scenes::char_select::warband::WarbandSceneEntry;
pub(super) type ScenePlacement = crate::scenes::char_select::warband::WarbandScenePlacement;

#[derive(Component, Clone)]
pub(super) struct CharSelectOrbit {
    pub(super) yaw: f32,
    pub(super) base_yaw: f32,
    pub(super) pitch: f32,
    pub(super) focus: Vec3,
    pub(super) distance: f32,
    pub(super) base_pitch: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct OrbitInputDebugState {
    pub(super) left_mouse_pressed: bool,
    pub(super) has_mouse_motion: bool,
    pub(super) orbit_entity_count: usize,
}

const ORBIT_YAW_LIMIT: f32 = FRAC_PI_8;
const ORBIT_PITCH_LIMIT: f32 = 0.15;
const SOLO_CHARACTER_CAMERA_DISTANCE: f32 = 6.5;
const SOLO_CHARACTER_MAX_FOV_DEGREES: f32 = 55.0;
pub(super) const CHAR_SELECT_CAMERA_GROUND_CLEARANCE: f32 = 0.5;
const CHAR_SELECT_FOG_START_DISTANCE_MULTIPLIER: f32 = 2.0;
const CHAR_SELECT_FOG_END_DISTANCE_MULTIPLIER: f32 = 5.0;
const CHAR_SELECT_CLEAR_COLOR: Color = Color::srgb(0.05, 0.06, 0.08);
const CHAR_SELECT_FOG_COLOR: Color = Color::srgb(0.18, 0.2, 0.23);
const CHAR_SELECT_FOG_LIGHT_COLOR: Color = Color::srgb(0.35, 0.38, 0.42);
const DEFAULT_CAMERA_EYE: Vec3 = Vec3::new(0.0, 1.8, 6.0);
const DEFAULT_CAMERA_FOCUS: Vec3 = Vec3::new(0.0, 1.0, 0.0);
const DEFAULT_CAMERA_FOV_DEGREES: f32 = 45.0;

enum CameraTarget<'a> {
    Default,
    Scene(&'a SceneEntry),
    Solo {
        scene: &'a SceneEntry,
        placement: &'a ScenePlacement,
    },
}

impl<'a> CameraTarget<'a> {
    fn resolve(scene: Option<&'a SceneEntry>, placement: Option<&'a ScenePlacement>) -> Self {
        match (scene, placement) {
            (Some(scene), Some(placement)) => Self::Solo { scene, placement },
            (Some(scene), None) => Self::Scene(scene),
            (None, _) => Self::Default,
        }
    }
}

pub(super) fn char_select_fog(camera_distance: f32) -> DistanceFog {
    let start = camera_distance * CHAR_SELECT_FOG_START_DISTANCE_MULTIPLIER;
    let end = camera_distance * CHAR_SELECT_FOG_END_DISTANCE_MULTIPLIER;
    DistanceFog {
        color: CHAR_SELECT_FOG_COLOR,
        directional_light_color: CHAR_SELECT_FOG_LIGHT_COLOR,
        directional_light_exponent: 8.0,
        falloff: FogFalloff::Linear { start, end },
    }
}

fn single_character_focus(
    scene: &SceneEntry,
    placement: &ScenePlacement,
    presentation: ModelPresentation,
) -> Vec3 {
    let _ = scene;
    let char_pos = placement.bevy_position();
    let focus_y = char_pos.y + presentation.customize_scale.max(0.01);
    Vec3::new(char_pos.x, focus_y, char_pos.z)
}

pub(super) fn camera_params(
    scene: Option<&SceneEntry>,
    placement: Option<&ScenePlacement>,
    presentation: ModelPresentation,
) -> (Vec3, Vec3, f32) {
    match CameraTarget::resolve(scene, placement) {
        CameraTarget::Solo { scene, placement } => {
            let scene_eye = scene.bevy_position();
            let scene_focus = scene.bevy_look_at();
            let focus = single_character_focus(scene, placement, presentation);
            let distance = (SOLO_CHARACTER_CAMERA_DISTANCE + presentation.camera_distance_offset)
                .clamp(3.5, (scene_eye - scene_focus).length());
            let eye = solo_camera_eye(scene_eye, scene_focus, focus, distance);
            let fov = scene.fov.min(SOLO_CHARACTER_MAX_FOV_DEGREES);
            (eye, focus, fov)
        }
        CameraTarget::Scene(scene) => (scene.bevy_position(), scene.bevy_look_at(), scene.fov),
        CameraTarget::Default => (
            DEFAULT_CAMERA_EYE,
            DEFAULT_CAMERA_FOCUS,
            DEFAULT_CAMERA_FOV_DEGREES,
        ),
    }
}

fn solo_camera_eye(scene_eye: Vec3, scene_focus: Vec3, focus: Vec3, distance: f32) -> Vec3 {
    let scene_offset = scene_eye - scene_focus;
    let vertical = scene_offset.y;
    let horizontal = Vec3::new(scene_offset.x, 0.0, scene_offset.z);
    let horizontal_dir = horizontal.normalize_or_zero();
    let horizontal_distance = (distance * distance - vertical * vertical).max(0.0).sqrt();
    focus + horizontal_dir * horizontal_distance + Vec3::Y * vertical
}

pub(super) fn orbit_from_eye_focus(eye: Vec3, focus: Vec3) -> CharSelectOrbit {
    let offset = eye - focus;
    let distance = offset.length();
    let base_yaw = offset.x.atan2(offset.z);
    let base_pitch = if distance > 0.0 {
        (offset.y / distance).asin()
    } else {
        0.0
    };
    CharSelectOrbit {
        yaw: 0.0,
        base_yaw,
        pitch: 0.0,
        focus,
        distance,
        base_pitch,
    }
}

pub(super) fn orbit_eye(orbit: &CharSelectOrbit) -> Vec3 {
    let yaw = orbit.base_yaw + orbit.yaw;
    let pitch = orbit.base_pitch + orbit.pitch;
    orbit.focus
        + Vec3::new(
            yaw.sin() * pitch.cos(),
            pitch.sin(),
            yaw.cos() * pitch.cos(),
        ) * orbit.distance
}

pub(super) fn clamp_char_select_eye(eye: Vec3, heightmap: Option<&TerrainHeightmap>) -> Vec3 {
    let mut clamped = eye;
    if let Some(terrain_y) = heightmap.and_then(|heightmap| heightmap.height_at(eye.x, eye.z)) {
        clamped.y = clamped
            .y
            .max(terrain_y + CHAR_SELECT_CAMERA_GROUND_CLEARANCE);
    }
    clamped
}

pub(super) fn spawn_char_select_camera(
    commands: &mut Commands,
    scene: Option<&SceneEntry>,
    placement: Option<&ScenePlacement>,
    heightmap: Option<&TerrainHeightmap>,
    presentation: ModelPresentation,
) -> Entity {
    let (eye, focus, fov) = camera_params(scene, placement, presentation);
    let eye = clamp_char_select_eye(eye, heightmap);
    let fog = char_select_fog(eye.distance(focus));
    commands
        .spawn((
            Name::new("CharSelectCamera"),
            CharSelectScene,
            Camera3d::default(),
            additive_particle_glow_tonemapping(),
            Camera {
                clear_color: ClearColorConfig::Custom(CHAR_SELECT_CLEAR_COLOR),
                ..default()
            },
            Projection::Perspective(PerspectiveProjection {
                fov: fov.to_radians(),
                ..default()
            }),
            Transform::from_translation(eye).looking_at(focus, Vec3::Y),
            orbit_from_eye_focus(eye, focus),
            fog,
        ))
        .id()
}

pub(super) fn char_select_orbit_camera(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    options: Res<crate::client_options::CameraOptions>,
    heightmap: Option<Res<TerrainHeightmap>>,
    mut last_debug_state: Local<Option<OrbitInputDebugState>>,
    mut query: Query<(&mut CharSelectOrbit, &mut Transform)>,
) {
    let delta = motion.delta;
    let debug_state = orbit_input_debug_state(
        mouse_buttons.pressed(MouseButton::Left),
        delta,
        query.iter_mut().count(),
    );
    if should_log_orbit_input(*last_debug_state, debug_state) {
        info!(
            left_mouse_pressed = debug_state.left_mouse_pressed,
            has_mouse_motion = debug_state.has_mouse_motion,
            orbit_entity_count = debug_state.orbit_entity_count,
            motion_delta = ?delta,
            "char-select orbit input"
        );
    }
    *last_debug_state = Some(debug_state);
    if !debug_state.left_mouse_pressed {
        return;
    }
    if !debug_state.has_mouse_motion {
        return;
    }
    let orbit_delta = scaled_orbit_delta(delta, options.mouse_sensitivity);
    for (mut orbit, mut transform) in &mut query {
        orbit.yaw = (orbit.yaw + orbit_delta.x).clamp(-ORBIT_YAW_LIMIT, ORBIT_YAW_LIMIT);
        orbit.pitch = (orbit.pitch + orbit_delta.y).clamp(-ORBIT_PITCH_LIMIT, ORBIT_PITCH_LIMIT);
        let eye = clamp_char_select_eye(orbit_eye(&orbit), heightmap.as_deref());
        *transform = Transform::from_translation(eye).looking_at(orbit.focus, Vec3::Y);
    }
}

pub(super) fn update_camera_for_scene(
    scene: &SceneEntry,
    placement: Option<&ScenePlacement>,
    heightmap: Option<&TerrainHeightmap>,
    presentation: ModelPresentation,
    camera_query: &mut Query<
        (&mut Transform, &mut CharSelectOrbit, &mut Projection),
        (With<CharSelectScene>, Without<CharSelectModelRoot>),
    >,
) {
    let (eye, focus, fov) = camera_params(Some(scene), placement, presentation);
    let eye = clamp_char_select_eye(eye, heightmap);
    let orbit = orbit_from_eye_focus(eye, focus);
    for (mut tf, mut orb, mut proj) in camera_query.iter_mut() {
        *tf = Transform::from_translation(eye).looking_at(focus, Vec3::Y);
        *orb = orbit.clone();
        if let Projection::Perspective(ref mut p) = *proj {
            p.fov = fov.to_radians();
        }
    }
}

pub(super) fn orbit_input_debug_state(
    left_mouse_pressed: bool,
    delta: Vec2,
    orbit_entity_count: usize,
) -> OrbitInputDebugState {
    OrbitInputDebugState {
        left_mouse_pressed,
        has_mouse_motion: delta != Vec2::ZERO,
        orbit_entity_count,
    }
}

pub(super) fn should_log_orbit_input(
    previous: Option<OrbitInputDebugState>,
    current: OrbitInputDebugState,
) -> bool {
    current.has_mouse_motion || previous != Some(current)
}
