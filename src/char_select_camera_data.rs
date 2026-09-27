//! Pure solo character-selection camera framing, using Bevy-space positions.

use glam::Vec3;

use crate::customization_data::ModelPresentation;

const SOLO_CHARACTER_CAMERA_DISTANCE: f32 = 6.5;
const SOLO_CHARACTER_MIN_DISTANCE: f32 = 3.5;
const SOLO_CHARACTER_MAX_FOV_DEGREES: f32 = 55.0;

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
