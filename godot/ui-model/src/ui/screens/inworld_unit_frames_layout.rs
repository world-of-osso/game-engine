//! Unit frame geometry in UI units. Where each frame sits comes from the active preset
//! (`crate::hud_layout`).

/// Width of the cast bar area.
pub const CAST_DOCK_W: f32 = 264.0;

/// Rect `(x, y, width, height)` from the parent's top-left.
pub type Rect = (f32, f32, f32, f32);

/// Retail PlayerFrame and TargetFrame: 232×100 (Blizzard_UnitFrame/Mainline/PlayerFrame.xml:15,
/// TargetFrame.xml:55) with their portrait-on `FrameTexture` centred (PlayerFrame.xml:49-53,
/// TargetFrame.xml:79-83).
pub const UNIT_FRAME_W: f32 = 232.0;
pub const UNIT_FRAME_H: f32 = 100.0;

/// A unit frame's text and bar slots, in its own top-left pixels.
#[derive(Clone, Copy)]
pub(super) struct FrameSlots {
    pub(super) name: Rect,
    pub(super) level: Rect,
    pub(super) level_justify: &'static str,
    pub(super) health: Rect,
    pub(super) power: Rect,
    pub(super) health_text: TextAnchors,
    pub(super) power_text: TextAnchors,
}

/// X offsets of a bar's `TextString` (CENTER), `LeftText` (LEFT) and `RightText` (RIGHT)
/// anchors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct TextAnchors {
    pub(super) center: f32,
    pub(super) left: f32,
    pub(super) right: f32,
}

impl TextAnchors {
    pub(super) const fn new(center: f32, left: f32, right: f32) -> Self {
        Self {
            center,
            left,
            right,
        }
    }
}

/// `TextStatusBar` default anchors of PlayerFrame bars (PlayerFrame.xml:200-214,260-274).
const PLAYER_BAR_TEXT: TextAnchors = TextAnchors::new(0.0, 2.0, -2.0);
/// TargetFrame health (TargetFrame.xml:167-181); boss frames keep it and move their mana
/// `RightText` to RIGHT (-5, 0) (TargetFrame.lua:995-996).
const TARGET_HEALTH_TEXT: TextAnchors = TextAnchors::new(0.0, 2.0, -5.0);

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
    health_text: TARGET_HEALTH_TEXT,
    power_text: TARGET_HEALTH_TEXT,
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
    health_text: PLAYER_BAR_TEXT,
    power_text: PLAYER_BAR_TEXT,
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
    health_text: TARGET_HEALTH_TEXT,
    // TargetFrame mana (TargetFrame.xml:220-234).
    power_text: TextAnchors::new(-4.0, 2.0, -13.0),
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
    /// `SetTexCoord` of the square portrait image: `[left, right, top, bottom]`.
    pub tex_coords: [f32; 4],
}

/// The whole portrait image: unit frames set no texture coordinates.
pub const FULL_PORTRAIT: [f32; 4] = [0.0, 1.0, 0.0, 1.0];

/// `PlayerPortrait` 60×60 TOPLEFT (24, -19) under `PlayerPortraitMask`
/// (`UI-HUD-UnitFrame-Player-Portrait-Mask`, atlas 2088
/// `interface/hud/uiunitframeplayerportraitmask.blp`) at the same rect
/// (PlayerFrame.xml:27-42).
pub const PLAYER_PORTRAIT: PortraitSlot = PortraitSlot {
    frame: "PlayerPortrait",
    rect: (24.0, 19.0, 60.0, 60.0),
    mask_fdid: 4_682_541,
    mask_rect: (24.0, 19.0, 60.0, 60.0),
    tex_coords: FULL_PORTRAIT,
};

/// TargetFrame `Portrait` 58×58 TOPRIGHT (-26, -19) under `PortraitMask` (`CircleMask`,
/// atlas 1582 `interface/masks/circlemask.blp`) from its TOPLEFT (0, -1) to its
/// BOTTOMRIGHT (-1, 0) (TargetFrame.xml:58-74).
pub const TARGET_PORTRAIT: PortraitSlot = PortraitSlot {
    frame: "TargetFramePortrait",
    rect: (UNIT_FRAME_W - 26.0 - 58.0, 19.0, 58.0, 58.0),
    mask_fdid: 3_528_314,
    mask_rect: (UNIT_FRAME_W - 26.0 - 58.0, 20.0, 57.0, 57.0),
    tex_coords: FULL_PORTRAIT,
};

/// Encounter portraits use target art and classification, with a unique native host per slot.
pub const BOSS_PORTRAITS: [PortraitSlot; 5] = [
    PortraitSlot {
        frame: "Boss1Portrait",
        ..TARGET_PORTRAIT
    },
    PortraitSlot {
        frame: "Boss2Portrait",
        ..TARGET_PORTRAIT
    },
    PortraitSlot {
        frame: "Boss3Portrait",
        ..TARGET_PORTRAIT
    },
    PortraitSlot {
        frame: "Boss4Portrait",
        ..TARGET_PORTRAIT
    },
    PortraitSlot {
        frame: "Boss5Portrait",
        ..TARGET_PORTRAIT
    },
];

pub const SMALL_FRAME_GAP: f32 = 8.0;
/// Target-of-target and focus draw the portrait-off art at 3/4 scale.
pub const SMALL_ART_SCALE: f32 = 0.75;
pub const TOT_W: f32 = FRAME_W * SMALL_ART_SCALE;
pub const TOT_H: f32 = FRAME_H * SMALL_ART_SCALE;
pub const FOCUS_W: f32 = TOT_W;
pub const FOCUS_H: f32 = TOT_H;

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

/// `BossPortraitFrameTexture` for elites and rare elites, at its atlas size: TOPRIGHT at
/// (-11, -8) (TargetFrame.lua:436-443, TargetFrame.xml:93-97), as right and top insets.
pub(super) const TARGET_BOSS_PORTRAIT_TOPRIGHT: (f32, f32) = (11.0, 8.0);
/// `BossIcon` star centred on the `Portrait`'s BOTTOM (TargetFrame.xml:281-284).
pub(super) const TARGET_BOSS_ICON_CENTRE: (f32, f32) = (
    TARGET_PORTRAIT.rect.0 + TARGET_PORTRAIT.rect.2 / 2.0,
    TARGET_PORTRAIT.rect.1 + TARGET_PORTRAIT.rect.3,
);
/// `RaidTargetIcon` centred on the `Portrait`'s TOP (TargetFrame.xml:275-279).
pub(super) const TARGET_RAID_TARGET_ICON_CENTRE: (f32, f32) = (
    TARGET_PORTRAIT.rect.0 + TARGET_PORTRAIT.rect.2 / 2.0,
    TARGET_PORTRAIT.rect.1,
);

pub(super) const GOLD_TEXT: &str = "1.0,0.82,0.0,1.0";
pub(super) const VALUE_TEXT: &str = "1.0,1.0,1.0,1.0";
pub(super) const UNIT_FONT: &str = "FrizQuadrata";
pub(super) const UNIT_FONT_SIZE: f32 = 10.0;

/// PetFrame 120×49 (PetFrame.xml:12-13).
pub const PET_FRAME_W: f32 = 120.0;
pub const PET_FRAME_H: f32 = 49.0;

/// `PetPortrait` 37×37 TOPLEFT (5, -5) under `PortraitMask` (`CircleMask`, atlas 1582
/// `interface/masks/circlemask.blp`) over the same rect (PetFrame.xml:22-38).
pub const PET_PORTRAIT: PortraitSlot = PortraitSlot {
    frame: "PetPortrait",
    rect: (5.0, 5.0, 37.0, 37.0),
    mask_fdid: 3_528_314,
    mask_rect: (5.0, 5.0, 37.0, 37.0),
    tex_coords: FULL_PORTRAIT,
};

/// `PetName` 68×10 TOPLEFT at `PetPortrait` TOPRIGHT + (2, 0); `PetFrameHealthBar` 70×10
/// BOTTOMLEFT at `PetPortrait` RIGHT + (2, -3.5); `PetFrameManaBar` 74×7 TOPLEFT at the
/// health bar's BOTTOMLEFT + (-4, -1) (PetFrame.xml:72-79,83-86,134-137).
pub(super) const PET_NAME: Rect = (5.0 + 37.0 + 2.0, 5.0, 68.0, 10.0);
pub(super) const PET_HEALTH: Rect = (5.0 + 37.0 + 2.0, 5.0 + 37.0 / 2.0 + 3.5 - 10.0, 70.0, 10.0);
pub(super) const PET_POWER: Rect = (PET_HEALTH.0 - 4.0, PET_HEALTH.1 + 10.0 + 1.0, 74.0, 7.0);
