//! Reusable orbit camera for debug scenes.
//! LMB drag to rotate, scroll to zoom.

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::prelude::*;

pub struct OrbitCameraPlugin;

impl Plugin for OrbitCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, orbit_camera_system);
    }
}

const ORBIT_SENSITIVITY: f32 = 0.003;
const ORBIT_ZOOM_STEP: f32 = 0.4;
const ORBIT_ZOOM_LERP: f32 = 0.25;

#[derive(Component)]
pub struct OrbitCamera {
    pub yaw: f32,
    pub pitch: f32,
    pub focus: Vec3,
    pub distance: f32,
    pub target_distance: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    pub base_pitch: f32,
}

impl OrbitCamera {
    pub fn new(focus: Vec3, distance: f32) -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            focus,
            distance,
            target_distance: distance,
            min_distance: 0.5,
            max_distance: 20.0,
            base_pitch: 0.15,
        }
    }

    pub fn eye_position(&self) -> Vec3 {
        let pitch = self.base_pitch + self.pitch;
        self.focus
            + Vec3::new(
                self.yaw.sin() * pitch.cos(),
                pitch.sin(),
                self.yaw.cos() * pitch.cos(),
            ) * self.distance
    }
}

pub fn orbit_camera_system(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    mut query: Query<(&mut OrbitCamera, &mut Transform)>,
) {
    for (mut orbit, mut transform) in &mut query {
        if scroll.delta.y != 0.0 {
            orbit.target_distance = (orbit.target_distance - scroll.delta.y * ORBIT_ZOOM_STEP)
                .clamp(orbit.min_distance, orbit.max_distance);
        }
        orbit.distance = orbit.distance.lerp(orbit.target_distance, ORBIT_ZOOM_LERP);
        if mouse_buttons.pressed(MouseButton::Left) && motion.delta != Vec2::ZERO {
            orbit.yaw -= motion.delta.x * ORBIT_SENSITIVITY;
            orbit.pitch += motion.delta.y * ORBIT_SENSITIVITY;
        }
        let eye = orbit.eye_position();
        *transform = Transform::from_translation(eye).looking_at(orbit.focus, Vec3::Y);
    }
}
