//! Retail `ReadyCheckFrame` (Blizzard_FrameXML/Mainline/ReadyCheck.xml): 323×100 at
//! CENTER (0, -10) with the initiator's message and Ready / Not Ready buttons. The portrait
//! is left out (no unit portraits in this UI) and the border is the static popup's.

use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::ui::screens::static_popup_component::STATIC_POPUP_PANEL_STYLE;
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont};

pub const READY_CHECK_FRAME: &str = "ReadyCheckFrame";
pub const ACTION_READY_CHECK_YES: &str = "ready_check_yes";
pub const ACTION_READY_CHECK_NO: &str = "ready_check_no";

const FRAME_W: f32 = 323.0;
const FRAME_H: f32 = 100.0;
const CENTER_Y: f32 = 10.0;
/// `UI-DialogBox-Background-Dark`, inset (3, -23 / -3, 3) under the title bar.
const BACKGROUND_FDID: u32 = 6_839_810;
const TITLE_H: f32 = 20.0;
const TEXT_W: f32 = 240.0;
const TEXT_TOP: f32 = 37.0;
const BUTTON_W: f32 = 119.0;
const BUTTON_H: f32 = 24.0;
const BUTTON_BOTTOM: f32 = 16.0;
const BUTTON_GAP: f32 = 11.0;
const TITLE_COLOR: FontColor = FontColor::new(1.0, 0.82, 0.0, 1.0);
const TEXT_COLOR: FontColor = FontColor::new(1.0, 1.0, 1.0, 1.0);

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ReadyCheckFrameState {
    pub visible: bool,
    pub initiator: String,
}

/// Built only while shown, like `StaticPopupN`, so automation can wait for its buttons.
pub fn ready_check_frame(state: &ReadyCheckFrameState) -> Element {
    if !state.visible {
        return Element::default();
    }
    // Retail READY_CHECK_MESSAGE.
    let text = format!("{} has initiated a ready check.", state.initiator);
    rsx! {
        r#frame {
            name: {DynName(READY_CHECK_FRAME.to_string())},
            width: FRAME_W,
            height: FRAME_H,
            strata: FrameStrata::Dialog,
            mouse_enabled: true,
            pos_type: "absolute",
            left: "50%",
            top: "50%",
            translate_x: "-50%",
            translate_y: "-50%",
            margin_top: CENTER_Y,
            texture {
                name: {DynName(format!("{READY_CHECK_FRAME}Background"))},
                width: {FRAME_W - 6.0},
                height: {FRAME_H - 26.0},
                texture_fdid: BACKGROUND_FDID,
                strata: FrameStrata::Dialog,
                pos_type: "absolute",
                pos_x: 3.0,
                pos_y: 23.0,
            }
            r#frame {
                name: {DynName(format!("{READY_CHECK_FRAME}Border"))},
                width: FRAME_W,
                height: FRAME_H,
                style: STATIC_POPUP_PANEL_STYLE,
                strata: FrameStrata::Dialog,
                frame_level: 5.0,
                pos_type: "absolute",
                pos_x: 0.0,
                pos_y: 0.0,
            }
            fontstring {
                name: {DynName(format!("{READY_CHECK_FRAME}Title"))},
                width: FRAME_W,
                height: TITLE_H,
                text: "Ready Check",
                font: GameFont::FrizQuadrata,
                font_size: 12.0,
                font_color: TITLE_COLOR,
                justify_h: "CENTER",
                strata: FrameStrata::Dialog,
                frame_level: 6.0,
                pos_type: "absolute",
                pos_x: 0.0,
                pos_y: 4.0,
            }
            fontstring {
                name: {DynName(format!("{READY_CHECK_FRAME}Text"))},
                width: TEXT_W,
                height: 28.0,
                text: text.as_str(),
                font: GameFont::FrizQuadrata,
                font_size: 12.0,
                font_color: TEXT_COLOR,
                justify_h: "CENTER",
                strata: FrameStrata::Dialog,
                frame_level: 6.0,
                pos_type: "absolute",
                pos_x: {(FRAME_W - TEXT_W) / 2.0},
                pos_y: {TEXT_TOP - TITLE_H / 2.0},
            }
            {ready_button("YesButton", "Ready", ACTION_READY_CHECK_YES, FRAME_W / 2.0 - BUTTON_GAP - BUTTON_W)}
            {ready_button("NoButton", "Not Ready", ACTION_READY_CHECK_NO, FRAME_W / 2.0 + BUTTON_GAP)}
        }
    }
}

fn ready_button(suffix: &str, text: &str, onclick: &str, x: f32) -> Element {
    rsx! {
        button {
            name: {DynName(format!("{READY_CHECK_FRAME}{suffix}"))},
            width: BUTTON_W,
            height: BUTTON_H,
            text,
            font_size: 12.0,
            onclick,
            strata: FrameStrata::Dialog,
            frame_level: 10.0,
            button_atlas_up: "defaultbutton-nineslice-up",
            button_atlas_pressed: "defaultbutton-nineslice-pressed",
            button_atlas_highlight: "defaultbutton-nineslice-highlight",
            button_atlas_disabled: "defaultbutton-nineslice-disabled",
            pos_type: "absolute",
            pos_x: x,
            pos_y: {FRAME_H - BUTTON_BOTTOM - BUTTON_H},
        }
    }
}

struct DynName(String);

impl std::fmt::Display for DynName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
