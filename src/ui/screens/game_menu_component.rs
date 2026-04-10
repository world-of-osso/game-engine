use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use super::screen_title::framed_title;
use crate::ui::anchor::{AnchorPoint, FrameName};
use crate::ui::screens::options_menu_component::{OptionsViewModel, options_view};
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
const LOGGED_IN_BUTTON_COUNT: f32 = 6.0;
const LOGGED_OUT_BUTTON_COUNT: f32 = 5.0;
const LOGGED_IN_SPACER_COUNT: f32 = 3.0;
const LOGGED_OUT_SPACER_COUNT: f32 = 2.0;

pub const ACTION_OPTIONS: &str = "menu_options";
pub const ACTION_SUPPORT: &str = "menu_support";
pub const ACTION_ADDONS: &str = "menu_addons";
pub const ACTION_LOGOUT: &str = "menu_logout";
pub const ACTION_EXIT: &str = "menu_exit";
pub const ACTION_RESUME: &str = "menu_resume";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMenuView {
    MainMenu,
    Options,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameMenuViewModel {
    pub logged_in: bool,
    pub view: GameMenuView,
    pub options: OptionsViewModel,
}

fn panel_title(text: &str) -> Element {
    framed_title(TITLE_FRAME, TITLE_LABEL, MENU_MOUNT, PANEL_W, text)
}

fn menu_button(name: &str, text: &str, action: &str) -> Element {
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
    rsx! {
        r#frame {
            name: {DynName(name.to_string())},
            width: BUTTON_W,
            height: SECTION_GAP,
        }
    }
}

fn menu_buttons(logged_in: bool) -> Element {
    let mut items = vec![
        menu_button("MenuBtnOptions", "Options", ACTION_OPTIONS),
        section_spacer("Spacer1"),
        menu_button("MenuBtnSupport", "Support", ACTION_SUPPORT),
        menu_button("MenuBtnAddons", "AddOns", ACTION_ADDONS),
        section_spacer("Spacer2"),
    ];
    if logged_in {
        items.push(menu_button("MenuBtnLogout", "Log Out", ACTION_LOGOUT));
    }
    items.extend([
        menu_button("MenuBtnExit", "Exit Game", ACTION_EXIT),
        section_spacer("Spacer3"),
        menu_button("MenuBtnResume", "Return to Game", ACTION_RESUME),
    ]);
    items.into_iter().flatten().collect()
}

fn menu_panel(logged_in: bool) -> Element {
    let y = (-(TITLE_H - TITLE_PANEL_OVERLAP)).to_string();
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
                y: {y},
            }
            {menu_buttons(logged_in)}
        }
    }
}

fn menu_mount_height(logged_in: bool) -> f32 {
    let (buttons, spacers) = if logged_in {
        (LOGGED_IN_BUTTON_COUNT, LOGGED_IN_SPACER_COUNT)
    } else {
        (LOGGED_OUT_BUTTON_COUNT, LOGGED_OUT_SPACER_COUNT)
    };
    let items = buttons + spacers;
    let gaps = items - 1.0;
    TITLE_H
        + (buttons * BUTTON_H)
        + (spacers * SECTION_GAP)
        + (gaps * PANEL_GAP)
        + (PANEL_PADDING * 2.0)
        - TITLE_PANEL_OVERLAP
}

fn main_menu_view(logged_in: bool) -> Element {
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
                height: {menu_mount_height(logged_in)},
                anchor {
                    point: AnchorPoint::Center,
                    relative_point: AnchorPoint::Center,
                }
                {menu_panel(logged_in)}
                {panel_title("Game Menu")}
            }
        }
    }
}

pub fn game_menu_screen(shared: &SharedContext) -> Element {
    let Some(model) = shared.get::<GameMenuViewModel>() else {
        return Vec::new();
    };
    match model.view {
        GameMenuView::MainMenu => main_menu_view(model.logged_in),
        GameMenuView::Options => options_menu_overlay(&model.options),
    }
}

fn options_menu_overlay(options: &OptionsViewModel) -> Element {
    let options = options_view(options);
    rsx! {
        r#frame {
            name: GAME_MENU_ROOT,
            stretch: true,
            background_color: "0.01,0.01,0.02,0.75",
            strata: FrameStrata::Dialog,
            mouse_enabled: true,
            {options}
        }
    }
}

#[cfg(test)]
#[path = "game_menu_component_tests.rs"]
mod tests;
