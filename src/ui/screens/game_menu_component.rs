use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::{AnchorPoint, FrameName};
use crate::ui::strata::FrameStrata;

struct DynName(String);

pub const GAME_MENU_ROOT: FrameName = FrameName("GameMenuRoot");
const MENU_MOUNT: FrameName = FrameName("GameMenuMount");
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
const PANEL_PADDING: f32 = 28.0;
const PANEL_GAP: f32 = 5.0;
const SECTION_GAP: f32 = 8.0;
const TITLE_H: f32 = 36.0;
const TITLE_PANEL_OVERLAP: f32 = 2.0;
const BUTTON_COUNT: f32 = 6.0;
const SPACER_COUNT: f32 = 3.0;

// Button onclick actions (matched in game_menu_screen.rs)
pub const ACTION_OPTIONS: &str = "menu_options";
pub const ACTION_SUPPORT: &str = "menu_support";
pub const ACTION_ADDONS: &str = "menu_addons";
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
    let relative_to = match style {
        PanelTitleStyle::TitleBar => MENU_MOUNT,
        PanelTitleStyle::Overlay => panel_name,
    };
    let (relative_to, pt, rp, y) = match style {
        PanelTitleStyle::TitleBar => (relative_to, AnchorPoint::Top, AnchorPoint::Top, 0.0),
        PanelTitleStyle::Overlay => (relative_to, AnchorPoint::Center, AnchorPoint::Top, 0.0),
    };
    let y = y.to_string();
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
                relative_to: {relative_to},
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

fn menu_panel_body() -> Element {
    let panel_y = (-(TITLE_H - TITLE_PANEL_OVERLAP)).to_string();
    rsx! {
        panel {
            name: MENU_PANEL,
            width: PANEL_W,
            height: 0.0,
            strata: FrameStrata::Fullscreen,
            layout: "flex-column",
            align: "center",
            padding: PANEL_PADDING,
            gap: PANEL_GAP,
            anchor {
                point: AnchorPoint::Top,
                relative_to: MENU_MOUNT,
                relative_point: AnchorPoint::Top,
                y: {panel_y},
            }
            {flex_button("MenuBtnOptions", "Options", ACTION_OPTIONS)}
            {section_spacer("Spacer1")}
            {flex_button("MenuBtnSupport", "Support", ACTION_SUPPORT)}
            {flex_button("MenuBtnAddons", "AddOns", ACTION_ADDONS)}
            {section_spacer("Spacer2")}
            {flex_button("MenuBtnLogout", "Log Out", ACTION_LOGOUT)}
            {flex_button("MenuBtnExit", "Exit Game", ACTION_EXIT)}
            {section_spacer("Spacer3")}
            {flex_button("MenuBtnResume", "Return to Game", ACTION_RESUME)}
        }
    }
}

fn menu_panel_height() -> f32 {
    let item_count = BUTTON_COUNT + SPACER_COUNT;
    let gap_count = item_count - 1.0;
    (BUTTON_COUNT * BUTTON_H)
        + (SPACER_COUNT * SECTION_GAP)
        + (gap_count * PANEL_GAP)
        + (PANEL_PADDING * 2.0)
}

fn menu_mount_height() -> f32 {
    TITLE_H + menu_panel_height() - TITLE_PANEL_OVERLAP
}

pub fn game_menu_screen(_shared: &SharedContext) -> Element {
    let title = panel_title(
        TITLE_FRAME,
        TITLE_LABEL,
        MENU_PANEL,
        "Game Menu",
        PANEL_W,
        PanelTitleStyle::TitleBar,
    );
    let body = menu_panel_body();
    rsx! {
        r#frame {
            name: GAME_MENU_ROOT,
            stretch: true,
            background_color: "0.01,0.01,0.02,0.75",
            strata: FrameStrata::Dialog,
            mouse_enabled: true,
            r#frame {
                name: MENU_MOUNT,
                width: PANEL_W,
                height: {menu_mount_height()},
                anchor {
                    point: AnchorPoint::Center,
                    relative_point: AnchorPoint::Center,
                }
                {body}
                {title}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use ui_toolkit::screen::Screen;

    use crate::ui::anchor::AnchorPoint;
    use crate::ui::registry::FrameRegistry;

    #[test]
    fn game_menu_title_is_anchored_to_mount_not_panel_flow() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let shared = SharedContext::new();
        Screen::new(game_menu_screen).sync(&shared, &mut reg);

        let mount_id = reg.get_by_name(MENU_MOUNT.0).expect("GameMenuMount");
        let title_id = reg.get_by_name(TITLE_FRAME.0).expect("GameMenuTitleFrame");
        let panel_id = reg.get_by_name(MENU_PANEL.0).expect("GameMenuPanel");

        let title = reg.get(title_id).expect("title frame");
        assert_eq!(title.anchors.len(), 1);
        assert_eq!(title.anchors[0].relative_to, Some(mount_id));
        assert_eq!(title.anchors[0].point, AnchorPoint::Top);
        assert_eq!(title.anchors[0].relative_point, AnchorPoint::Top);
        assert_eq!(title.anchors[0].y_offset, 0.0);

        let panel = reg.get(panel_id).expect("panel frame");
        assert_eq!(panel.anchors.len(), 1);
        assert_eq!(panel.anchors[0].relative_to, Some(mount_id));
        assert_eq!(panel.anchors[0].point, AnchorPoint::Top);
        assert_eq!(panel.anchors[0].relative_point, AnchorPoint::Top);
        assert_eq!(panel.anchors[0].y_offset, -(TITLE_H - TITLE_PANEL_OVERLAP));
    }
}
