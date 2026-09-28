//! In-world camera component; state and direction behavior are shared with Godot.

use bevy::prelude::{Component, Deref, DerefMut};

use crate::camera_control_data::CameraState;
pub use crate::camera_control_data::PITCH_LIMIT_DEGREES;

#[derive(Component, Deref, DerefMut, Default)]
pub struct WowCamera(pub CameraState);
