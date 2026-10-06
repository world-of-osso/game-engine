//! HUD frame anchors of the two system presets. Modern is Retail's Modern Edit Mode
//! preset (`Blizzard_EditMode/Mainline/EditModePresetLayouts.lua`); Forever takes FlareUI's
//! unit-frame and cast-bar positions (FlareUI 1.3 `Modules/UnitFrames.lua:1888-1894`) and
//! Forever's own Edit Mode preset, the Mainline layouts with Camelot constants
//! (`Blizzard_EditMode/Camelot/EditModePresetLayoutConstants.lua`), for the rest, except
//! the reference's three centred action bars. A screen
//! reads its preset through the canvas's `ActiveSkin`, so a preset switch rebuilds it.

use std::sync::RwLock;

use game_engine_core::ui_layout_data::{
    CHAT_HEIGHT_RANGE, CHAT_WIDTH_RANGE, DAMAGE_METER_HEIGHT_RANGE, DAMAGE_METER_WIDTH_RANGE,
    FrameSizeSettings, LayoutSettings, LayoutSkin, SettingRange,
};
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::screen::SharedContext;

use Point::*;

use crate::main_action_bar_component::{BAR_BOTTOM, BUTTON_PADDING, BUTTON_SIZE};
use crate::minimap::FOREVER_CLUSTER_SIZE;
use crate::ui::screens::inworld_unit_frames_component::{
    PET_FRAME_H, PET_FRAME_W, SMALL_FRAME_GAP, TOT_H, TOT_W, UNIT_FRAME_H, UNIT_FRAME_W,
};
use crate::unit_frame_style::UnitFrameStyle;

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

/// One action bar's Edit Mode settings (`EditModePresetLayouts.lua:11-27`,
/// `Shared/EditModeSettingDisplayInfo.lua:50-79`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActionBarLayout {
    pub anchor: HudAnchor,
    /// `NumIcons` (6..=12): the first buttons of the bar's 12 that show.
    pub num_icons: usize,
    /// `NumRows` (1..=4).
    pub num_rows: usize,
    /// `IconSize` / 100 (50 % to 200 %): the button containers' `SetScale`, which scales
    /// the padding between them too (`EditModeSystemTemplates.lua:1077-1088`).
    pub icon_scale: f32,
}

impl ActionBarLayout {
    /// Buttons per row: `math.ceil(#shownButtonContainers / numRows)` (`ActionBar.lua:100`).
    pub const fn stride(&self) -> usize {
        self.num_icons.div_ceil(self.num_rows)
    }

    /// Rows the buttons fill at that stride.
    pub const fn rows(&self) -> usize {
        self.num_icons.div_ceil(self.stride())
    }

    /// Distance between neighbouring buttons' origins when the skin scales buttons by `scale`.
    const fn pitch(&self, scale: f32) -> f32 {
        (BUTTON_SIZE + BUTTON_PADDING) * scale * self.icon_scale
    }

    /// The bar's width and height.
    pub const fn size(&self, scale: f32) -> (f32, f32) {
        let padding = BUTTON_PADDING * scale * self.icon_scale;
        (
            self.stride() as f32 * self.pitch(scale) - padding,
            self.rows() as f32 * self.pitch(scale) - padding,
        )
    }

    /// Button `index`'s top-left inside the bar: rows fill left to right and stack upward
    /// from the bottom-left (`addButtonsToRight`/`addButtonsToTop`, `MainActionBar.xml:41`,
    /// `MultiActionBars.xml:53-54`, `ActionBar.lua:108-138`).
    pub const fn button_origin(&self, index: usize, scale: f32) -> (f32, f32) {
        let row_from_top = self.rows() - 1 - index / self.stride();
        (
            (index % self.stride()) as f32 * self.pitch(scale),
            row_from_top as f32 * self.pitch(scale),
        )
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
    /// Size and text of the player, target (with its target-of-target), focus and pet frames.
    pub player_style: UnitFrameStyle,
    pub target_style: UnitFrameStyle,
    pub focus_style: UnitFrameStyle,
    pub pet_style: UnitFrameStyle,
    pub cast_bar: HudAnchor,
    pub main_action_bar: ActionBarLayout,
    /// `MultiBarBottomLeft` (Action Bar 2) and `MultiBarBottomRight` (Action Bar 3); `None`
    /// while the bar is not enabled.
    pub action_bar_2: Option<ActionBarLayout>,
    pub action_bar_3: Option<ActionBarLayout>,
    pub pet_action_bar: HudAnchor,
    pub micro_menu: HudAnchor,
    pub show_micro_menu: bool,
    pub bags_bar: HudAnchor,
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

const MODERN_MAIN_ACTION_BAR: ActionBarLayout = ActionBarLayout {
    anchor: anchor(Bottom, Bottom, 0.0, BAR_BOTTOM),
    num_icons: 12,
    num_rows: 1,
    icon_scale: 1.0,
};

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
    player_style: UnitFrameStyle::AUTHORED,
    target_style: UnitFrameStyle::AUTHORED,
    focus_style: UnitFrameStyle::AUTHORED,
    pet_style: UnitFrameStyle::AUTHORED,
    // Centred above two action bar rows.
    cast_bar: anchor(Bottom, Bottom, 0.0, 152.0),
    // `MAIN_ACTION_BAR_DEFAULT_OFFSET_Y` (Standard/EditModePresetLayoutConstants.lua:2);
    // settings EditModePresetLayouts.lua:11-19.
    main_action_bar: MODERN_MAIN_ACTION_BAR,
    show_micro_menu: false,
    // Both frames are `hidden="true"` (MultiActionBars.xml:45,75) until the player ticks
    // "Action Bar 2/3" (`GetActionBarToggles`, MultiActionBars.lua:17-33,79-91,
    // Blizzard_SettingsDefinitions_Frame/ActionBars.lua:9,29-30); no such toggle exists here.
    action_bar_2: None,
    action_bar_3: None,
    // Standard constants :2,12 and EditModeManager.lua's fixed bottom stack.
    pet_action_bar: anchor(
        BottomLeft,
        Bottom,
        -MODERN_MAIN_ACTION_BAR.size(1.0).0 / 2.0,
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
    // User choice 2026-10-05: top-left, mirroring the minimap's top-right margins.
    // Modern already used this anchor (EditModePresetLayouts.lua:857-863).
    damage_meter: anchor(TopLeft, TopLeft, 0.0, 0.0),
    damage_meter_size: (400.0, 140.0),
    // Chattynator Core/Config.lua:28-29; preserve Modern's existing output.
    chat: anchor(BottomLeft, BottomLeft, 0.0, 40.0),
    chat_size: (500.0, 280.0),
    // `ObjectiveTrackerFrame` Edit Mode default y, flush right (user decision
    // 2026-10-05): Retail's x -110 leaves room for the two right-side action bars.
    objective_tracker: anchor(TopRight, TopRight, 0.0, -275.0),
    // Status bar 1: EditModePresetLayouts.lua:582-594, `STATUS_BAR_1_ANCHOR_OFFSET_Y` 0.
    xp_bar: anchor(Bottom, Bottom, 0.0, 0.0),
};

/// FlareUI Core.lua:102-107,218; ActionBars.lua:43,179-190 includes pet buttons.
pub const FOREVER_ACTION_BUTTON_SCALE: f32 = 1.06;

/// The reference player's Edit Mode layout, measured on
/// `user-flareui-reddit-e6wzfwkui5th1.png`: three centred bars stacked from Camelot's
/// bottom inset 2, the lower two of 9 buttons (pitch 76 px), the top one of 10 smaller
/// buttons (pitch 59 px, so 59/76 of the lower rows' size), rows 3-5 px apart, which is the
/// button padding. Which bars and `IconSize` steps the player uses is not recoverable.
const FOREVER_BAR_GAP: f32 = BUTTON_PADDING * FOREVER_ACTION_BUTTON_SCALE;
const FOREVER_SMALL_ICON_SCALE: f32 = 0.78;

const fn forever_bar(bottom: f32, num_icons: usize, icon_scale: f32) -> ActionBarLayout {
    ActionBarLayout {
        anchor: anchor(Bottom, Bottom, 0.0, bottom),
        num_icons,
        num_rows: 1,
        icon_scale,
    }
}

/// The offset of a bar's top edge above UIParent's bottom.
const fn forever_bar_top(bar: &ActionBarLayout) -> f32 {
    bar.anchor.y + bar.size(FOREVER_ACTION_BUTTON_SCALE).1
}

const FOREVER_MAIN_ACTION_BAR: ActionBarLayout = forever_bar(2.0, 9, 1.0);
const FOREVER_ACTION_BAR_2: ActionBarLayout = forever_bar(
    forever_bar_top(&FOREVER_MAIN_ACTION_BAR) + FOREVER_BAR_GAP,
    9,
    1.0,
);
const FOREVER_ACTION_BAR_3: ActionBarLayout = forever_bar(
    forever_bar_top(&FOREVER_ACTION_BAR_2) + FOREVER_BAR_GAP,
    10,
    FOREVER_SMALL_ICON_SCALE,
);

/// FlareUI Modules/Minimap.lua:366-379,401-406 ("Match Objective Tracker Width", default on,
/// Core.lua:262): `ObjectiveTrackerFrame:SetScale(minimap frame outer width / 288)`, 288
/// being the visible line of the 300-wide header art. The frame here is the cluster border.
pub const FOREVER_TRACKER_SCALE: f32 = FOREVER_CLUSTER_SIZE / 288.0;

/// Chat messages: Mainline/EditModePresetLayouts.lua:490-503, Camelot constants:66.
/// FlareUI Chat.lua:1515-1520 adds padding 10 and header 24; DamageMeter.lua:1150-1159
/// matches that skin's size; meter placement follows the user's top-left choice.
pub const FOREVER_CHAT_PANEL_SIZE: (f32, f32) = (450.0, 214.0);

/// Forever loads the shared Mainline preset file with Camelot constants
/// (`Blizzard_EditMode.toc:9`); FlareUI/reference overrides chat and meter geometry.
pub const FOREVER: HudLayout = HudLayout {
    player: anchor(Center, Center, -330.0, -270.0),
    target: anchor(Center, Center, 330.0, -270.0),
    // FlareUI UnitFrames.lua:1890-1892 pins ToT to UIParent's BOTTOM (270, 248) and focus to
    // its RIGHT (-453, -258), laid out on a UIParent of about 2146x1207 units: the focus
    // frame right of the target in `user-unit-frames-2026-10-03.png` measures a 9-10 unit
    // gap, 2146/2 - 453 - 160 - 450. Edge pins drift onto the target on other canvases, so
    // both keep that layout's relation to the target instead: ToT left-aligned with it, 28
    // below (4 under its 24-tall cast bar); focus top-aligned, 10 right of it.
    target_of_target: anchor(TopLeft, Center, 330.0 - 120.0, -270.0 - 30.0 - 28.0),
    focus: anchor(TopLeft, Center, 330.0 + 120.0 + 10.0, -270.0 + 30.0),
    // Reference screenshot: pet right-aligned, measured 6px below the player.
    pet: anchor(TopRight, Center, -330.0 + 120.0, -270.0 - 30.0 - 6.0),
    // FlareUI UnitFrames.lua:1894 (keyboard default, :1900-1903).
    cast_bar: anchor(Bottom, Bottom, 0.0, 268.0),
    // The reference uses the player's Edit Mode layout, not Camelot's combined strip.
    main_action_bar: FOREVER_MAIN_ACTION_BAR,
    action_bar_2: Some(FOREVER_ACTION_BAR_2),
    action_bar_3: Some(FOREVER_ACTION_BAR_3),
    // Micro menu and bags bar keep Modern's bottom-right corner: Camelot's strip beside
    // the main bar is gone. User decision 2026-10-05 hides only the micro menu by default;
    // both independently anchored frames keep their authored positions.
    // Mainline/EditModePresetLayouts.lua:183-198; Camelot constants :3,31,35.
    // Shared/EditModeManager.lua:664-672,703-718: BOTTOMLEFT on the base bar's
    // BOTTOMLEFT, indented 30, 4 above the topmost bar of the bottom stack.
    pet_action_bar: anchor(
        BottomLeft,
        Bottom,
        -FOREVER_MAIN_ACTION_BAR.size(FOREVER_ACTION_BUTTON_SCALE).0 / 2.0 + 30.0,
        forever_bar_top(&FOREVER_ACTION_BAR_3) + 4.0,
    ),
    // User choice 2026-10-05: inherit Modern's top-left meter anchor, matching the
    // inherited minimap's mirrored margins. Saved custom placements remain untouched.
    damage_meter: MODERN.damage_meter,
    damage_meter_size: FOREVER_CHAT_PANEL_SIZE,
    // User decision 2026-10-06: visible chat border touches the bottom-left corner.
    // The skin begins 24 right of the canvas and ends 28 above its bottom; its
    // tooltip-border line is 2 units inside that rectangle.
    chat: anchor(BottomLeft, BottomLeft, -26.0, -30.0),
    chat_size: (469.0, 235.0),
    // The scaled tracker header begins four local units above its frame; clear the
    // 260-unit minimap on the default 1080-unit canvas without moving other frames.
    objective_tracker: anchor(TopRight, TopRight, 0.0, -300.0),
    // Reference screenshot: top centre, bar art beginning 7 units below the screen edge.
    xp_bar: anchor(Top, Top, 0.0, -6.0),
    ..MODERN
};

/// A dimension's setting within its Edit Mode range, else the preset's `default`.
fn sized(default: f32, setting: Option<u16>, range: SettingRange) -> f32 {
    setting.map_or(default, |value| f32::from(range.clamp(value)))
}

fn frame_size(
    (width, height): (f32, f32),
    setting: FrameSizeSettings,
    (width_range, height_range): (SettingRange, SettingRange),
) -> (f32, f32) {
    (
        sized(width, setting.width, width_range),
        sized(height, setting.height, height_range),
    )
}

/// A frame drawn inside `parent`'s scaled space: its anchor point keeps its place in the
/// parent's own units, as a child of a Retail frame does under `SetScale`.
fn child_anchor(child: HudAnchor, parent: &HudAnchor, scale: f32) -> HudAnchor {
    if scale == 1.0 {
        return child;
    }
    assert_eq!(
        child.relative, parent.relative,
        "a child frame anchors to its parent's screen point"
    );
    HudAnchor {
        x: parent.x + (child.x - parent.x) * scale,
        y: parent.y + (child.y - parent.y) * scale,
        ..child
    }
}

impl HudLayout {
    /// The preset with a layout's settings applied (docs/specs/hud-edit-mode.md
    /// "Customisable layout settings").
    fn with_settings(mut self, settings: &LayoutSettings) -> Self {
        self.show_micro_menu = settings.show_micro_menu.unwrap_or(self.show_micro_menu);
        self.chat_size = frame_size(
            self.chat_size,
            settings.chat,
            (CHAT_WIDTH_RANGE, CHAT_HEIGHT_RANGE),
        );
        self.damage_meter_size = frame_size(
            self.damage_meter_size,
            settings.damage_meter,
            (DAMAGE_METER_WIDTH_RANGE, DAMAGE_METER_HEIGHT_RANGE),
        );
        self.player_style = UnitFrameStyle::of(settings.player_frame);
        self.target_style = UnitFrameStyle::of(settings.target_frame);
        self.focus_style = UnitFrameStyle::of(settings.focus_frame);
        self.pet_style = UnitFrameStyle::of(settings.pet_frame);
        self.target_of_target =
            child_anchor(self.target_of_target, &self.target, self.target_style.scale);
        self.pet = child_anchor(self.pet, &self.player, self.player_style.scale);
        self
    }
}

fn layout_of(skin: ActiveSkin, settings: &LayoutSettings) -> HudLayout {
    match skin {
        ActiveSkin::Modern => MODERN,
        ActiveSkin::Forever => FOREVER,
    }
    .with_settings(settings)
}

/// The HUD layout of a layout with `skin` and `settings`, for the Options controls that
/// show a layout's effective values.
pub fn layout_with(skin: LayoutSkin, settings: &LayoutSettings) -> HudLayout {
    let skin = match skin {
        LayoutSkin::Modern => ActiveSkin::Modern,
        LayoutSkin::Forever => ActiveSkin::Forever,
    };
    layout_of(skin, settings)
}

/// The settings of the layout the client draws (`None` until one is applied: the preset's
/// own values); every canvas mirrors them into its `SharedContext` beside the active skin.
static ACTIVE_SETTINGS: RwLock<Option<LayoutSettings>> = RwLock::new(None);

pub fn active_layout_settings() -> LayoutSettings {
    ACTIVE_SETTINGS
        .read()
        .expect("layout settings lock")
        .unwrap_or_default()
}

pub fn set_active_layout_settings(settings: LayoutSettings) {
    *ACTIVE_SETTINGS.write().expect("layout settings lock") = Some(settings);
}

/// The HUD layout the client draws, for state built outside a canvas.
pub fn active_hud_layout() -> HudLayout {
    layout_of(ui_toolkit::atlas::thread_skin(), &active_layout_settings())
}

/// The canvas's HUD layout: its preset (every canvas mirrors the active skin) with the
/// active layout's settings, the preset's own values on a canvas that carries none.
pub fn hud_layout(ctx: &SharedContext) -> HudLayout {
    let skin = *ctx
        .get::<ActiveSkin>()
        .expect("canvas carries the active skin");
    let settings = ctx.get::<LayoutSettings>().copied().unwrap_or_default();
    layout_of(skin, &settings)
}
