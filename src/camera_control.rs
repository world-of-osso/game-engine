//! In-world camera state shared by input, camera follow, and IPC.

use bevy::prelude::Component;

pub const PITCH_LIMIT_DEGREES: f32 = 88.0;

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

impl WowCamera {
    /// Set absolute angles without changing omitted axes, zoom, or player facing.
    pub fn set_direction_degrees(
        &mut self,
        yaw: Option<f32>,
        pitch: Option<f32>,
    ) -> Result<(), String> {
        if yaw.is_none() && pitch.is_none() {
            return Err("provide yaw or pitch in degrees".into());
        }
        if yaw.is_some_and(|angle| !angle.is_finite())
            || pitch.is_some_and(|angle| !angle.is_finite())
        {
            return Err("camera angles must be finite".into());
        }
        if pitch.is_some_and(|angle| angle.abs() > PITCH_LIMIT_DEGREES) {
            return Err(format!(
                "camera pitch must be between -{PITCH_LIMIT_DEGREES} and {PITCH_LIMIT_DEGREES} degrees"
            ));
        }
        if let Some(yaw) = yaw {
            self.yaw = yaw.to_radians();
        }
        if let Some(pitch) = pitch {
            self.pitch = pitch.to_radians();
        }
        Ok(())
    }
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
