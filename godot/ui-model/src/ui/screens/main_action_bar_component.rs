//! Retail `MainActionBar` (`Blizzard_ActionBar/Mainline/MainActionBar.xml`,
//! `ActionButtonTemplate.xml`) with the Modern Edit Mode preset
//! (`Blizzard_EditMode/Mainline/EditModePresetLayouts.lua`): 12 buttons of 45×45,
//! 2 px apart (`minButtonPadding`) at the active preset's anchor (`crate::hud_layout`),
//! gryphon end caps, keys 1..=. Art names Blizzard atlas elements, which the active skin
//! resolves; under Forever the buttons take FlareUI's scale and art and the end caps are
//! the project's class shields. `MultiBarBottomLeft` and `MultiBarBottomRight`
//! (`Shared/MultiActionBars.xml`) are further instances of the same bar, shown where the
//! preset enables them.

use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::hud_layout::{ActionBarLayout, FOREVER_ACTION_BUTTON_SCALE, HudLayout, hud_layout};
use crate::input_bindings::{InputAction, InputBinding, InputBindingsData, InputState};
use crate::ui::anchor::FrameName;
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::GameFont;

pub const MAIN_ACTION_BAR: FrameName = FrameName("MainActionBar");
/// Buttons of every action bar (`NUM_ACTIONBAR_BUTTONS`, `numButtons`).
pub const MAIN_BAR_BUTTONS: usize = 12;
/// Clicking main bar button `n` (0-based) emits `"{ACTION_BUTTON_PREFIX}{n}"`.
pub const ACTION_BUTTON_PREFIX: &str = "action_button:";

pub const BUTTON_SIZE: f32 = 45.0;
pub const BUTTON_PADDING: f32 = 2.0;
/// Modern `MAIN_ACTION_BAR_DEFAULT_OFFSET_Y` (Standard/EditModePresetLayoutConstants.lua:2).
pub const BAR_BOTTOM: f32 = 45.0;
/// `NormalTexture`/`PushedTexture`/`HighlightTexture` are 46×45 at TOPLEFT.
const FRAME_ART_W: f32 = 46.0;
/// `Cooldown` is inset 3 px from the icon.
const COOLDOWN_INSET: f32 = 3.0;
/// Cooldown `SwipeTexture` colour 0,0,0,0.8.
const COOLDOWN_SWIPE: &str = "0.0,0.0,0.0,0.8";
/// `HotKey` at TOPRIGHT -4,-5 (`hotkeyTextKeyboardX/Y`).
const HOTKEY_ANCHOR: (f32, f32) = (4.0, 5.0);
/// `HotKey` (`ActionButtonTemplate.xml:85-91`): 32×10, `justifyH="RIGHT"`, one line (its
/// height holds no second), `NumberFontNormalSmallGray` grey (`GameFontStyles.xml:21-23`).
const HOTKEY_W: f32 = 32.0;
const HOTKEY_H: f32 = 10.0;
const HOTKEY_COLOR: &str = "0.6,0.6,0.6,1.0";
/// FlareUI `hotkeyFont` (Core.lua:224): Arial Narrow 12 OUTLINE at TOPRIGHT -4,-4 with a
/// black 1,-1 shadow (Modules/ActionBars.lua:263,274-275; Core.lua:447-451) on every
/// button. FlareUI sets no colour, so Forever's runtime `ACTIONBAR_HOTKEY_FONT_COLOR`
/// (`ActionButton.lua:1258`) shows: 210/255 grey-white in the user's reference capture.
const FOREVER_HOTKEY_ANCHOR: (f32, f32) = (4.0, 4.0);
const FOREVER_HOTKEY_COLOR: &str = "0.82,0.82,0.82,1.0";
/// `CooldownFrameTemplate` countdown numbers.
const COOLDOWN_TEXT_COLOR: &str = "1.0,1.0,1.0,1.0";

/// Every chrome sheet the Modern bar draws (`uiactionbar`), for hosts that copy art out
/// of local CASC.
pub const ACTION_BAR_ART_FDIDS: [u32; 1] = [4_613_342];

const SLOT_BACKGROUND: &str = "UI-HUD-ActionBar-IconFrame-Background";
const SLOT_ART: &str = "UI-HUD-ActionBar-IconFrame-Slot";
const NORMAL: &str = "UI-HUD-ActionBar-IconFrame";
/// `SlotBackground`, `SlotArt` and the icon are the button's `BACKGROUND` layer
/// (`ActionButtonTemplate.xml:22-33`), under its `NormalTexture`/`PushedTexture` border
/// whenever the icon is created.
const SLOT_LAYER: &str = "BACKGROUND";
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
    skin: ActiveSkin,
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
            skin,
            scale: 1.0,
            slot_background: true,
        },
        ActiveSkin::Forever => BarStyle {
            skin,
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
    /// The button's binding; its `HotKey` shows it abbreviated per skin, nothing when unbound.
    pub hotkey: Option<InputBinding>,
}

/// The action bars this client draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionBar {
    Main,
    /// Action Bar 2.
    BottomLeft,
    /// Action Bar 3.
    BottomRight,
    /// Action Bars 4/5, the right-hand vertical columns.
    Right,
    Left,
}

impl ActionBar {
    pub const ALL: [Self; 5] = [
        Self::Main,
        Self::BottomLeft,
        Self::BottomRight,
        Self::Right,
        Self::Left,
    ];

    fn frame_name(self) -> &'static str {
        match self {
            Self::Main => MAIN_ACTION_BAR.0,
            Self::BottomLeft => "MultiBarBottomLeft",
            Self::BottomRight => "MultiBarBottomRight",
            Self::Right => "MultiBarRight",
            Self::Left => "MultiBarLeft",
        }
    }

    /// `ActionButton<n>` on the main bar, `<bar>Button<n>` elsewhere (`ActionBar.lua:19-28`).
    fn button_prefix(self) -> &'static str {
        match self {
            Self::Main => "ActionButton",
            Self::BottomLeft => "MultiBarBottomLeftButton",
            Self::BottomRight => "MultiBarBottomRightButton",
            Self::Right => "MultiBarRightButton",
            Self::Left => "MultiBarLeftButton",
        }
    }

    /// Retail name of button `index` (0-based).
    pub fn button_name(self, index: usize) -> String {
        format!("{}{}", self.button_prefix(), index + 1)
    }

    /// The bar and 0-based index of the button named `name`.
    pub fn of_button_name(name: &str) -> Option<(Self, usize)> {
        Self::ALL.into_iter().find_map(|bar| {
            let number: usize = name.strip_prefix(bar.button_prefix())?.parse().ok()?;
            Some((bar, number.checked_sub(1)?)).filter(|&(_, index)| index < MAIN_BAR_BUTTONS)
        })
    }

    fn click_prefix(self) -> &'static str {
        match self {
            Self::Main => ACTION_BUTTON_PREFIX,
            Self::BottomLeft => "multi_bar_1_button:",
            Self::BottomRight => "multi_bar_2_button:",
            Self::Right => "multi_bar_3_button:",
            Self::Left => "multi_bar_4_button:",
        }
    }

    /// The bar's `actionpage`: the main bar's first page (which the player's form re-pages),
    /// `BOTTOMLEFT_ACTIONBAR_PAGE` 6, `BOTTOMRIGHT_ACTIONBAR_PAGE` 5
    /// (`MultiActionBars.xml:62,91`, `MultiActionBars.lua:3-4`).
    const fn page(self) -> usize {
        match self {
            Self::Main => 1,
            Self::BottomLeft => 6,
            Self::BottomRight => 5,
            Self::Right => 3,
            Self::Left => 4,
        }
    }

    /// 0-based action slot of button `index` on the bar's own page:
    /// `(page - 1) * NUM_ACTIONBAR_BUTTONS + id` (`ActionButtonUtil.lua:3,66`), so Retail's
    /// slots 1-12, 61-72 and 49-60.
    pub const fn action_slot(self, index: usize) -> usize {
        (self.page() - 1) * MAIN_BAR_BUTTONS + index
    }

    /// The binding each button runs: `ACTIONBUTTONn` on the main bar,
    /// `MULTIACTIONBAR1BUTTONn` / `MULTIACTIONBAR2BUTTONn` on Action Bar 2 / 3
    /// (`ActionBarActionButtonMixin:UpdateHotkeys`, `ActionButton.lua:470-486`;
    /// `MultiActionBars.xml` `buttonType`).
    pub const fn binding_actions(self) -> &'static [InputAction] {
        match self {
            Self::Main => &InputAction::ACTION_BAR_BUTTONS,
            Self::BottomLeft => &InputAction::MULTI_ACTION_BAR_1,
            Self::BottomRight => &InputAction::MULTI_ACTION_BAR_2,
            Self::Right => &InputAction::MULTI_ACTION_BAR_3,
            Self::Left => &InputAction::MULTI_ACTION_BAR_4,
        }
    }

    /// Shared Retail ReceiveDrag destination; assignment is sent to the server by the host.
    pub fn assignment(
        self,
        index: usize,
        action: shared::protocol::ActionRef,
        bonus_offset: u8,
    ) -> Option<shared::protocol::SetActionButton> {
        if index >= MAIN_BAR_BUTTONS {
            return None;
        }
        let slot = if self == Self::Main && bonus_offset > 0 {
            (6 + usize::from(bonus_offset) - 1) * MAIN_BAR_BUTTONS + index
        } else {
            self.action_slot(index)
        };
        Some(shared::protocol::SetActionButton {
            slot: slot as u8,
            action: Some(action),
        })
    }

    /// The bar's Edit Mode settings under `layout`; `None` when the preset does not show it.
    fn layout(self, layout: &HudLayout) -> Option<&ActionBarLayout> {
        match self {
            Self::Main => Some(&layout.main_action_bar),
            Self::BottomLeft => layout.action_bar_2.as_ref(),
            Self::BottomRight => layout.action_bar_3.as_ref(),
            Self::Right | Self::Left => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MainActionBarState {
    /// The main bar's buttons.
    pub buttons: [ActionButtonView; MAIN_BAR_BUTTONS],
    /// `MultiBarBottomLeft`'s and `MultiBarBottomRight`'s buttons.
    pub multi_bars: [[ActionButtonView; MAIN_BAR_BUTTONS]; 4],
    pub extra_action_bars: game_engine_core::client_options_data::ExtraActionBars,
    /// The player's `ChrClasses` ID; `None` until the player's unit has replicated.
    pub player_class: Option<u8>,
}

impl Default for MainActionBarState {
    fn default() -> Self {
        Self {
            buttons: std::array::from_fn(|_| ActionButtonView::default()),
            multi_bars: std::array::from_fn(|_| {
                std::array::from_fn(|_| ActionButtonView::default())
            }),
            extra_action_bars: Default::default(),
            player_class: None,
        }
    }
}

impl MainActionBarState {
    pub fn bar(&self, bar: ActionBar) -> &[ActionButtonView; MAIN_BAR_BUTTONS] {
        match bar {
            ActionBar::Main => &self.buttons,
            ActionBar::BottomLeft => &self.multi_bars[0],
            ActionBar::BottomRight => &self.multi_bars[1],
            ActionBar::Right => &self.multi_bars[2],
            ActionBar::Left => &self.multi_bars[3],
        }
    }

    pub fn bar_mut(&mut self, bar: ActionBar) -> &mut [ActionButtonView; MAIN_BAR_BUTTONS] {
        match bar {
            ActionBar::Main => &mut self.buttons,
            ActionBar::BottomLeft => &mut self.multi_bars[0],
            ActionBar::BottomRight => &mut self.multi_bars[1],
            ActionBar::Right => &mut self.multi_bars[2],
            ActionBar::Left => &mut self.multi_bars[3],
        }
    }

    /// `UpdateHotkeys` on every button: `GetBindingText(GetBindingKey(bindingAction), 1)`,
    /// empty for an unbound button (`ActionButton.lua:488-495`).
    pub fn set_hotkeys(&mut self, bindings: &InputBindingsData) {
        for bar in ActionBar::ALL {
            for (view, action) in self.bar_mut(bar).iter_mut().zip(bar.binding_actions()) {
                view.hotkey = bindings.binding(*action);
            }
        }
    }
}

/// Buttons whose binding went down this frame, on every bar (`ActionButtonDown`,
/// `MultiActionButtonDown`).
pub fn pressed_action_buttons(
    bindings: &InputBindingsData,
    input: &impl InputState,
) -> Vec<(ActionBar, usize)> {
    ActionBar::ALL
        .into_iter()
        .flat_map(|bar| {
            bar.binding_actions()
                .iter()
                .copied()
                .enumerate()
                .filter(|(_, action)| bindings.is_just_pressed(*action, input))
                .map(move |(index, _)| (bar, index))
        })
        .collect()
}

/// Bar and button index of an action this screen emitted.
pub fn parse_action_button(action: &str) -> Option<(ActionBar, usize)> {
    ActionBar::ALL.into_iter().find_map(|bar| {
        let index: usize = action.strip_prefix(bar.click_prefix())?.parse().ok()?;
        (index < MAIN_BAR_BUTTONS).then_some((bar, index))
    })
}

struct DynName(String);

/// `(x, y, width, height)` inside the parent.
type Rect = (f32, f32, f32, f32);

/// A button's border art, in the default `ARTWORK` layer.
fn art(name: String, atlas: &str, rect: Rect, hidden: bool) -> Element {
    layered_art(name, atlas, rect, hidden, "ARTWORK")
}

fn layered_art(
    name: String,
    atlas: &str,
    (x, y, width, height): Rect,
    hidden: bool,
    layer: &str,
) -> Element {
    rsx! {
        texture {
            name: {DynName(name)},
            width,
            height,
            hidden,
            texture_atlas: atlas,
            draw_layer: layer,
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
            draw_layer: SLOT_LAYER,
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

/// Button `name`'s `HotKey` showing `binding`, at the skin's anchor or the template's
/// `retail_anchor` (right, top). Modern labels it `GetBindingText(key, 1)` ("c-1"); Forever
/// with FlareUI's `ShortenKey` ("C1", `Modules/ActionBars.lua:259`).
pub(crate) fn hotkey(
    name: &str,
    binding: Option<InputBinding>,
    skin: ActiveSkin,
    scale: f32,
    retail_anchor: (f32, f32),
) -> Element {
    let label = binding.map_or_else(String::new, |binding| match skin {
        ActiveSkin::Modern => binding.hotkey_text(),
        ActiveSkin::Forever => binding.flare_hotkey_text(),
    });
    let label = label.as_str();
    let name = DynName(format!("{name}HotKey"));
    let (width, height, font_size) = (HOTKEY_W * scale, HOTKEY_H * scale, 12.0 * scale);
    match skin {
        ActiveSkin::Modern => rsx! {
            fontstring {
                name,
                width,
                height,
                text: label,
                font: GameFont::ArialNarrow,
                font_size,
                font_color: HOTKEY_COLOR,
                outline: "OUTLINE",
                justify_h: "RIGHT",
                pos_type: "absolute",
                right: {retail_anchor.0 * scale},
                pos_y: {retail_anchor.1 * scale},
            }
        },
        ActiveSkin::Forever => rsx! {
            fontstring {
                name,
                width,
                height,
                text: label,
                font: GameFont::ArialNarrow,
                font_size,
                font_color: FOREVER_HOTKEY_COLOR,
                outline: "OUTLINE",
                shadow_color: "0.0,0.0,0.0,1.0",
                shadow_offset: "1,-1",
                justify_h: "RIGHT",
                pos_type: "absolute",
                right: {FOREVER_HOTKEY_ANCHOR.0 * scale},
                pos_y: {FOREVER_HOTKEY_ANCHOR.1 * scale},
            }
        },
    }
}

/// Button `index` of `bar` at `(x, y)` inside it, drawn at `scale`.
fn button(
    (bar, index): (ActionBar, usize),
    view: &ActionButtonView,
    (x, y): (f32, f32),
    scale: f32,
    style: &BarStyle,
) -> Element {
    let name = bar.button_name(index);
    let size = BUTTON_SIZE * scale;
    let frame_art = (0.0, 0.0, FRAME_ART_W * scale, size);
    let cell = (0.0, 0.0, size, size);
    let children: Element = [
        layered_art(
            format!("{name}SlotBackground"),
            SLOT_BACKGROUND,
            cell,
            !style.slot_background,
            SLOT_LAYER,
        ),
        layered_art(format!("{name}SlotArt"), SLOT_ART, cell, false, SLOT_LAYER),
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
        hotkey(&name, view.hotkey, style.skin, scale, HOTKEY_ANCHOR),
    ]
    .into_iter()
    .flatten()
    .collect();
    rsx! {
        button {
            name: {DynName(name)},
            width: size,
            height: size,
            onclick: {format!("{}{index}", bar.click_prefix())},
            button_default_skin: false,
            pos_type: "absolute",
            pos_x: x,
            pos_y: y,
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

/// One bar: its `NumIcons` first buttons in the layout's grid, at the layout's anchor.
fn action_bar(
    bar: ActionBar,
    views: &[ActionButtonView],
    layout: &ActionBarLayout,
    style: &BarStyle,
    end_caps: Element,
) -> Element {
    let scale = style.scale * layout.icon_scale;
    let buttons: Element = views
        .iter()
        .take(layout.num_icons)
        .enumerate()
        .flat_map(|(index, view)| {
            let origin = if matches!(bar, ActionBar::Right | ActionBar::Left) {
                (0.0, index as f32 * (BUTTON_SIZE + BUTTON_PADDING) * scale)
            } else {
                layout.button_origin(index, style.scale)
            };
            button((bar, index), view, origin, scale, style)
        })
        .collect();
    let size = layout.size(style.scale);
    let at = layout.anchor.place(size);
    rsx! {
        r#frame {
            name: {DynName(bar.frame_name().into())},
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

pub fn main_action_bar_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<MainActionBarState>()
        .expect("MainActionBarState must be in SharedContext");
    let skin = *ctx
        .get::<ActiveSkin>()
        .expect("canvas carries the active skin");
    let style = bar_style(skin);
    let hud = extra_bar_layout(ctx, state);
    let mut elements: Element = ActionBar::ALL
        .into_iter()
        .filter_map(|bar| Some((bar, bar.layout(&hud)?)))
        .flat_map(|(bar, layout)| {
            let size = layout.size(style.scale);
            let end_caps = match (bar, skin) {
                (ActionBar::Main, ActiveSkin::Modern) => gryphons(size),
                (ActionBar::Main, ActiveSkin::Forever) => class_shields(size, state.player_class),
                _ => Vec::new(),
            };
            action_bar(bar, state.bar(bar), layout, &style, end_caps)
        })
        .collect();
    elements.extend(side_action_bars(state, &style));
    elements
}

fn extra_bar_layout(ctx: &SharedContext, state: &MainActionBarState) -> HudLayout {
    let mut hud = hud_layout(ctx);
    let editing = ctx
        .get::<crate::hud_edit::EditModeActive>()
        .is_some_and(|edit| edit.0);
    let mut preview = hud.main_action_bar;
    preview.anchor.y += BUTTON_SIZE + BUTTON_PADDING;
    hud.action_bar_2 = bottom_bar_layout(
        hud.action_bar_2,
        preview,
        state.extra_action_bars.action_bar_2,
        editing,
    );
    preview.anchor.y += BUTTON_SIZE + BUTTON_PADDING;
    hud.action_bar_3 = bottom_bar_layout(
        hud.action_bar_3,
        preview,
        state.extra_action_bars.action_bar_3,
        editing,
    );
    hud
}

fn bottom_bar_layout(
    authored: Option<ActionBarLayout>,
    preview: ActionBarLayout,
    enabled: Option<bool>,
    editing: bool,
) -> Option<ActionBarLayout> {
    match enabled {
        Some(false) => None,
        Some(true) => Some(authored.unwrap_or(preview)),
        None if editing => Some(authored.unwrap_or(preview)),
        None => authored,
    }
}

fn side_action_bars(state: &MainActionBarState, style: &BarStyle) -> Element {
    use crate::hud_layout::{HudAnchor, Point};
    [
        (ActionBar::Right, state.extra_action_bars.action_bar_4, 6.0),
        (ActionBar::Left, state.extra_action_bars.action_bar_5, 53.0),
    ]
    .into_iter()
    .filter(|(_, enabled, _)| *enabled)
    .flat_map(|(bar, _, right)| {
        let layout = ActionBarLayout {
            anchor: HudAnchor {
                point: Point::Right,
                relative: Point::Right,
                x: -right,
                y: 0.0,
            },
            num_icons: MAIN_BAR_BUTTONS,
            num_rows: MAIN_BAR_BUTTONS,
            icon_scale: 1.0,
        };
        action_bar(bar, state.bar(bar), &layout, style, Vec::new())
    })
    .collect()
}
