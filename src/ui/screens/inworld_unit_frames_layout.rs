//! Combat cluster geometry at the 1920×1080 reference: player frame, cast dock and target frame
//! centred above the action bars; target-of-target and focus to the target's right; party
//! frames left of the player frame, above chat.

/// Width of the shared resource/cast area between the player and target frames.
pub const CAST_DOCK_W: f32 = 264.0;
pub const CAST_DOCK_GAP: f32 = 16.0;
/// Cluster bottom edge above the bottom of the screen; clears two action bar rows.
pub const CLUSTER_BOTTOM: f32 = 152.0;

pub const FRAME_W: f32 = 232.0;
pub const FRAME_H: f32 = 60.0;
pub const PLAYER_FRAME_LEFT: f32 = -(CAST_DOCK_W / 2.0 + CAST_DOCK_GAP + FRAME_W);
pub const TARGET_FRAME_LEFT: f32 = CAST_DOCK_W / 2.0 + CAST_DOCK_GAP;

pub const SMALL_FRAME_GAP: f32 = 8.0;
pub const TOT_W: f32 = 120.0;
pub const TOT_H: f32 = 32.0;
pub const TOT_LEFT: f32 = TARGET_FRAME_LEFT + FRAME_W + SMALL_FRAME_GAP;
pub const FOCUS_W: f32 = 150.0;
pub const FOCUS_H: f32 = 32.0;
pub const FOCUS_LEFT: f32 = TOT_LEFT + TOT_W + SMALL_FRAME_GAP;
/// Small frames align with the target frame's top edge.
pub const SMALL_FRAME_BOTTOM: f32 = CLUSTER_BOTTOM + FRAME_H - TOT_H;

/// Party frames: right edge `PARTY_GAP` left of the player frame, bottom raised above chat.
pub const PARTY_GAP: f32 = 12.0;
pub const PARTY_BOTTOM: f32 = 232.0;

pub(super) const BORDER: f32 = 1.0;
pub const BAR_X: f32 = 4.0;
pub const BAR_W: f32 = FRAME_W - 2.0 * BAR_X;
pub(super) const NAME_Y: f32 = 4.0;
pub(super) const NAME_H: f32 = 12.0;
pub(super) const LEVEL_W: f32 = 28.0;
pub(super) const HEALTH_Y: f32 = 18.0;
pub(super) const HEALTH_H: f32 = 20.0;
pub(super) const POWER_Y: f32 = 40.0;
pub(super) const POWER_H: f32 = 10.0;
pub(super) const PIPS_Y: f32 = 52.0;
pub(super) const PIPS_H: f32 = 5.0;
pub(super) const PIP_GAP: f32 = 2.0;
pub(super) const SMALL_NAME_Y: f32 = 3.0;
pub(super) const SMALL_BAR_Y: f32 = 17.0;
pub(super) const SMALL_BAR_H: f32 = 11.0;

pub(super) const METAL_BORDER: &str = "0.50,0.45,0.36,0.95";
pub(super) const DARK_BACKING: &str = "0.03,0.03,0.04,0.88";
pub(super) const BAR_BG: &str = "0.06,0.06,0.07,0.95";
pub(super) const GOLD_TEXT: &str = "1.0,0.82,0.0,1.0";
pub(super) const NAME_TEXT: &str = "0.98,0.95,0.90,1.0";
pub(super) const VALUE_TEXT: &str = "1.0,1.0,1.0,0.95";
pub(super) const UNIT_FONT: &str = "FrizQuadrata";
pub(super) const UNIT_FONT_SIZE: f32 = 10.0;
