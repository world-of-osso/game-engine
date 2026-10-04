//! Retail `MainActionBar` (`Blizzard_ActionBar/Mainline/MainActionBar.xml`,
//! `ActionButtonTemplate.xml`) with the Modern Edit Mode preset
//! (`Blizzard_EditMode/Mainline/EditModePresetLayouts.lua`): 12 buttons of 45×45,
//! 2 px apart (`minButtonPadding`) at the active preset's anchor (`crate::hud_layout`),
//! gryphon end caps, keys 1..=. Art names Blizzard atlas elements, which the active skin
//! resolves; under Forever the buttons take FlareUI's scale and art and the end caps are
//! the project's class shields.

use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::hud_layout::{FOREVER_ACTION_BUTTON_SCALE, hud_layout};
use crate::ui::anchor::FrameName;
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
/// Modern `MAIN_ACTION_BAR_DEFAULT_OFFSET_Y` (Standard/EditModePresetLayoutConstants.lua:2).
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

/// Every chrome sheet the Modern bar draws (`uiactionbar`), for hosts that copy art out
/// of local CASC.
pub const ACTION_BAR_ART_FDIDS: [u32; 1] = [4_613_342];

const SLOT_BACKGROUND: &str = "UI-HUD-ActionBar-IconFrame-Background";
const SLOT_ART: &str = "UI-HUD-ActionBar-IconFrame-Slot";
const NORMAL: &str = "UI-HUD-ActionBar-IconFrame";
const PUSHED: &str = "UI-HUD-ActionBar-IconFrame-Down";
const HIGHLIGHT: &str = "UI-HUD-ActionBar-IconFrame-Mouseover";
const GRYPHON_LEFT: &str = "ui-hud-actionbar-gryphon-left";
const GRYPHON_RIGHT: &str = "ui-hud-actionbar-gryphon-right";
const LEFT_END_CAP: &str = "MainActionBarLeftEndCap";
const RIGHT_END_CAP: &str = "MainActionBarRightEndCap";
/// Project art (`data/ui/endcaps/README.md`): `left/<class>.ktx2` and its mirror
/// `right/<class>.ktx2`, 68×256 px, drawn 95 units tall.
const CLASS_SHIELD_DIR: &str = "data/ui/endcaps";
const CLASS_SHIELD_H: f32 = 95.0;
const CLASS_SHIELD_W: f32 = CLASS_SHIELD_H * 68.0 / 256.0;
/// Space between a shield and the nearest button.
const CLASS_SHIELD_GAP: f32 = 6.0;

/// How a skin draws the bar.
struct BarStyle {
    /// Every button's `SetScale`.
    scale: f32,
    /// Whether `SlotBackground` shows under the icon.
    slot_background: bool,
}

/// FlareUI's defaults (Core.lua:207,218): `buttonArt` forces `SlotArt` over a hidden
/// `SlotBackground` with the `UI-HUD-ActionBar-IconFrame`/`-Down` atlases at 46×45, and
/// every bar's buttons take scale 1.06 (Modules/ActionBars.lua:141-157,186).
fn bar_style(skin: ActiveSkin) -> BarStyle {
    match skin {
        ActiveSkin::Modern => BarStyle {
            scale: 1.0,
            slot_background: true,
        },
        ActiveSkin::Forever => BarStyle {
            scale: FOREVER_ACTION_BUTTON_SCALE,
            slot_background: false,
        },
    }
}

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
    /// The player's `ChrClasses` ID; `None` until the player's unit has replicated.
    pub player_class: Option<u8>,
}

impl Default for MainActionBarState {
    fn default() -> Self {
        Self {
            buttons: std::array::from_fn(|_| ActionButtonView::default()),
            player_class: None,
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

/// `(x, y, width, height)` inside the parent.
type Rect = (f32, f32, f32, f32);

fn art(name: String, atlas: &str, (x, y, width, height): Rect, hidden: bool) -> Element {
    rsx! {
        texture {
            name: {DynName(name)},
            width,
            height,
            hidden,
            texture_atlas: atlas,
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
        }
    }
}

fn icon(name: String, fdid: u32, size: f32) -> Element {
    // Retail ActionButton.lua:620-628 draws an icon only when the action has a texture.
    if fdid == 0 {
        return Vec::new();
    }
    rsx! {
        texture {
            name: {DynName(name)},
            width: size,
            height: size,
            texture_fdid: {fdid},
            pos_type: "absolute",
            pos_x: 0.0,
            pos_y: 0.0,
        }
    }
}

/// The swipe covers the icon's remaining fraction, draining downward.
fn cooldown(name: &str, view: &ActionButtonView, scale: f32) -> Element {
    let size = BUTTON_SIZE * scale;
    let inset = COOLDOWN_INSET * scale;
    let side = size - 2.0 * inset;
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
            pos_x: inset,
            pos_y: {size - inset - height},
        }
        fontstring {
            name: text,
            width: size,
            height: size,
            text: {view.cooldown_text.as_str()},
            font: GameFont::FrizQuadrata,
            font_size: {16.0 * scale},
            font_color: COOLDOWN_TEXT_COLOR,
            outline: "OUTLINE",
            justify_h: "CENTER",
            pos_type: "absolute",
            pos_x: 0.0,
            pos_y: 0.0,
        }
    }
}

fn hotkey(name: &str, index: usize, scale: f32) -> Element {
    rsx! {
        fontstring {
            name: {DynName(format!("{name}HotKey"))},
            width: {HOTKEY_W * scale},
            height: {HOTKEY_H * scale},
            text: {hotkey_label(index)},
            font: GameFont::ArialNarrow,
            font_size: {12.0 * scale},
            font_color: HOTKEY_COLOR,
            outline: "OUTLINE",
            justify_h: "RIGHT",
            pos_type: "absolute",
            right: {HOTKEY_RIGHT * scale},
            pos_y: {HOTKEY_TOP * scale},
        }
    }
}

fn button(index: usize, view: &ActionButtonView, style: &BarStyle) -> Element {
    let name = action_button_name(index);
    let scale = style.scale;
    let size = BUTTON_SIZE * scale;
    let x = index as f32 * (BUTTON_SIZE + BUTTON_PADDING) * scale;
    let frame_art = (0.0, 0.0, FRAME_ART_W * scale, size);
    let cell = (0.0, 0.0, size, size);
    let children: Element = [
        art(
            format!("{name}SlotBackground"),
            SLOT_BACKGROUND,
            cell,
            !style.slot_background,
        ),
        art(format!("{name}SlotArt"), SLOT_ART, cell, false),
        icon(format!("{name}Icon"), view.icon_fdid, size),
        cooldown(&name, view, scale),
        art(
            format!("{name}NormalTexture"),
            NORMAL,
            frame_art,
            view.pushed,
        ),
        art(
            format!("{name}PushedTexture"),
            PUSHED,
            frame_art,
            !view.pushed,
        ),
        art(
            format!("{name}HighlightTexture"),
            HIGHLIGHT,
            frame_art,
            !view.hovered,
        ),
        hotkey(&name, index, scale),
    ]
    .into_iter()
    .flatten()
    .collect();
    rsx! {
        button {
            name: {DynName(name)},
            width: size,
            height: size,
            onclick: {format!("{ACTION_BUTTON_PREFIX}{index}")},
            button_default_skin: false,
            pos_type: "absolute",
            pos_x: x,
            pos_y: 0.0,
            {children}
        }
    }
}

/// Mainline MainMenuBarEndCaps.xml: 104.5×98 gryphons, BOTTOMRIGHT of the left one at the
/// bar's BOTTOMLEFT +9,-22; the right one's BOTTOMLEFT at its BOTTOMRIGHT -8,-22.
fn gryphons((width, height): (f32, f32)) -> Element {
    let (cap_w, cap_h) = (104.5, 98.0);
    let top = height + 22.0 - cap_h;
    [
        art(
            LEFT_END_CAP.into(),
            GRYPHON_LEFT,
            (9.0 - cap_w, top, cap_w, cap_h),
            false,
        ),
        art(
            RIGHT_END_CAP.into(),
            GRYPHON_RIGHT,
            (width - 8.0, top, cap_w, cap_h),
            false,
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// File name of `ChrClasses` ID `class`'s shield.
fn class_shield_name(class: u8) -> Option<&'static str> {
    let names = [
        "warrior",
        "paladin",
        "hunter",
        "rogue",
        "priest",
        "deathknight",
        "shaman",
        "mage",
        "warlock",
        "monk",
        "druid",
        "demonhunter",
        "evoker",
    ];
    names.get(usize::from(class).checked_sub(1)?).copied()
}

fn class_shield(name: &str, side: &str, class: &str, x: f32, bar_height: f32) -> Element {
    let file = format!("{CLASS_SHIELD_DIR}/{side}/{class}.ktx2");
    rsx! {
        texture {
            name: {DynName(name.into())},
            width: CLASS_SHIELD_W,
            height: CLASS_SHIELD_H,
            texture_file: {file.as_str()},
            pos_type: "absolute",
            pos_x: x,
            pos_y: {bar_height - CLASS_SHIELD_H},
        }
    }
}

/// Forever's end caps (user decision 2026-10-03): the project's narrow shields carrying the
/// player's class emblem, bottoms on the bar's bottom edge, outside the first and last
/// button. None while the class is unknown or has no shield.
fn class_shields((width, height): (f32, f32), class: Option<u8>) -> Element {
    let Some(class) = class.and_then(class_shield_name) else {
        return Vec::new();
    };
    let left = -CLASS_SHIELD_GAP - CLASS_SHIELD_W;
    let right = width + CLASS_SHIELD_GAP;
    [
        class_shield(LEFT_END_CAP, "left", class, left, height),
        class_shield(RIGHT_END_CAP, "right", class, right, height),
    ]
    .into_iter()
    .flatten()
    .collect()
}

pub fn main_action_bar_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<MainActionBarState>()
        .expect("MainActionBarState must be in SharedContext");
    let skin = *ctx
        .get::<ActiveSkin>()
        .expect("canvas carries the active skin");
    let style = bar_style(skin);
    let buttons: Element = state
        .buttons
        .iter()
        .enumerate()
        .flat_map(|(index, view)| button(index, view, &style))
        .collect();
    let size = (BAR_W * style.scale, BUTTON_SIZE * style.scale);
    let at = hud_layout(ctx).main_action_bar.place(size);
    let end_caps = match skin {
        ActiveSkin::Modern => gryphons(size),
        ActiveSkin::Forever => class_shields(size, state.player_class),
    };
    rsx! {
        r#frame {
            name: MAIN_ACTION_BAR,
            width: {size.0},
            height: {size.1},
            strata: FrameStrata::Medium,
            pos_type: "absolute",
            left: {at.left.as_str()},
            right: {at.right.as_str()},
            top: {at.top.as_str()},
            bottom: {at.bottom.as_str()},
            margin_left: {at.margin_left},
            margin_top: {at.margin_top},
            {buttons}
            {end_caps}
        }
    }
}
