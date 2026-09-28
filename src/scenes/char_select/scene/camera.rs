use bevy::camera::ClearColorConfig;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use game_engine::char_select_camera_data::{SelectOrbit, solo_camera_params};
use game_engine::customization_data::ModelPresentation;

use crate::camera::world_camera_tonemapping;
use crate::orbit_camera::scaled_orbit_delta;
use crate::terrain_heightmap::TerrainHeightmap;

use super::{CharSelectModelRoot, CharSelectScene};

pub(super) type SceneEntry = crate::scenes::char_select::warband::WarbandSceneEntry;
pub(super) type ScenePlacement = crate::scenes::char_select::warband::WarbandScenePlacement;

#[derive(Component, Clone, Deref, DerefMut)]
pub(super) struct CharSelectOrbit(pub(super) SelectOrbit);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct OrbitInputDebugState {
    pub(super) left_mouse_pressed: bool,
    pub(super) has_mouse_motion: bool,
    pub(super) orbit_entity_count: usize,
}

pub(super) const CHAR_SELECT_CAMERA_GROUND_CLEARANCE: f32 = 0.5;
// Keep the nearby campsite clear; fog uses world distance, not portrait framing.
const CHAR_SELECT_FOG_START_DISTANCE: f32 = 75.0;
const CHAR_SELECT_FOG_END_DISTANCE: f32 = 300.0;
const CHAR_SELECT_CLEAR_COLOR: Color = Color::srgb(0.05, 0.06, 0.08);
// Reference-guided haze preserves distant detail instead of becoming a solid wall.
const CHAR_SELECT_FOG_COLOR: Color = Color::srgba(0.30, 0.42, 0.42, 0.5);
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

pub(super) fn char_select_fog() -> DistanceFog {
    DistanceFog {
        color: CHAR_SELECT_FOG_COLOR,
        directional_light_color: CHAR_SELECT_FOG_LIGHT_COLOR,
        directional_light_exponent: 8.0,
        falloff: FogFalloff::Linear {
            start: CHAR_SELECT_FOG_START_DISTANCE,
            end: CHAR_SELECT_FOG_END_DISTANCE,
        },
    }
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
            solo_camera_params(
                scene_eye,
                scene_focus,
                scene.fov,
                placement.bevy_position(),
                presentation,
            )
        }
        CameraTarget::Scene(scene) => (scene.bevy_position(), scene.bevy_look_at(), scene.fov),
        CameraTarget::Default => (
            DEFAULT_CAMERA_EYE,
            DEFAULT_CAMERA_FOCUS,
            DEFAULT_CAMERA_FOV_DEGREES,
        ),
    }
}

pub(super) fn orbit_from_eye_focus(eye: Vec3, focus: Vec3) -> CharSelectOrbit {
    CharSelectOrbit(SelectOrbit::from_eye_focus(eye, focus))
}

pub(super) fn orbit_eye(orbit: &CharSelectOrbit) -> Vec3 {
    orbit.eye()
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
    let fog = char_select_fog();
    commands
        .spawn((
            Name::new("CharSelectCamera"),
            CharSelectScene,
            Camera3d::default(),
            world_camera_tonemapping(),
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
        debug!(
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
        orbit.drag(orbit_delta);
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
