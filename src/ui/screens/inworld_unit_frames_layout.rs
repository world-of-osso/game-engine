//! Unit frame geometry in UI units. The player and target frames sit at the Retail Modern
//! Edit Mode preset, target-of-target and focus to the target's right; the cast bar is
//! centred above the action bars. Party frames (`group_frames_component`) sit left of the
//! player frame, the raid grid above the action bars.

/// Width of the cast bar area centred above the action bars.
pub const CAST_DOCK_W: f32 = 264.0;
/// Cast bar bottom edge above the bottom of the screen; clears two action bar rows.
pub const CLUSTER_BOTTOM: f32 = 152.0;

/// Player and target frames are the Retail portrait-off art at its authored 133×51 size
/// (`UI-HUD-UnitFrame-Player-PortraitOff`), placed so its health slot is where Retail's
/// 232×100 portrait frames draw their health bars.
pub const FRAME_W: f32 = 133.0;
pub const FRAME_H: f32 = 51.0;
/// Height of the art below the health slot.
const BELOW_HEALTH: f32 = FRAME_H - HEALTH_Y - HEALTH_H;

/// Retail Modern preset PlayerFrame: BOTTOMRIGHT to UIParent BOTTOM at (-300, 250)
/// (Blizzard_EditMode/Mainline/EditModePresetLayouts.lua:231-243), 232×100
/// (Blizzard_UnitFrame/Mainline/PlayerFrame.xml:15). `PlayerFrame_ToPlayerArt` puts its
/// 124×20 health bar TOPLEFT at (85, -41) (PlayerFrame.lua:697).
const RETAIL_PLAYER_RIGHT: f32 = -300.0;
const RETAIL_PLAYER_BOTTOM: f32 = 250.0;
const RETAIL_PLAYER_HEALTH_LEFT: f32 = 85.0 - 232.0;
const RETAIL_PLAYER_HEALTH_BOTTOM: f32 = 100.0 - 41.0 - 20.0;
pub const PLAYER_FRAME_LEFT: f32 = RETAIL_PLAYER_RIGHT + RETAIL_PLAYER_HEALTH_LEFT - BAR_X;
pub const PLAYER_FRAME_BOTTOM: f32 =
    RETAIL_PLAYER_BOTTOM + RETAIL_PLAYER_HEALTH_BOTTOM - BELOW_HEALTH;

/// Retail Modern preset TargetFrame: BOTTOMLEFT to UIParent BOTTOM at (300, 250)
/// (EditModePresetLayouts.lua:245-257), 232×100 (TargetFrame.xml:144). For a normal or
/// elite unit `CheckClassification` moves its 126×20 health bar's BOTTOMRIGHT to the
/// frame's LEFT + (149, -10) (TargetFrame.lua:417-419), overriding the XML's (148, 2).
const RETAIL_TARGET_LEFT: f32 = 300.0;
const RETAIL_TARGET_BOTTOM: f32 = 250.0;
const RETAIL_TARGET_HEALTH_LEFT: f32 = 149.0 - 126.0;
const RETAIL_TARGET_HEALTH_BOTTOM: f32 = 100.0 / 2.0 - 10.0;
pub const TARGET_FRAME_LEFT: f32 = RETAIL_TARGET_LEFT + RETAIL_TARGET_HEALTH_LEFT - BAR_X;
pub const TARGET_FRAME_BOTTOM: f32 =
    RETAIL_TARGET_BOTTOM + RETAIL_TARGET_HEALTH_BOTTOM - BELOW_HEALTH;

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
/// Retail `PlayerFrameBottomManagedFramesContainer` hangs from the 232×100 player frame's
/// BOTTOM at (30, 25) (PlayerFrame.lua:758), whose mana bar is TOPLEFT (85, -61), 124×10
/// (PlayerFrame.lua:716): its top is 4 px below the mana bar and its centre 61 px right of
/// the bar's left edge. Its class bars are `align="center"` (PlayerFrameTemplates.xml:7).
pub(super) const CLASS_BAR_TOP: f32 = POWER_Y + POWER_H + 4.0;
pub(super) const CLASS_BAR_CENTRE_X: f32 = BAR_X + 61.0;
/// Retail TargetFrame aura container: TOPLEFT at the 192×67 `FrameTexture`'s BOTTOMLEFT
/// + (5, 9) (TargetFrame.lua:4-6,547-553), the texture centred in the 232×100 frame
/// (TargetFrame.xml:54,79-83; UiTextureAtlasMember 16118). Placed from the health slot the
/// two frames share: Retail's frame left is `RETAIL_TARGET_HEALTH_LEFT` left of it and its
/// bottom `RETAIL_TARGET_HEALTH_BOTTOM` below the health bar's bottom.
const RETAIL_FRAME_TEXTURE: (f32, f32) = (192.0, 67.0);
const AURA_START: (f32, f32) = (5.0, 9.0);
pub(super) const TARGET_AURAS_LEFT: f32 =
    BAR_X - RETAIL_TARGET_HEALTH_LEFT + (232.0 - RETAIL_FRAME_TEXTURE.0) / 2.0 + AURA_START.0;
pub(super) const TARGET_AURAS_TOP: f32 = HEALTH_Y + HEALTH_H + RETAIL_TARGET_HEALTH_BOTTOM
    - (100.0 - RETAIL_FRAME_TEXTURE.1) / 2.0
    - AURA_START.1;

/// Retail-frame points mapped into the portrait-off art through the health slot both share:
/// Retail's 126×20 health bar's top-left is at (`RETAIL_TARGET_HEALTH_LEFT`, 100 −
/// `RETAIL_TARGET_HEALTH_BOTTOM` − 20) of its 232×100 frame.
const RETAIL_TARGET_TO_ART: (f32, f32) = (
    BAR_X - RETAIL_TARGET_HEALTH_LEFT,
    HEALTH_Y - (100.0 - RETAIL_TARGET_HEALTH_BOTTOM - HEALTH_H),
);
/// `BossPortraitFrameTexture` (80×79) for elites and rare elites: TOPRIGHT at (-11, -8) of
/// the 232×100 frame (TargetFrame.lua:436-443, TargetFrame.xml:93-97).
pub(super) const TARGET_BOSS_PORTRAIT: (f32, f32) = (
    232.0 - 11.0 - 80.0 + RETAIL_TARGET_TO_ART.0,
    8.0 + RETAIL_TARGET_TO_ART.1,
);
/// `BossIcon` star centred on the 58×58 `Portrait`'s BOTTOM, the portrait TOPRIGHT at
/// (-26, -19) (TargetFrame.xml:62-66,281-284).
pub(super) const TARGET_BOSS_ICON_CENTRE: (f32, f32) = (
    232.0 - 26.0 - 58.0 / 2.0 + RETAIL_TARGET_TO_ART.0,
    19.0 + 58.0 + RETAIL_TARGET_TO_ART.1,
);
/// `RaidTargetIcon` centred on the 58×58 `Portrait`'s TOP (TargetFrame.xml:275-279).
pub(super) const TARGET_RAID_TARGET_ICON_CENTRE: (f32, f32) = (
    232.0 - 26.0 - 58.0 / 2.0 + RETAIL_TARGET_TO_ART.0,
    19.0 + RETAIL_TARGET_TO_ART.1,
);

pub(super) const GOLD_TEXT: &str = "1.0,0.82,0.0,1.0";
pub(super) const VALUE_TEXT: &str = "1.0,1.0,1.0,1.0";
pub(super) const UNIT_FONT: &str = "FrizQuadrata";
pub(super) const UNIT_FONT_SIZE: f32 = 10.0;
