//! HUD frame anchors of the two system presets. Modern is Retail's Modern Edit Mode
//! preset (`Blizzard_EditMode/Mainline/EditModePresetLayouts.lua`); Forever takes FlareUI's
//! unit-frame and cast-bar positions (FlareUI 1.3 `Modules/UnitFrames.lua:1888-1894`) and
//! Forever's own Edit Mode preset, the Mainline layouts with Camelot constants
//! (`Blizzard_EditMode/Camelot/EditModePresetLayoutConstants.lua`), for the rest, except
//! the reference-centred main bar and hidden utility bars. A screen
//! reads its preset through the canvas's `ActiveSkin`, so a preset switch rebuilds it.

use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::screen::SharedContext;

use Point::*;

use crate::main_action_bar_component::{BAR_BOTTOM, BAR_W, BUTTON_SIZE};
use crate::micro_menu::MICRO_MENU_W;
use crate::minimap::FOREVER_CLUSTER_SIZE;
use crate::ui::screens::inworld_unit_frames_component::{
    PET_FRAME_H, PET_FRAME_W, SMALL_FRAME_GAP, TOT_H, TOT_W, UNIT_FRAME_H, UNIT_FRAME_W,
};

/// A point of a rect: `SetPoint`'s `point` and `relativePoint`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Point {
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl Point {
    /// Where the point sits across and down its rect, as fractions of width and height.
    const fn fractions(self) -> (f32, f32) {
        match self {
            Self::TopLeft => (0.0, 0.0),
            Self::Top => (0.5, 0.0),
            Self::TopRight => (1.0, 0.0),
            Self::Left => (0.0, 0.5),
            Self::Center => (0.5, 0.5),
            Self::Right => (1.0, 0.5),
            Self::BottomLeft => (0.0, 1.0),
            Self::Bottom => (0.5, 1.0),
            Self::BottomRight => (1.0, 1.0),
        }
    }
}

/// `frame:SetPoint(point, UIParent, relative, x, y)`; `y` grows up, as in WoW.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HudAnchor {
    pub point: Point,
    pub relative: Point,
    pub x: f32,
    pub y: f32,
}

const fn anchor(point: Point, relative: Point, x: f32, y: f32) -> HudAnchor {
    HudAnchor {
        point,
        relative,
        x,
        y,
    }
}

/// Absolute-position attributes that put a frame at its anchor: the insets from the sides
/// its relative point sits on ("auto" for the others) and the margins of centred axes.
#[derive(Clone, Debug, PartialEq)]
pub struct Placement {
    pub left: String,
    pub right: String,
    pub top: String,
    pub bottom: String,
    pub margin_left: f32,
    pub margin_top: f32,
}

/// One axis: (start inset, end inset, margin) for an offset `offset` toward the end, the
/// frame's point at `point` and UIParent's at `relative` (fractions of the axis).
fn axis(relative: f32, point: f32, offset: f32, size: f32) -> (String, String, f32) {
    let auto = || "auto".to_string();
    if relative == 0.0 {
        ((offset - point * size).to_string(), auto(), 0.0)
    } else if relative == 1.0 {
        (auto(), (-offset - (1.0 - point) * size).to_string(), 0.0)
    } else {
        ("50%".to_string(), auto(), offset - point * size)
    }
}

impl HudAnchor {
    /// A child TOPLEFT offset in the parent's rect (x right, y down), retaining its screen anchor.
    pub fn offset_from_top_left(&self, size: (f32, f32), offset: (f32, f32)) -> Self {
        let (point_x, point_y) = self.point.fractions();
        Self {
            point: TopLeft,
            relative: self.relative,
            x: self.x - point_x * size.0 + offset.0,
            y: self.y + point_y * size.1 - offset.1,
        }
    }

    /// Attributes placing a `width`×`height` frame at this anchor on its full-screen parent.
    pub fn place(&self, (width, height): (f32, f32)) -> Placement {
        let (point_x, point_y) = self.point.fractions();
        let (relative_x, relative_y) = self.relative.fractions();
        let (left, right, margin_left) = axis(relative_x, point_x, self.x, width);
        let (top, bottom, margin_top) = axis(relative_y, point_y, -self.y, height);
        Placement {
            left,
            right,
            top,
            bottom,
            margin_left,
            margin_top,
        }
    }
}

/// Where a preset puts each HUD frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HudLayout {
    pub player: HudAnchor,
    pub target: HudAnchor,
    pub target_of_target: HudAnchor,
    pub focus: HudAnchor,
    pub pet: HudAnchor,
    pub cast_bar: HudAnchor,
    pub main_action_bar: HudAnchor,
    pub pet_action_bar: HudAnchor,
    pub micro_menu: HudAnchor,
    pub bags_bar: HudAnchor,
    /// The supplied Forever reference shows neither utility bar; FlareUI supports hiding them.
    pub hide_utility_bars: bool,
    pub buffs: HudAnchor,
    pub debuffs: HudAnchor,
    pub minimap: HudAnchor,
    pub party: HudAnchor,
    pub raid: HudAnchor,
    pub damage_meter: HudAnchor,
    pub damage_meter_size: (f32, f32),
    pub chat: HudAnchor,
    pub chat_size: (f32, f32),
    pub objective_tracker: HudAnchor,
    pub xp_bar: HudAnchor,
}

/// Target of target and focus sit right of the whole TargetFrame, tops level with its
/// centred 192×67 `FrameTexture`.
const SMALL_FRAME_BOTTOM: f32 = 250.0 + UNIT_FRAME_H - (UNIT_FRAME_H - 67.0) / 2.0 - TOT_H;
const TOT_LEFT: f32 = 300.0 + UNIT_FRAME_W + SMALL_FRAME_GAP;
/// `MicroButtonAndBagsBar` 232×80 at BOTTOMRIGHT (-6, 6).
const MICRO_BAGS_BAR: (f32, f32) = (-6.0, 6.0 + 80.0);

pub const MODERN: HudLayout = HudLayout {
    // EditModePresetLayouts.lua:231-257.
    player: anchor(BottomRight, Bottom, -300.0, 250.0),
    target: anchor(BottomLeft, Bottom, 300.0, 250.0),
    target_of_target: anchor(BottomLeft, Bottom, TOT_LEFT, SMALL_FRAME_BOTTOM),
    focus: anchor(
        BottomLeft,
        Bottom,
        TOT_LEFT + TOT_W + SMALL_FRAME_GAP,
        SMALL_FRAME_BOTTOM,
    ),
    // PetFrame (`leftPadding` 15, centred) hangs from the player frame's BOTTOM + (30, 25)
    // (PlayerFrame.xml:467-474, PlayerFrameTemplates.xml:4-10, LayoutFrame.lua:345-348).
    pet: anchor(
        BottomLeft,
        Bottom,
        -300.0 - UNIT_FRAME_W / 2.0 + 30.0 + 15.0 / 2.0 - PET_FRAME_W / 2.0,
        250.0 + 25.0 - PET_FRAME_H,
    ),
    // Centred above two action bar rows.
    cast_bar: anchor(Bottom, Bottom, 0.0, 152.0),
    // `MAIN_ACTION_BAR_DEFAULT_OFFSET_Y` (Standard/EditModePresetLayoutConstants.lua:2).
    main_action_bar: anchor(Bottom, Bottom, 0.0, BAR_BOTTOM),
    // Standard constants :2,12 and EditModeManager.lua's fixed bottom stack.
    pet_action_bar: anchor(
        BottomLeft,
        Bottom,
        -BAR_W / 2.0,
        BAR_BOTTOM + BUTTON_SIZE + 5.0,
    ),
    // Micro menu BOTTOMRIGHT and bags TOPRIGHT (0, 10) of `MicroButtonAndBagsBar`
    // (Standard/EditModePresetLayoutConstants.lua:15-26).
    micro_menu: anchor(BottomRight, BottomRight, MICRO_BAGS_BAR.0, 6.0),
    bags_bar: anchor(
        TopRight,
        BottomRight,
        MICRO_BAGS_BAR.0,
        MICRO_BAGS_BAR.1 + 10.0,
    ),
    hide_utility_bars: false,
    // EditModePresetLayouts.lua:425-431,443-449.
    buffs: anchor(TopRight, TopRight, -255.0, -10.0),
    debuffs: anchor(TopRight, TopRight, -270.0, -155.0),
    // EditModePresetLayouts.lua:371-384.
    minimap: anchor(TopRight, TopRight, 0.0, 0.0),
    // On collapsed `CompactRaidFrameManager`'s (222 wide, TOPLEFT -200, -140) TOPRIGHT at
    // (0, -7) (EditModePresetLayouts.lua:290-295, Blizzard_CompactRaidFrameManager.lua:93).
    party: anchor(TopLeft, TopLeft, -200.0 + 222.0, -140.0 - 7.0),
    // Above the central unit-frame cluster.
    raid: anchor(Bottom, Bottom, 0.0, 215.0),
    // EditModePresetLayouts.lua:857-863.
    damage_meter: anchor(TopLeft, TopLeft, 0.0, 0.0),
    damage_meter_size: (400.0, 140.0),
    // Chattynator Core/Config.lua:28-29; preserve Modern's existing output.
    chat: anchor(BottomLeft, BottomLeft, 0.0, 40.0),
    chat_size: (500.0, 280.0),
    // `ObjectiveTrackerFrame` Edit Mode default.
    objective_tracker: anchor(TopRight, TopRight, -110.0, -275.0),
    // Status bar 1: EditModePresetLayouts.lua:582-594, `STATUS_BAR_1_ANCHOR_OFFSET_Y` 0.
    xp_bar: anchor(Bottom, Bottom, 0.0, 0.0),
};

/// Camelot `MICRO_MENU_ANCHOR_*` BOTTOM (116.5, 6) (EditModePresetLayoutConstants.lua:38-42).
const CAMELOT_MICRO_MENU: (f32, f32) = (116.5, 6.0);
/// Reference correction: centre the existing bar, retaining Camelot's bottom inset.
const FOREVER_MAIN_ACTION_BAR: HudAnchor = anchor(Bottom, Bottom, 0.0, 2.0);
/// FlareUI Core.lua:102-107,218; ActionBars.lua:43,179-190 includes pet buttons.
pub const FOREVER_ACTION_BUTTON_SCALE: f32 = 1.06;

/// FlareUI Modules/Minimap.lua:366-379,401-406 ("Match Objective Tracker Width", default on,
/// Core.lua:262): `ObjectiveTrackerFrame:SetScale(minimap frame outer width / 288)`, 288
/// being the visible line of the 300-wide header art. The frame here is the cluster border.
pub const FOREVER_TRACKER_SCALE: f32 = FOREVER_CLUSTER_SIZE / 288.0;

/// Chat messages: Mainline/EditModePresetLayouts.lua:490-503, Camelot constants:66.
/// FlareUI Chat.lua:1515-1520 adds padding 10 and header 24; DamageMeter.lua:1150-1159
/// matches that skin's size. The reference mirrors the meter to the right.
pub const FOREVER_CHAT_PANEL_SIZE: (f32, f32) = (450.0, 214.0);

/// Forever loads the shared Mainline preset file with Camelot constants
/// (`Blizzard_EditMode.toc:9`); FlareUI/reference overrides chat and meter geometry.
pub const FOREVER: HudLayout = HudLayout {
    player: anchor(Center, Center, -330.0, -270.0),
    target: anchor(Center, Center, 330.0, -270.0),
    // Reference screenshot: ToT top-aligned, measured 8px right of the target.
    target_of_target: anchor(TopLeft, Center, 330.0 + 120.0 + 8.0, -270.0 + 30.0),
    focus: anchor(Right, Right, -453.0, -258.0),
    // Reference screenshot: pet right-aligned, measured 6px below the player.
    pet: anchor(TopRight, Center, -330.0 + 120.0, -270.0 - 30.0 - 6.0),
    // FlareUI UnitFrames.lua:1894 (keyboard default, :1900-1903).
    cast_bar: anchor(Bottom, Bottom, 0.0, 268.0),
    micro_menu: anchor(Bottom, Bottom, CAMELOT_MICRO_MENU.0, CAMELOT_MICRO_MENU.1),
    // The reference uses the player's Edit Mode layout, not Camelot's combined strip.
    main_action_bar: FOREVER_MAIN_ACTION_BAR,
    // FlareUI Visibility.lua:317-344 supports permanent hiding. Reference profile's
    // exact hide/fade settings are unknown; this preset matches its visible result.
    hide_utility_bars: true,
    // Mainline/EditModePresetLayouts.lua:183-198; Camelot constants :3,31,35.
    // Shared/EditModeManager.lua:664-672,703-718: BOTTOMLEFT on the base bar's
    // BOTTOMLEFT, above its scaled height + 4, indented 30.
    pet_action_bar: anchor(
        BottomLeft,
        Bottom,
        FOREVER_MAIN_ACTION_BAR.x - BAR_W * FOREVER_ACTION_BUTTON_SCALE / 2.0 + 30.0,
        FOREVER_MAIN_ACTION_BAR.y + BUTTON_SIZE * FOREVER_ACTION_BUTTON_SCALE + 4.0,
    ),
    // `BAGS_ANCHOR_*`: BOTTOMLEFT on the micro menu's BOTTOMRIGHT at (7, -4) (:45-49).
    bags_bar: anchor(
        BottomLeft,
        Bottom,
        CAMELOT_MICRO_MENU.0 + MICRO_MENU_W / 2.0 + 7.0,
        CAMELOT_MICRO_MENU.1 - 4.0,
    ),
    // Mirror the chat skin's 25-unit inset and 135-unit baseline.
    damage_meter: anchor(BottomRight, BottomRight, -25.0, 135.0),
    damage_meter_size: FOREVER_CHAT_PANEL_SIZE,
    // Chat canvas messages begin at (34,27) and end 38 above the canvas bottom.
    // Place the source's 430x170 messages at BOTTOMLEFT(35,145).
    chat: anchor(BottomLeft, BottomLeft, 1.0, 107.0),
    chat_size: (469.0, 235.0),
    // Reference screenshot: top centre, bar art beginning 7 units below the screen edge.
    xp_bar: anchor(Top, Top, 0.0, -6.0),
    ..MODERN
};

/// The HUD layout of the canvas's preset; every canvas mirrors the active skin.
pub fn hud_layout(ctx: &SharedContext) -> &'static HudLayout {
    match ctx
        .get::<ActiveSkin>()
        .expect("canvas carries the active skin")
    {
        ActiveSkin::Modern => &MODERN,
        ActiveSkin::Forever => &FOREVER,
    }
}
