//! Retail `MainActionBar` (`Blizzard_ActionBar/Mainline/MainActionBar.xml`,
//! `ActionButtonTemplate.xml`) with the Modern Edit Mode preset
//! (`Blizzard_EditMode/Mainline/EditModePresetLayouts.lua`): 12 buttons of 45×45,
//! 2 px apart (`minButtonPadding`), anchored BOTTOM at y 45
//! (`MAIN_ACTION_BAR_DEFAULT_OFFSET_Y`), gryphon end caps, keys 1..=.

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::FrameName;
use crate::ui::screens::inworld_unit_frames_component::inworld_unit_frames_art::AtlasArt;
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::GameFont;

pub const MAIN_ACTION_BAR: FrameName = FrameName("MainActionBar");
pub const MAIN_BAR_BUTTONS: usize = 12;
/// Clicking button `n` (0-based) emits `"{ACTION_BUTTON_PREFIX}{n}"`.
pub const ACTION_BUTTON_PREFIX: &str = "action_button:";

pub const BUTTON_SIZE: f32 = 45.0;
pub const BUTTON_PADDING: f32 = 2.0;
pub const BAR_W: f32 =
    MAIN_BAR_BUTTONS as f32 * BUTTON_SIZE + (MAIN_BAR_BUTTONS as f32 - 1.0) * BUTTON_PADDING;
pub const BAR_BOTTOM: f32 = 45.0;
/// `NormalTexture`/`PushedTexture`/`HighlightTexture` are 46×45 at TOPLEFT.
const FRAME_ART_W: f32 = 46.0;
/// `Cooldown` is inset 3 px from the icon.
const COOLDOWN_INSET: f32 = 3.0;
/// Cooldown `SwipeTexture` colour 0,0,0,0.8.
const COOLDOWN_SWIPE: &str = "0.0,0.0,0.0,0.8";
/// `HotKey`: 32×10 at TOPRIGHT -4,-5 (`hotkeyTextKeyboardX/Y`), `NumberFontNormalSmallGray`.
const HOTKEY_W: f32 = 32.0;
const HOTKEY_H: f32 = 10.0;
const HOTKEY_RIGHT: f32 = 4.0;
const HOTKEY_TOP: f32 = 5.0;
const HOTKEY_COLOR: &str = "0.6,0.6,0.6,1.0";
/// `CooldownFrameTemplate` countdown numbers.
const COOLDOWN_TEXT_COLOR: &str = "1.0,1.0,1.0,1.0";
/// End caps 104.5×98: BOTTOMRIGHT of the left cap at the bar's BOTTOMLEFT +9,-22; the
/// right cap's BOTTOMLEFT at the bar's BOTTOMRIGHT -8,-22.
const END_CAP_W: f32 = 104.5;
const END_CAP_H: f32 = 98.0;

/// UiTextureAtlas `interface/hud/uiactionbar.blp` (FDID 4613342, 256×1024).
const fn action_bar(rect: (f32, f32, f32, f32)) -> AtlasArt {
    AtlasArt {
        fdid: 4_613_342,
        atlas: (256.0, 1024.0),
        rect,
    }
}

/// Every chrome texture the bar draws, for hosts that copy art out of local CASC.
pub const ACTION_BAR_ART_FDIDS: [u32; 1] = [4_613_342];

/// `UI-HUD-ActionBar-IconFrame-Background`.
const SLOT_BACKGROUND: AtlasArt = action_bar((181.0, 227.0, 411.0, 456.0));
/// `ui-hud-actionbar-iconframe-slot`.
const SLOT_ART: AtlasArt = action_bar((181.0, 245.0, 136.0, 198.0));
/// `UI-HUD-ActionBar-IconFrame`.
const NORMAL: AtlasArt = action_bar((181.0, 227.0, 254.0, 299.0));
/// `UI-HUD-ActionBar-IconFrame-Down`.
const PUSHED: AtlasArt = action_bar((181.0, 227.0, 521.0, 566.0));
/// `UI-HUD-ActionBar-IconFrame-Mouseover`.
const HIGHLIGHT: AtlasArt = action_bar((181.0, 227.0, 643.0, 688.0));
/// `ui-hud-actionbar-gryphon-left` / `-right`.
const GRYPHON_LEFT: AtlasArt = action_bar((1.0, 179.0, 136.0, 303.0));
const GRYPHON_RIGHT: AtlasArt = action_bar((1.0, 179.0, 305.0, 472.0));

/// One button's contents.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ActionButtonView {
    /// Spell icon FDID; 0 for an empty slot.
    pub icon_fdid: u32,
    /// Remaining fraction of the running cooldown or GCD, 0 when none.
    pub cooldown_fraction: f32,
    /// Countdown text of cooldowns of 2 s and longer.
    pub cooldown_text: String,
    /// Key held or button pressed: `PushedTexture` replaces `NormalTexture`.
    pub pushed: bool,
    /// Pointer over the button: `HighlightTexture`.
    pub hovered: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MainActionBarState {
    pub buttons: [ActionButtonView; MAIN_BAR_BUTTONS],
}

impl Default for MainActionBarState {
    fn default() -> Self {
        Self {
            buttons: std::array::from_fn(|_| ActionButtonView::default()),
        }
    }
}

/// Retail `ActionButton<n>` name of button `index` (0-based).
pub fn action_button_name(index: usize) -> String {
    format!("ActionButton{}", index + 1)
}

/// Key label of button `index` (`ACTIONBUTTON1..12` default bindings).
pub fn hotkey_label(index: usize) -> &'static str {
    ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "-", "="]
        .get(index)
        .copied()
        .unwrap_or("")
}

/// Button index of an action this screen emitted.
pub fn parse_action_button(action: &str) -> Option<usize> {
    action
        .strip_prefix(ACTION_BUTTON_PREFIX)?
        .parse()
        .ok()
        .filter(|&index| index < MAIN_BAR_BUTTONS)
}

struct DynName(String);

fn art(name: String, art: &AtlasArt, rect: (f32, f32, f32, f32), hidden: bool) -> Element {
    let (x, y, width, height) = rect;
    let coords = art.tex_coords(1.0);
    rsx! {
        texture {
            name: {DynName(name)},
            width,
            height,
            hidden,
            texture_fdid: {art.fdid},
            tex_coords: {coords.as_str()},
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
        }
    }
}

fn icon(name: String, fdid: u32) -> Element {
    rsx! {
        texture {
            name: {DynName(name)},
            width: BUTTON_SIZE,
            height: BUTTON_SIZE,
            hidden: {fdid == 0},
            texture_fdid: {fdid},
            pos_type: "absolute",
            pos_x: 0.0,
            pos_y: 0.0,
        }
    }
}

/// The swipe covers the icon's remaining fraction, draining downward.
fn cooldown(name: &str, view: &ActionButtonView) -> Element {
    let side = BUTTON_SIZE - 2.0 * COOLDOWN_INSET;
    let height = (side * view.cooldown_fraction.clamp(0.0, 1.0)).round();
    let swipe = DynName(format!("{name}Cooldown"));
    let text = DynName(format!("{name}CooldownText"));
    rsx! {
        r#frame {
            name: swipe,
            width: side,
            height,
            hidden: {height <= 0.0},
            background_color: COOLDOWN_SWIPE,
            pos_type: "absolute",
            pos_x: COOLDOWN_INSET,
            pos_y: {BUTTON_SIZE - COOLDOWN_INSET - height},
        }
        fontstring {
            name: text,
            width: BUTTON_SIZE,
            height: BUTTON_SIZE,
            text: {view.cooldown_text.as_str()},
            font: GameFont::FrizQuadrata,
            font_size: 16.0,
            font_color: COOLDOWN_TEXT_COLOR,
            outline: "OUTLINE",
            justify_h: "CENTER",
            pos_type: "absolute",
            pos_x: 0.0,
            pos_y: 0.0,
        }
    }
}

fn hotkey(name: &str, index: usize) -> Element {
    rsx! {
        fontstring {
            name: {DynName(format!("{name}HotKey"))},
            width: HOTKEY_W,
            height: HOTKEY_H,
            text: {hotkey_label(index)},
            font: GameFont::ArialNarrow,
            font_size: 12.0,
            font_color: HOTKEY_COLOR,
            outline: "OUTLINE",
            justify_h: "RIGHT",
            pos_type: "absolute",
            right: HOTKEY_RIGHT,
            pos_y: HOTKEY_TOP,
        }
    }
}

fn button(index: usize, view: &ActionButtonView) -> Element {
    let name = action_button_name(index);
    let x = index as f32 * (BUTTON_SIZE + BUTTON_PADDING);
    let frame_art = (0.0, 0.0, FRAME_ART_W, BUTTON_SIZE);
    let cell = (0.0, 0.0, BUTTON_SIZE, BUTTON_SIZE);
    let children: Element = [
        art(
            format!("{name}SlotBackground"),
            &SLOT_BACKGROUND,
            cell,
            false,
        ),
        art(format!("{name}SlotArt"), &SLOT_ART, cell, false),
        icon(format!("{name}Icon"), view.icon_fdid),
        cooldown(&name, view),
        art(
            format!("{name}NormalTexture"),
            &NORMAL,
            frame_art,
            view.pushed,
        ),
        art(
            format!("{name}PushedTexture"),
            &PUSHED,
            frame_art,
            !view.pushed,
        ),
        art(
            format!("{name}HighlightTexture"),
            &HIGHLIGHT,
            frame_art,
            !view.hovered,
        ),
        hotkey(&name, index),
    ]
    .into_iter()
    .flatten()
    .collect();
    rsx! {
        button {
            name: {DynName(name)},
            width: BUTTON_SIZE,
            height: BUTTON_SIZE,
            onclick: {format!("{ACTION_BUTTON_PREFIX}{index}")},
            button_default_skin: false,
            pos_type: "absolute",
            pos_x: x,
            pos_y: 0.0,
            {children}
        }
    }
}

fn end_caps() -> Element {
    let top = BUTTON_SIZE + 22.0 - END_CAP_H;
    [
        art(
            "MainActionBarLeftEndCap".into(),
            &GRYPHON_LEFT,
            (9.0 - END_CAP_W, top, END_CAP_W, END_CAP_H),
            false,
        ),
        art(
            "MainActionBarRightEndCap".into(),
            &GRYPHON_RIGHT,
            (BAR_W - 8.0, top, END_CAP_W, END_CAP_H),
            false,
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

pub fn main_action_bar_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<MainActionBarState>()
        .expect("MainActionBarState must be in SharedContext");
    let buttons: Element = state
        .buttons
        .iter()
        .enumerate()
        .flat_map(|(index, view)| button(index, view))
        .collect();
    rsx! {
        r#frame {
            name: MAIN_ACTION_BAR,
            width: BAR_W,
            height: BUTTON_SIZE,
            strata: FrameStrata::Medium,
            pos_type: "absolute",
            left: "50%",
            translate_x: "-50%",
            bottom: BAR_BOTTOM,
            {buttons}
            {end_caps()}
        }
    }
}
