//! Renderer-free character-creation orbit camera: mouse drag, camera buttons and
//! the eased zoom toward the face while a face customization is open.

use std::f32::consts::{PI, TAU};

use glam::{Vec2, Vec3};

use super::deps::{ModelPresentation, OptionType, char_create_component::CameraControl};

pub const ORBIT_PITCH_LIMIT: f32 = 0.15;
pub const DEFAULT_FOCUS: Vec3 = Vec3::new(0.0, 1.0, 0.0);
pub const DEFAULT_EYE: Vec3 = Vec3::new(0.0, 1.8, 6.0);
pub const FACE_FOCUS: Vec3 = Vec3::new(0.0, 1.55, 0.0);
pub const FACE_DISTANCE: f32 = 2.5;
pub const CAMERA_ZOOM_SPEED: f32 = 5.0;

#[derive(Clone, Debug, PartialEq)]
pub struct CreationOrbit {
    pub yaw: f32,
    pub pitch: f32,
    pub focus: Vec3,
    pub distance: f32,
    pub base_pitch: f32,
    pub manual_distance: Option<f32>,
    pub default_focus: Vec3,
    pub default_distance: f32,
}

impl CreationOrbit {
    /// Orbit over the authored shot, with no user rotation or zoom.
    pub fn new(eye: Vec3, focus: Vec3) -> Self {
        let offset = eye - focus;
        let distance = offset.length();
        Self {
            yaw: 0.0,
            pitch: 0.0,
            focus,
            distance,
            base_pitch: (offset.y / distance).asin(),
            manual_distance: None,
            default_focus: focus,
            default_distance: distance,
        }
    }

    pub fn eye(&self) -> Vec3 {
        let pitch = self.base_pitch + self.pitch;
        self.focus
            + Vec3::new(
                self.yaw.sin() * pitch.cos(),
                pitch.sin(),
                self.yaw.cos() * pitch.cos(),
            ) * self.distance
    }

    /// Apply a scaled mouse-drag orbit delta.
    pub fn drag(&mut self, delta: Vec2) {
        self.yaw = (self.yaw + delta.x).rem_euclid(TAU);
        self.pitch = (self.pitch + delta.y).clamp(-ORBIT_PITCH_LIMIT, ORBIT_PITCH_LIMIT);
    }

    pub fn apply_control(&mut self, control: CameraControl) {
        match control {
            CameraControl::Reset => {
                self.yaw = 0.0;
                self.pitch = 0.0;
                self.manual_distance = None;
            }
            CameraControl::ZoomIn => {
                self.manual_distance =
                    Some((self.manual_distance.unwrap_or(self.distance) - 0.5).max(1.0))
            }
            CameraControl::ZoomOut => {
                self.manual_distance =
                    Some((self.manual_distance.unwrap_or(self.distance) + 0.5).min(10.0))
            }
            CameraControl::RotateLeft => self.yaw = (self.yaw - PI / 12.0).rem_euclid(TAU),
            CameraControl::RotateRight => self.yaw = (self.yaw + PI / 12.0).rem_euclid(TAU),
        }
    }

    /// Ease focus and distance toward the face (open face dropdown) or the
    /// authored shot plus race offset; a manual zoom overrides the distance.
    pub fn ease_toward(
        &mut self,
        open_dropdown: Option<OptionType>,
        presentation: ModelPresentation,
        delta_secs: f32,
    ) {
        let (field_focus, field_distance) = zoom_target_for_dropdown(open_dropdown);
        let (target_focus, target_distance) = if field_focus == FACE_FOCUS {
            (
                field_focus * presentation.customize_scale,
                field_distance * presentation.customize_scale,
            )
        } else {
            (
                self.default_focus,
                self.default_distance + presentation.camera_distance_offset,
            )
        };
        let t = (CAMERA_ZOOM_SPEED * delta_secs).min(1.0);
        self.focus = self.focus.lerp(target_focus, t);
        let distance = self.manual_distance.unwrap_or(target_distance);
        self.distance += (distance - self.distance) * t;
    }
}

pub fn zoom_target_for_dropdown(open_dropdown: Option<OptionType>) -> (Vec3, f32) {
    let is_face_field = open_dropdown.is_some_and(|f| {
        matches!(
            f,
            OptionType::Face
                | OptionType::EyeColor
                | OptionType::HairStyle
                | OptionType::HairColor
                | OptionType::FacialHair
                | OptionType::Ears
                | OptionType::Horns
                | OptionType::Blindfold
        )
    });
    if is_face_field {
        (FACE_FOCUS, FACE_DISTANCE)
    } else {
        (DEFAULT_FOCUS, (DEFAULT_EYE - DEFAULT_FOCUS).length())
    }
}
