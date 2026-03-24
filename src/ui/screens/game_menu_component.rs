use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::{AnchorPoint, FrameName};
use crate::ui::strata::FrameStrata;

struct DynName(String);

pub const GAME_MENU_ROOT: FrameName = FrameName("GameMenuRoot");
const MENU_PANEL: FrameName = FrameName("GameMenuPanel");
const TITLE_FRAME: FrameName = FrameName("GameMenuTitleFrame");
const TITLE_LABEL: FrameName = FrameName("GameMenuTitleLabel");

const BUTTON_ATLAS_UP: &str = "defaultbutton-nineslice-up";
const BUTTON_ATLAS_PRESSED: &str = "defaultbutton-nineslice-pressed";
const BUTTON_ATLAS_HIGHLIGHT: &str = "defaultbutton-nineslice-highlight";
const BUTTON_ATLAS_DISABLED: &str = "defaultbutton-nineslice-disabled";

const BUTTON_W: f32 = 200.0;
const BUTTON_H: f32 = 36.0;
const PANEL_W: f32 = 260.0;
const SECTION_GAP: f32 = 8.0;

// Button onclick actions (matched in game_menu_screen.rs)
pub const ACTION_OPTIONS: &str = "menu_options";
pub const ACTION_SUPPORT: &str = "menu_support";
pub const ACTION_MACROS: &str = "menu_macros";
pub const ACTION_LOGOUT: &str = "menu_logout";
pub const ACTION_EXIT: &str = "menu_exit";
pub const ACTION_RESUME: &str = "menu_resume";

#[derive(Clone, Copy)]
pub enum PanelTitleStyle {
    TitleBar,
    Overlay,
}

pub fn panel_title(
    frame_name: FrameName,
    label_name: FrameName,
    panel_name: FrameName,
    text: &str,
    width: f32,
    style: PanelTitleStyle,
) -> Element {
    let frame_dn = DynName(frame_name.0.to_string());
    let label_dn = DynName(label_name.0.to_string());
    let text = text.to_string();
    let (pt, rp, y) = match style {
        PanelTitleStyle::TitleBar => (AnchorPoint::Bottom, AnchorPoint::Top, -2.0),
        PanelTitleStyle::Overlay => (AnchorPoint::Center, AnchorPoint::Top, 0.0),
    };
    let label = title_label(&label_dn, &text, width - 20.0, frame_name);
    rsx! {
        panel {
            name: {&frame_dn},
            width: {width},
            height: 36.0,
            strata: FrameStrata::Fullscreen,
            frame_level: 10.0,
            anchor {
                point: {pt},
                relative_to: panel_name,
                relative_point: {rp},
                y: {y},
            }
            {label}
        }
    }
}

fn title_label(label: &DynName, text: &str, w: f32, parent: FrameName) -> Element {
    rsx! {
        fontstring {
            name: {label},
            text: {text},
            font_size: 20.0,
            color: "0.96,0.84,0.56,1.0",
            width: {w},
            height: 30.0,
            justify_h: "CENTER",
            frame_level: 100.0,
            draw_layer: "OVERLAY",
            anchor {
                point: AnchorPoint::Center,
                relative_to: parent,
                relative_point: AnchorPoint::Center,
            }
        }
    }
}

fn flex_button(name: &str, text: &str, action: &str) -> Element {
    let n = DynName(name.to_string());
    let text = text.to_string();
    let action = action.to_string();
    rsx! {
        button {
            name: {&n},
            width: BUTTON_W,
            height: BUTTON_H,
            text: {&text},
            font_size: 16.0,
            strata: FrameStrata::Fullscreen,
            frame_level: 20.0,
            onclick: {&action},
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
        }
    }
}

fn section_spacer(name: &str) -> Element {
    let n = DynName(name.to_string());
    rsx! { r#frame { name: {&n}, width: BUTTON_W, height: SECTION_GAP } }
}

pub fn game_menu_screen(_shared: &SharedContext) -> Element {
    let style = PanelTitleStyle::TitleBar;
    let title = panel_title(TITLE_FRAME, TITLE_LABEL, MENU_PANEL, "Game Menu", PANEL_W, style);
    rsx! {
        r#frame {
            name: GAME_MENU_ROOT,
            stretch: true,
            background_color: "0.02,0.02,0.03,0.85",
            strata: FrameStrata::Background,
            panel {
                name: MENU_PANEL,
                width: PANEL_W,
                height: 0.0,
                strata: FrameStrata::Dialog,
                layout: "flex-column",
                align: "center",
                padding: 28.0,
                gap: 0.0,
                anchor {
                    point: AnchorPoint::Center,
                    relative_point: AnchorPoint::Center,
                }
                {flex_button("MenuBtnOptions", "Options", ACTION_OPTIONS)}
                {section_spacer("Spacer1")}
                {flex_button("MenuBtnSupport", "Support", ACTION_SUPPORT)}
                {flex_button("MenuBtnMacros", "Macros", ACTION_MACROS)}
                {section_spacer("Spacer2")}
                {flex_button("MenuBtnLogout", "Log Out", ACTION_LOGOUT)}
                {flex_button("MenuBtnExit", "Exit Game", ACTION_EXIT)}
                {section_spacer("Spacer3")}
                {flex_button("MenuBtnResume", "Return to Game", ACTION_RESUME)}
            }
            {title}
        }
    }
}
