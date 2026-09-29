//! Unit frame geometry in UI units. The player frame and cast dock form the central cluster
//! above the action bars; the target frame sits at the Retail Modern Edit Mode preset, with
//! target-of-target and focus to its right. Party and raid frames (`group_frames_component`)
//! sit left of and above the player frame.

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

/// Retail Modern preset TargetFrame: BOTTOMLEFT to UIParent BOTTOM at (300, 250)
/// (Blizzard_EditMode/Mainline/EditModePresetLayouts.lua:245-257).
const RETAIL_TARGET_LEFT: f32 = 300.0;
const RETAIL_TARGET_BOTTOM: f32 = 250.0;
/// The Retail frame is 232×100 around a portrait (TargetFrame.xml:144); its 126×20 health
/// bar has its BOTTOMRIGHT at the frame's LEFT + (148, 2) (TargetFrame.xml:218-220).
const RETAIL_HEALTH_LEFT: f32 = 148.0 - 126.0;
const RETAIL_HEALTH_BOTTOM: f32 = 100.0 / 2.0 + 2.0;
/// The portrait-off frame is placed so its health slot is where Retail draws the health bar.
pub const TARGET_FRAME_LEFT: f32 = RETAIL_TARGET_LEFT + RETAIL_HEALTH_LEFT - BAR_X;
pub const TARGET_FRAME_BOTTOM: f32 =
    RETAIL_TARGET_BOTTOM + RETAIL_HEALTH_BOTTOM - (FRAME_H - HEALTH_Y - HEALTH_H);

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
pub const SMALL_FRAME_BOTTOM: f32 = TARGET_FRAME_BOTTOM + FRAME_H - TOT_H;

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
/// Retail TargetFrame aura container: TOPLEFT at the 192×67 `FrameTexture`'s BOTTOMLEFT
/// + (5, 9) (TargetFrame.lua:4-6,547-553), the texture centred in the 232×100 frame
/// (TargetFrame.xml:54,79-83; UiTextureAtlasMember 16118). Placed from the health slot the
/// two frames share: Retail's frame left is `RETAIL_HEALTH_LEFT` left of it and its bottom
/// `RETAIL_HEALTH_BOTTOM` below the health bar's bottom.
const RETAIL_FRAME_TEXTURE: (f32, f32) = (192.0, 67.0);
const AURA_START: (f32, f32) = (5.0, 9.0);
pub(super) const TARGET_AURAS_LEFT: f32 =
    BAR_X - RETAIL_HEALTH_LEFT + (232.0 - RETAIL_FRAME_TEXTURE.0) / 2.0 + AURA_START.0;
pub(super) const TARGET_AURAS_TOP: f32 = HEALTH_Y + HEALTH_H + RETAIL_HEALTH_BOTTOM
    - (100.0 - RETAIL_FRAME_TEXTURE.1) / 2.0
    - AURA_START.1;

pub(super) const GOLD_TEXT: &str = "1.0,0.82,0.0,1.0";
pub(super) const VALUE_TEXT: &str = "1.0,1.0,1.0,1.0";
pub(super) const UNIT_FONT: &str = "FrizQuadrata";
pub(super) const UNIT_FONT_SIZE: f32 = 10.0;
