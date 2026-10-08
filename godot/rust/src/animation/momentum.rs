//! Momentum sway: a damped spring that swings the arms, spine and head against the
//! unit's change in horizontal velocity, on top of the finished local pose.
//! Retail has no such layer (it only crossfades movement clips); see
//! docs/specs/momentum-sway.md.
use super::BonePose;
use game_engine_core::m2;
use godot::builtin::{Vector2, Vector3};

/// Largest swing of any bone (12°).
pub(super) const MAX_SWING: f32 = 12.0 * std::f32::consts::PI / 180.0;

/// The spring's swing of the bone tips in the character's horizontal plane
/// (x: model +X forward, y: model +Z lateral), radians, and its rate.
pub(super) struct MomentumSway {
    pub(super) angle: Vector2,
    pub(super) rate: Vector2,
}

impl MomentumSway {
    pub(super) fn new(_model: &m2::Model) -> Self {
        Self {
            angle: Vector2::ZERO,
            rate: Vector2::ZERO,
        }
    }

    pub(super) fn kick(&mut self, _velocity_change: Vector3) {}

    pub(super) fn advance(&mut self, _delta_ms: f64) {}

    pub(super) fn active(&self) -> bool {
        false
    }

    pub(super) fn apply(&self, poses: Vec<BonePose>) -> Vec<BonePose> {
        poses
    }
}
