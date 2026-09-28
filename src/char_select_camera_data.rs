//! Pure solo character-selection camera framing, using Bevy-space positions.

use std::f32::consts::FRAC_PI_8;

use glam::{Vec2, Vec3};

use crate::customization_data::ModelPresentation;

const SOLO_CHARACTER_CAMERA_DISTANCE: f32 = 6.5;
const SOLO_CHARACTER_MIN_DISTANCE: f32 = 3.5;
const SOLO_CHARACTER_MAX_FOV_DEGREES: f32 = 55.0;
const ORBIT_YAW_LIMIT: f32 = FRAC_PI_8;
const ORBIT_PITCH_LIMIT: f32 = 0.15;

/// Mouse motion to orbit yaw/pitch radians: dragging right orbits left.
pub fn scaled_orbit_delta(delta: Vec2, sensitivity: f32) -> Vec2 {
    Vec2::new(-delta.x * sensitivity, delta.y * sensitivity)
}

/// Left-drag orbit around the framed character, bounded near the authored shot.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectOrbit {
    pub yaw: f32,
    pub base_yaw: f32,
    pub pitch: f32,
    pub focus: Vec3,
    pub distance: f32,
    pub base_pitch: f32,
}

impl SelectOrbit {
    pub fn from_eye_focus(eye: Vec3, focus: Vec3) -> Self {
        let offset = eye - focus;
        let distance = offset.length();
        let base_pitch = if distance > 0.0 {
            (offset.y / distance).asin()
        } else {
            0.0
        };
        Self {
            yaw: 0.0,
            base_yaw: offset.x.atan2(offset.z),
            pitch: 0.0,
            focus,
            distance,
            base_pitch,
        }
    }

    pub fn eye(&self) -> Vec3 {
        let yaw = self.base_yaw + self.yaw;
        let pitch = self.base_pitch + self.pitch;
        self.focus
            + Vec3::new(
                yaw.sin() * pitch.cos(),
                pitch.sin(),
                yaw.cos() * pitch.cos(),
            ) * self.distance
    }

    /// Apply a `scaled_orbit_delta`.
    pub fn drag(&mut self, delta: Vec2) {
        self.yaw = (self.yaw + delta.x).clamp(-ORBIT_YAW_LIMIT, ORBIT_YAW_LIMIT);
        self.pitch = (self.pitch + delta.y).clamp(-ORBIT_PITCH_LIMIT, ORBIT_PITCH_LIMIT);
    }
}

/// Reframe the authored scene camera around one character while retaining its vertical lift.
pub fn solo_camera_params(
    scene_eye: Vec3,
    scene_focus: Vec3,
    scene_fov: f32,
    character_position: Vec3,
    presentation: ModelPresentation,
) -> (Vec3, Vec3, f32) {
    let focus_y = character_position.y + presentation.customize_scale.max(0.01);
    let focus = Vec3::new(character_position.x, focus_y, character_position.z);
    let scene_offset = scene_eye - scene_focus;
    let distance = (SOLO_CHARACTER_CAMERA_DISTANCE + presentation.camera_distance_offset)
        .clamp(SOLO_CHARACTER_MIN_DISTANCE, scene_offset.length());
    let vertical = scene_offset.y;
    let horizontal = Vec3::new(scene_offset.x, 0.0, scene_offset.z);
    let horizontal_dir = horizontal.normalize_or_zero();
    let horizontal_distance = (distance * distance - vertical * vertical).max(0.0).sqrt();
    let eye = focus + horizontal_dir * horizontal_distance + Vec3::Y * vertical;
    (eye, focus, scene_fov.min(SOLO_CHARACTER_MAX_FOV_DEGREES))
}
