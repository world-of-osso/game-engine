//! Unit frame geometry in UI units. The player and target frames sit at the Retail Modern
//! Edit Mode preset, target-of-target and focus to the target's right; the cast bar is
//! centred above the action bars. Party frames (`group_frames_component`) sit left of the
//! player frame, the raid grid above the action bars.

/// Width of the cast bar area centred above the action bars.
pub const CAST_DOCK_W: f32 = 264.0;
/// Cast bar bottom edge above the bottom of the screen; clears two action bar rows.
pub const CLUSTER_BOTTOM: f32 = 152.0;

/// Rect `(x, y, width, height)` from the parent's top-left.
pub type Rect = (f32, f32, f32, f32);

/// Retail PlayerFrame and TargetFrame: 232×100 (Blizzard_UnitFrame/Mainline/PlayerFrame.xml:15,
/// TargetFrame.xml:55) with their portrait-on `FrameTexture` centred (PlayerFrame.xml:49-53,
/// TargetFrame.xml:79-83).
pub const UNIT_FRAME_W: f32 = 232.0;
pub const UNIT_FRAME_H: f32 = 100.0;

/// Retail Modern preset PlayerFrame: BOTTOMRIGHT to UIParent BOTTOM at (-300, 250)
/// (Blizzard_EditMode/Mainline/EditModePresetLayouts.lua:231-243).
pub const PLAYER_FRAME_LEFT: f32 = -300.0 - UNIT_FRAME_W;
pub const PLAYER_FRAME_BOTTOM: f32 = 250.0;
/// Retail Modern preset TargetFrame: BOTTOMLEFT to UIParent BOTTOM at (300, 250)
/// (EditModePresetLayouts.lua:245-257).
pub const TARGET_FRAME_LEFT: f32 = 300.0;
pub const TARGET_FRAME_BOTTOM: f32 = 250.0;

/// A unit frame's text and bar slots, in its own top-left pixels.
#[derive(Clone, Copy)]
pub(super) struct FrameSlots {
    pub(super) name: Rect,
    pub(super) level: Rect,
    pub(super) level_justify: &'static str,
    pub(super) health: Rect,
    pub(super) power: Rect,
}

/// The portrait-off art (`UI-HUD-UnitFrame-Player-PortraitOff`, 133×51) boss,
/// target-of-target and focus frames draw: name tab on rows 0..12, health slot inside
/// the top border and divider, power slot below the divider.
pub const FRAME_W: f32 = 133.0;
pub const FRAME_H: f32 = 51.0;
pub const BAR_X: f32 = 3.0;
pub const BAR_W: f32 = 124.0;
pub(super) const NAME_Y: f32 = 0.0;
pub(super) const NAME_H: f32 = 12.0;
pub(super) const LEVEL_W: f32 = 28.0;
pub(super) const PORTRAIT_OFF_SLOTS: FrameSlots = FrameSlots {
    name: (7.0, NAME_Y, BAR_W - LEVEL_W, NAME_H),
    level: (FRAME_W - 8.0 - LEVEL_W, NAME_Y, LEVEL_W, NAME_H),
    level_justify: "RIGHT",
    health: (BAR_X, 14.0, BAR_W, 20.0),
    power: (BAR_X, 35.0, BAR_W, 10.0),
};

/// PlayerFrame (PlayerFrame.xml:73-83; `PlayerFrame_ToPlayerArt`, PlayerFrame.lua:698,717):
/// `PlayerName` 96×12 TOPLEFT (88, -27), `PlayerLevelText` TOPRIGHT (-24.5, -28), the
/// 124×20 health bar TOPLEFT (85, -41) and 124×10 mana bar TOPLEFT (85, -61).
pub(super) const PLAYER_SLOTS: FrameSlots = FrameSlots {
    name: (88.0, 27.0, 96.0, NAME_H),
    level: (UNIT_FRAME_W - 24.5 - LEVEL_W, 28.0, LEVEL_W, NAME_H),
    level_justify: "RIGHT",
    health: (85.0, 41.0, 124.0, 20.0),
    power: (85.0, 61.0, 124.0, 10.0),
};

/// TargetFrame `ReputationColor` (135×18) TOPRIGHT (-75, -25) (TargetFrame.xml:100-104).
pub(super) const TARGET_REPUTATION: (f32, f32) = (UNIT_FRAME_W - 75.0 - 135.0, 25.0);
/// TargetFrame (TargetFrame.xml:107-117,212-215): `Name` 90×12 and `LevelText` from the
/// `ReputationColor`'s TOPRIGHT at (-106, -1) and (-133, -2); for a normal, rare or elite
/// unit `CheckClassification` puts the 126×20 health bar's BOTTOMRIGHT at the frame's
/// LEFT + (149, -10) (TargetFrame.lua:417-419); the 134×10 mana bar's TOPRIGHT sits at
/// its BOTTOMRIGHT + (8, -1).
pub(super) const TARGET_SLOTS: FrameSlots = FrameSlots {
    name: (TARGET_REPUTATION.0 + 135.0 - 106.0, 26.0, 90.0, NAME_H),
    level: (TARGET_REPUTATION.0 + 135.0 - 133.0, 27.0, LEVEL_W, NAME_H),
    level_justify: "LEFT",
    health: (149.0 - 126.0, 60.0 - 20.0, 126.0, 20.0),
    power: (149.0 + 8.0 - 134.0, 61.0, 134.0, 10.0),
};

/// A unit portrait (`SetPortraitTexture`, UnitFrame.lua:188) and the mask that rounds it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PortraitSlot {
    pub frame: &'static str,
    pub rect: Rect,
    /// Mask texture FileDataID: its alpha keeps the portrait; outside the mask rect is
    /// clear (`CLAMPTOBLACKADDITIVE`).
    pub mask_fdid: u32,
    pub mask_rect: Rect,
}

/// `PlayerPortrait` 60×60 TOPLEFT (24, -19) under `PlayerPortraitMask`
/// (`UI-HUD-UnitFrame-Player-Portrait-Mask`, atlas 2088
/// `interface/hud/uiunitframeplayerportraitmask.blp`) at the same rect
/// (PlayerFrame.xml:27-42).
pub const PLAYER_PORTRAIT: PortraitSlot = PortraitSlot {
    frame: "PlayerPortrait",
    rect: (24.0, 19.0, 60.0, 60.0),
    mask_fdid: 4_682_541,
    mask_rect: (24.0, 19.0, 60.0, 60.0),
};

/// TargetFrame `Portrait` 58×58 TOPRIGHT (-26, -19) under `PortraitMask` (`CircleMask`,
/// atlas 1582 `interface/masks/circlemask.blp`) from its TOPLEFT (0, -1) to its
/// BOTTOMRIGHT (-1, 0) (TargetFrame.xml:58-74).
pub const TARGET_PORTRAIT: PortraitSlot = PortraitSlot {
    frame: "TargetFramePortrait",
    rect: (UNIT_FRAME_W - 26.0 - 58.0, 19.0, 58.0, 58.0),
    mask_fdid: 3_528_314,
    mask_rect: (UNIT_FRAME_W - 26.0 - 58.0, 20.0, 57.0, 57.0),
};

pub const SMALL_FRAME_GAP: f32 = 8.0;
/// Target-of-target and focus draw the portrait-off art at 3/4 scale.
pub const SMALL_ART_SCALE: f32 = 0.75;
pub const TOT_W: f32 = FRAME_W * SMALL_ART_SCALE;
pub const TOT_H: f32 = FRAME_H * SMALL_ART_SCALE;
/// Right of the whole 232-wide TargetFrame, clear of its portrait and classification art.
pub const TOT_LEFT: f32 = TARGET_FRAME_LEFT + UNIT_FRAME_W + SMALL_FRAME_GAP;
pub const FOCUS_W: f32 = TOT_W;
pub const FOCUS_H: f32 = TOT_H;
pub const FOCUS_LEFT: f32 = TOT_LEFT + TOT_W + SMALL_FRAME_GAP;
/// Small frames align with the top of the target's 192×67 `FrameTexture`.
pub const SMALL_FRAME_BOTTOM: f32 =
    TARGET_FRAME_BOTTOM + UNIT_FRAME_H - (UNIT_FRAME_H - 67.0) / 2.0 - TOT_H;

/// Retail `PlayerFrameBottomManagedFramesContainer` hangs from the player frame's BOTTOM at
/// (30, 25) (PlayerFrame.lua:758): its top is 4 px below the mana bar and its centre 61 px
/// right of the bar's left edge. Its class bars are `align="center"`
/// (PlayerFrameTemplates.xml:7).
pub(super) const CLASS_BAR_TOP: f32 = PLAYER_SLOTS.power.1 + PLAYER_SLOTS.power.3 + 4.0;
pub(super) const CLASS_BAR_CENTRE_X: f32 = PLAYER_SLOTS.power.0 + 61.0;
/// `AttackIcon` TOPLEFT (64, -62) at its 16×16 atlas size; `PlayerRestLoop` 20×20 TOPLEFT
/// (64, -6) with its 30×30 `RestTexture` centred (PlayerFrame.xml:329-333,380-392).
pub(super) const PLAYER_ATTACK_ICON: (f32, f32) = (64.0, 62.0);
pub(super) const PLAYER_REST_ICON: Rect = (64.0 + 10.0 - 15.0, 6.0 + 10.0 - 15.0, 30.0, 30.0);
/// Retail TargetFrame aura container: TOPLEFT at the 192×67 `FrameTexture`'s BOTTOMLEFT
/// + (5, 9) (TargetFrame.lua:4-6,547-553), the texture centred in the frame
/// (TargetFrame.xml:79-83; UiTextureAtlasMember 16118).
pub(super) const TARGET_AURAS_LEFT: f32 = (UNIT_FRAME_W - 192.0) / 2.0 + 5.0;
pub(super) const TARGET_AURAS_TOP: f32 = UNIT_FRAME_H - (UNIT_FRAME_H - 67.0) / 2.0 - 9.0;

/// `BossPortraitFrameTexture` (80×79) for elites and rare elites: TOPRIGHT at (-11, -8)
/// (TargetFrame.lua:436-443, TargetFrame.xml:93-97).
pub(super) const TARGET_BOSS_PORTRAIT: (f32, f32) = (UNIT_FRAME_W - 11.0 - 80.0, 8.0);
/// `BossIcon` star centred on the `Portrait`'s BOTTOM (TargetFrame.xml:281-284).
pub(super) const TARGET_BOSS_ICON_CENTRE: (f32, f32) = (
    TARGET_PORTRAIT.rect.0 + TARGET_PORTRAIT.rect.2 / 2.0,
    TARGET_PORTRAIT.rect.1 + TARGET_PORTRAIT.rect.3,
);

pub(super) const GOLD_TEXT: &str = "1.0,0.82,0.0,1.0";
pub(super) const VALUE_TEXT: &str = "1.0,1.0,1.0,1.0";
pub(super) const UNIT_FONT: &str = "FrizQuadrata";
pub(super) const UNIT_FONT_SIZE: f32 = 10.0;
