use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::{AnchorPoint, FrameName};
use crate::ui::screens::options_menu_component::{
    OptionsViewModel, options_view,
};
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
    let label = title_label(text);
    rsx! {
        panel {
            name: TITLE_FRAME,
            width: {PANEL_W},
            height: 36.0,
            strata: FrameStrata::Fullscreen,
            frame_level: 10.0,
            anchor {
                point: AnchorPoint::Top,
                relative_to: MENU_MOUNT,
                relative_point: AnchorPoint::Top,
            }
            {label}
        }
    }
}

fn title_label(text: &str) -> Element {
    rsx! {
        fontstring {
            name: TITLE_LABEL,
            text: {text},
            font_size: 20.0,
            color: "0.96,0.84,0.56,1.0",
            width: {PANEL_W - 20.0},
            height: 30.0,
            justify_h: "CENTER",
            frame_level: 100.0,
            draw_layer: "OVERLAY",
            anchor {
                point: AnchorPoint::Center,
                relative_to: TITLE_FRAME,
                relative_point: AnchorPoint::Center,
            }
        }
    }
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
    rsx! { r#frame { name: {DynName(name.to_string())}, width: BUTTON_W, height: SECTION_GAP } }
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
        GameMenuView::Options => {
            let options = options_view(&model.options);
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::screens::options_menu_component::{
        CameraOptionsView, HudOptionsView, OptionsCategory, SoundOptionsView,
    };
    use ui_toolkit::screen::Screen;

    use crate::ui::anchor::AnchorPoint;
    use crate::ui::registry::FrameRegistry;

    fn model(view: GameMenuView) -> GameMenuViewModel {
        GameMenuViewModel {
            logged_in: true,
            view,
            options: OptionsViewModel {
                category: OptionsCategory::Sound,
                position: [500.0, 180.0],
                sound: SoundOptionsView {
                    muted: false,
                    music_enabled: true,
                    master_volume: 0.8,
                    music_volume: 0.4,
                    ambient_volume: 0.3,
                    footstep_volume: 0.5,
                },
                camera: CameraOptionsView {
                    look_sensitivity: 0.01,
                    invert_y: false,
                    zoom_speed: 8.0,
                    follow_speed: 10.0,
                    min_distance: 2.0,
                    max_distance: 40.0,
                },
                hud: HudOptionsView {
                    show_minimap: true,
                    show_action_bars: true,
                    show_nameplates: true,
                    show_health_bars: true,
                    show_target_marker: true,
                    show_fps_overlay: true,
                },
            },
        }
    }

    #[test]
    fn game_menu_title_is_anchored_to_mount_not_panel_flow() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(model(GameMenuView::MainMenu));
        Screen::new(game_menu_screen).sync(&shared, &mut reg);

        let mount_id = reg.get_by_name(MENU_MOUNT.0).expect("GameMenuMount");
        let title_id = reg.get_by_name(TITLE_FRAME.0).expect("GameMenuTitleFrame");
        let panel_id = reg.get_by_name(MENU_PANEL.0).expect("GameMenuPanel");
        let title = reg.get(title_id).expect("title frame");
        let panel = reg.get(panel_id).expect("panel frame");

        assert_eq!(title.anchors[0].relative_to, Some(mount_id));
        assert_eq!(panel.anchors[0].relative_to, Some(mount_id));
        assert_eq!(panel.anchors[0].point, AnchorPoint::Top);
    }
}
