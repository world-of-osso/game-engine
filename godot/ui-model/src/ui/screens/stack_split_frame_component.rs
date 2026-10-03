//! `StackSplitFrame` (StackSplitFrame.xml): background, the amount between the
//! left/right arrows, and Okay / Cancel. Offsets below are the XML anchors
//! resolved against the 172×96 frame (172×120 for vendor bundles).

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::GameFont;

pub const FRAME_NAME: &str = "StackSplitFrame";
pub const FRAME_W: f32 = 172.0;
const SINGLE_H: f32 = 96.0;
const MULTI_H: f32 = 120.0;

pub const ACTION_LEFT: &str = "stack_split:left";
pub const ACTION_RIGHT: &str = "stack_split:right";
pub const ACTION_OKAY: &str = "stack_split:okay";
pub const ACTION_CANCEL: &str = "stack_split:cancel";

/// `Interface\MoneyFrame\UI-MoneyFrame`, texcoords 0-0.671875 × 0-0.75.
const SINGLE_BG: u32 = 136_494;
/// `UI-MoneyFrame-Large` in atlas 1667824 (256×128) at 1-173 × 1-121.
const MULTI_BG: u32 = 1_667_824;
const MULTI_BG_COORDS: &str = "0.00390625,0.67578125,0.0078125,0.9453125";
const ARROW_LEFT_UP: u32 = 136_487;
const ARROW_LEFT_DISABLED: u32 = 136_485;
const ARROW_RIGHT_UP: u32 = 136_490;
const ARROW_RIGHT_DISABLED: u32 = 136_488;
const BUTTON_W: f32 = 64.0;
const BUTTON_H: f32 = 24.0;
/// `GameFontHighlight` / `GameFontNormal`.
const HIGHLIGHT_FONT_COLOR: &str = "1.0,1.0,1.0,1.0";
const NORMAL_FONT_COLOR: &str = "1.0,0.82,0.0,1.0";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StackSplitFrameState {
    pub visible: bool,
    /// Top-left in UI coordinates, placed against the owner button.
    pub x: f32,
    pub y: f32,
    pub text: String,
    /// `StackItemCountText`, shown for vendor bundles only.
    pub total_text: Option<String>,
    pub left_enabled: bool,
    pub right_enabled: bool,
}

impl StackSplitFrameState {
    pub fn height(&self) -> f32 {
        if self.total_text.is_some() {
            MULTI_H
        } else {
            SINGLE_H
        }
    }
}

struct DynName(String);

impl std::fmt::Display for DynName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

pub fn stack_split_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<StackSplitFrameState>()
        .expect("StackSplitFrameState must be in SharedContext");
    let hide = !state.visible;
    let height = state.height();
    let mut children = background(state);
    children.extend(amount_texts(state));
    // LeftButton RIGHT at CENTER -59,18; RightButton LEFT at CENTER 64,18.
    let arrow_top = height / 2.0 - 18.0 - 8.0;
    children.extend(arrow(
        "LeftButton",
        FRAME_W / 2.0 - 59.0 - 16.0,
        arrow_top,
        state.left_enabled,
        (ARROW_LEFT_UP, ARROW_LEFT_DISABLED),
        ACTION_LEFT,
    ));
    children.extend(arrow(
        "RightButton",
        FRAME_W / 2.0 + 64.0,
        arrow_top,
        state.right_enabled,
        (ARROW_RIGHT_UP, ARROW_RIGHT_DISABLED),
        ACTION_RIGHT,
    ));
    // Okay RIGHT / Cancel LEFT at BOTTOM -3 / 5, 32 up (40 for bundles).
    let button_top = height
        - if state.total_text.is_some() {
            40.0
        } else {
            32.0
        }
        - BUTTON_H / 2.0;
    children.extend(button(
        "OkayButton",
        "Okay",
        FRAME_W / 2.0 - 3.0 - BUTTON_W,
        button_top,
        ACTION_OKAY,
    ));
    children.extend(button(
        "CancelButton",
        "Cancel",
        FRAME_W / 2.0 + 5.0,
        button_top,
        ACTION_CANCEL,
    ));
    rsx! {
        r#frame {
            name: {DynName(FRAME_NAME.into())},
            width: FRAME_W,
            height: {height},
            strata: FrameStrata::FullscreenDialog,
            hidden: hide,
            mouse_enabled: true,
            pos_type: "absolute",
            left: {state.x},
            top: {state.y},
            {children}
        }
    }
}

fn background(state: &StackSplitFrameState) -> Element {
    let (fdid, coords) = if state.total_text.is_some() {
        (MULTI_BG, MULTI_BG_COORDS)
    } else {
        (SINGLE_BG, "0.0,0.671875,0.0,0.75")
    };
    rsx! {
        texture {
            name: {DynName(format!("{FRAME_NAME}Background"))},
            width: FRAME_W,
            height: {state.height()},
            texture_fdid: fdid,
            tex_coords: coords,
            pos_type: "absolute",
            left: 0.0,
            top: 0.0,
        }
    }
}

/// `StackSplitText` RIGHT at -50,18 (single) or CENTER at 5,30 (bundles), with
/// `StackItemCountText` 25 below it.
fn amount_texts(state: &StackSplitFrameState) -> Element {
    let (width, height) = (100.0, 14.0);
    let (left, top, justify) = match state.total_text {
        Some(_) => (
            FRAME_W / 2.0 + 5.0 - width / 2.0,
            MULTI_H / 2.0 - 30.0 - height / 2.0,
            "CENTER",
        ),
        None => (
            FRAME_W - 50.0 - width,
            SINGLE_H / 2.0 - 18.0 - height / 2.0,
            "RIGHT",
        ),
    };
    let mut texts = label(
        "StackSplitText",
        &state.text,
        (left, top, width, height),
        HIGHLIGHT_FONT_COLOR,
        justify,
    );
    if let Some(total) = &state.total_text {
        texts.extend(label(
            "StackItemCountText",
            total,
            (left, top + 25.0, width, height),
            NORMAL_FONT_COLOR,
            "CENTER",
        ));
    }
    texts
}

fn label(
    name: &str,
    text: &str,
    (left, top, width, height): (f32, f32, f32, f32),
    color: &str,
    justify: &str,
) -> Element {
    rsx! {
        fontstring {
            name: {DynName(name.into())},
            width,
            height,
            text,
            font: GameFont::FrizQuadrata,
            font_size: 12.0,
            font_color: color,
            justify_h: justify,
            pos_type: "absolute",
            left,
            top,
        }
    }
}

fn arrow(
    key: &str,
    left: f32,
    top: f32,
    enabled: bool,
    (up, disabled): (u32, u32),
    action: &str,
) -> Element {
    let name = format!("{FRAME_NAME}{key}");
    let action = if enabled { action } else { "" };
    let fdid = if enabled { up } else { disabled };
    rsx! {
        r#frame {
            name: {DynName(name.clone())},
            width: 16.0,
            height: 16.0,
            onclick: action,
            mouse_enabled: true,
            pos_type: "absolute",
            left,
            top,
            texture {
                name: {DynName(format!("{name}Normal"))},
                width: 16.0,
                height: 16.0,
                texture_fdid: fdid,
                pos_type: "absolute",
                left: 0.0,
                top: 0.0,
            }
        }
    }
}

fn button(key: &str, text: &str, left: f32, top: f32, action: &str) -> Element {
    rsx! {
        button {
            name: {DynName(format!("{FRAME_NAME}{key}"))},
            width: BUTTON_W,
            height: BUTTON_H,
            text,
            font_size: 12.0,
            onclick: action,
            button_atlas_up: "defaultbutton-nineslice-up",
            button_atlas_pressed: "defaultbutton-nineslice-pressed",
            button_atlas_highlight: "defaultbutton-nineslice-highlight",
            button_atlas_disabled: "defaultbutton-nineslice-disabled",
            pos_type: "absolute",
            left,
            top,
        }
    }
}
