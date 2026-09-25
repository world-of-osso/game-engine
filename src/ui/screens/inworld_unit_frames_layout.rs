//! Combat cluster geometry at the 1920×1080 reference: player frame, cast dock and target frame
//! centred above the action bars; target-of-target and focus to the target's right. Party and
//! raid frames (`group_frames_component`) sit left of and above the cluster.

/// Width of the shared resource/cast area between the player and target frames.
pub const CAST_DOCK_W: f32 = 264.0;
pub const CAST_DOCK_GAP: f32 = 16.0;
/// Cluster bottom edge above the bottom of the screen; clears two action bar rows.
pub const CLUSTER_BOTTOM: f32 = 152.0;

/// Player and target frames are the Retail portrait-off art at its authored 133×51 size
/// (`UI-HUD-UnitFrame-Player-PortraitOff`).
pub const FRAME_W: f32 = 133.0;
pub const FRAME_H: f32 = 51.0;
pub const PLAYER_FRAME_LEFT: f32 = -(CAST_DOCK_W / 2.0 + CAST_DOCK_GAP + FRAME_W);
pub const TARGET_FRAME_LEFT: f32 = CAST_DOCK_W / 2.0 + CAST_DOCK_GAP;

pub const SMALL_FRAME_GAP: f32 = 8.0;
/// Target-of-target and focus draw the same art at 3/4 scale.
pub const SMALL_ART_SCALE: f32 = 0.75;
pub const TOT_W: f32 = FRAME_W * SMALL_ART_SCALE;
pub const TOT_H: f32 = FRAME_H * SMALL_ART_SCALE;
pub const TOT_LEFT: f32 = TARGET_FRAME_LEFT + FRAME_W + SMALL_FRAME_GAP;
pub const FOCUS_W: f32 = TOT_W;
pub const FOCUS_H: f32 = TOT_H;
pub const FOCUS_LEFT: f32 = TOT_LEFT + TOT_W + SMALL_FRAME_GAP;
/// Small frames align with the target frame's top edge.
pub const SMALL_FRAME_BOTTOM: f32 = CLUSTER_BOTTOM + FRAME_H - TOT_H;

/// Slots inside the portrait-off art (pixels of the 133×51 crop): name tab on rows 0..12,
/// health slot inside the top border and divider, power slot below the divider.
pub const BAR_X: f32 = 3.0;
pub const BAR_W: f32 = 124.0;
pub(super) const NAME_Y: f32 = 0.0;
pub(super) const NAME_H: f32 = 12.0;
pub(super) const NAME_X: f32 = 7.0;
pub(super) const LEVEL_W: f32 = 28.0;
pub(super) const LEVEL_RIGHT: f32 = 8.0;
pub(super) const HEALTH_Y: f32 = 14.0;
pub(super) const HEALTH_H: f32 = 20.0;
pub(super) const POWER_Y: f32 = 35.0;
pub(super) const POWER_H: f32 = 10.0;
/// Class resource bar hangs under the player frame, overlapping its bottom shadow like
/// Retail's `PlayerFrameBottomManagedFramesContainer`.
pub(super) const CLASS_BAR_Y: f32 = FRAME_H - 4.0;
/// Target auras start below the frame (Retail `TargetFrame` buffs/debuffs).
pub(super) const TARGET_BUFF_Y: f32 = FRAME_H + 2.0;
pub(super) const TARGET_DEBUFF_Y: f32 = TARGET_BUFF_Y + 24.0;

pub(super) const GOLD_TEXT: &str = "1.0,0.82,0.0,1.0";
pub(super) const VALUE_TEXT: &str = "1.0,1.0,1.0,1.0";
pub(super) const UNIT_FONT: &str = "FrizQuadrata";
pub(super) const UNIT_FONT_SIZE: f32 = 10.0;
