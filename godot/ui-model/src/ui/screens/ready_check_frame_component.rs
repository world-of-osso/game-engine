//! Retail `ReadyCheckFrame` (Blizzard_FrameXML/Mainline/ReadyCheck.xml): 323×100 at
//! CENTER (0, -10) with the initiator's portrait, message and Ready / Not Ready buttons.

use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::panel_style_data::MetalTopLeft;
use crate::ui::screens::inworld_unit_frames_component::{FULL_PORTRAIT, PortraitSlot};
use crate::ui::screens::quest_art;
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont};

pub const READY_CHECK_FRAME: &str = "ReadyCheckFrame";
pub const ACTION_READY_CHECK_YES: &str = "ready_check_yes";
pub const ACTION_READY_CHECK_NO: &str = "ready_check_no";

/// ReadyCheck.xml:63–74, TOPLEFT(0,+3) in WoW's y-up coordinates.
/// Masking uses the existing native portrait renderer's shared circle mask.
pub const PORTRAIT: PortraitSlot = PortraitSlot {
    frame: "ReadyCheckPortrait",
    rect: (0.0, -3.0, 54.0, 54.0),
    mask_fdid: 130_924,
    mask_rect: (0.0, -3.0, 54.0, 54.0),
    tex_coords: FULL_PORTRAIT,
};

const FRAME_W: f32 = 323.0;
const FRAME_H: f32 = 100.0;
const CENTER_Y: f32 = 10.0;
/// `UI-DialogBox-Background-Dark`, inset (3, -23 / -3, 3) under the title bar.
const BACKGROUND_FDID: u32 = 6_839_810;
// ReadyCheck.xml:45–58: left58/right24; container top1 + TitleText top5.
const TITLE_W: f32 = 241.0;
const TITLE_LEFT: f32 = 58.0;
const TITLE_TOP: f32 = 6.0;
const TITLE_H: f32 = 20.0;
const TEXT_W: f32 = 240.0;
const TEXT_CENTER_X: f32 = 20.0;
const TEXT_TOP: f32 = 37.0;
const BUTTON_W: f32 = 119.0;
const BUTTON_H: f32 = 24.0;
const BUTTON_BOTTOM: f32 = 16.0;
const YES_RIGHT_OFFSET: f32 = 11.0;
const NO_LEFT_OFFSET: f32 = 22.0;
const TITLE_COLOR: FontColor = FontColor::new(1.0, 210.0 / 255.0, 0.0, 1.0);
const TEXT_COLOR: FontColor = FontColor::new(1.0, 210.0 / 255.0, 0.0, 1.0);

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
            {quest_art::metal_border(READY_CHECK_FRAME, FRAME_W, FRAME_H, MetalTopLeft::Portrait)}
            {quest_art::window_portrait(&PORTRAIT)}
            fontstring {
                name: {DynName(format!("{READY_CHECK_FRAME}Title"))},
                width: TITLE_W,
                height: TITLE_H,
                text: "Ready Check",
                font: GameFont::FrizQuadrata,
                font_size: 12.0,
                font_color: TITLE_COLOR,
                shadow_color: "0,0,0,1",
                shadow_offset: "1,-1",
                justify_h: "CENTER",
                strata: FrameStrata::Dialog,
                frame_level: 6.0,
                pos_type: "absolute",
                pos_x: TITLE_LEFT,
                pos_y: TITLE_TOP,
            }
            fontstring {
                name: {DynName(format!("{READY_CHECK_FRAME}Text"))},
                width: TEXT_W,
                height: 28.0,
                text: text.as_str(),
                font: GameFont::FrizQuadrata,
                font_size: 12.0,
                font_color: TEXT_COLOR,
                shadow_color: "0,0,0,1",
                shadow_offset: "1,-1",
                justify_h: "CENTER",
                strata: FrameStrata::Dialog,
                frame_level: 6.0,
                pos_type: "absolute",
                pos_x: {(FRAME_W - TEXT_W) / 2.0 + TEXT_CENTER_X},
                pos_y: TEXT_TOP,
            }
            {ready_button("YesButton", "Ready", ACTION_READY_CHECK_YES, FRAME_W / 2.0 + YES_RIGHT_OFFSET - BUTTON_W)}
            {ready_button("NoButton", "Not Ready", ACTION_READY_CHECK_NO, FRAME_W / 2.0 + NO_LEFT_OFFSET)}
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
            button_fdid_up: 130828_u32,
            button_fdid_pressed: 130825_u32,
            button_fdid_highlight: 130826_u32,
            button_fdid_disabled: 130824_u32,
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
