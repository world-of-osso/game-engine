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

#[derive(Clone, Copy)]
pub enum PanelTitleStyle {
    /// Separate panel strip above the main panel.
    TitleBar,
    /// Panel strip centered on the top border (half in, half out).
    Overlay,
}

/// Panel title as a narrow panel strip positioned relative to the parent.
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
    let (anchor_point, anchor_rel, y_off) = title_anchor(style);
    let label = title_label(&label_dn, &text, width - 20.0, frame_name);
    rsx! {
        panel {
            name: {&frame_dn},
            width: {width},
            height: 36.0,
            strata: FrameStrata::Dialog,
            frame_level: 10.0,
            anchor {
                point: {anchor_point},
                relative_to: panel_name,
                relative_point: {anchor_rel},
                y: {y_off},
            }
            {label}
        }
    }
}

fn title_anchor(style: PanelTitleStyle) -> (AnchorPoint, AnchorPoint, f32) {
    match style {
        PanelTitleStyle::TitleBar => (AnchorPoint::Bottom, AnchorPoint::Top, -2.0),
        PanelTitleStyle::Overlay => (AnchorPoint::Center, AnchorPoint::Top, 0.0),
    }
}

fn title_label(label: &DynName, text: &str, width: f32, parent: FrameName) -> Element {
    rsx! {
        fontstring {
            name: {label},
            text: {text},
            font_size: 20.0,
            color: "0.96,0.84,0.56,1.0",
            width: {width},
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

fn menu_panel() -> Element {
    rsx! {
        panel {
            name: MENU_PANEL,
            width: 340.0,
            height: 420.0,
            strata: FrameStrata::Dialog,
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
            }
        }
    }
}

pub fn game_menu_screen(_shared: &SharedContext) -> Element {
    let style = PanelTitleStyle::TitleBar;
    let body = menu_panel();
    let title = panel_title(TITLE_FRAME, TITLE_LABEL, MENU_PANEL, "Game Menu", 340.0, style);
    rsx! {
        r#frame {
            name: GAME_MENU_ROOT,
            stretch: true,
            background_color: "0.02,0.02,0.03,0.85",
            strata: FrameStrata::Background,
            {body} {title}
        }
    }
}
