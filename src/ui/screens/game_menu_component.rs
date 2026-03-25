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
    use ui_toolkit::layout::recompute_layouts;
    use ui_toolkit::screen::Screen;

    use crate::ui::anchor::AnchorPoint;
    use crate::ui::frame::{WidgetData, WidgetType};
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

    #[test]
    fn options_panel_root_is_screen_centered_with_zero_offset() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        let mut view = model(GameMenuView::Options);
        view.options.position = [0.0, 0.0];
        shared.insert(view);
        Screen::new(game_menu_screen).sync(&shared, &mut reg);
        recompute_layouts(&mut reg);

        let game_menu_root = reg.get_by_name(GAME_MENU_ROOT.0).expect("GameMenuRoot");
        let root_id = reg.get_by_name("OptionsRoot").expect("OptionsRoot");
        let root = reg.get(root_id).expect("options root");
        let anchor = &root.anchors[0];
        let rect = root.layout_rect.as_ref().expect("options root rect");

        assert_eq!(anchor.point, AnchorPoint::Center);
        assert_eq!(anchor.relative_to, Some(game_menu_root));
        assert_eq!(anchor.relative_point, AnchorPoint::Center);
        assert_eq!(rect.x, (1920.0 - rect.width) * 0.5);
        assert_eq!(rect.y, (1080.0 - rect.height) * 0.5);
    }

    #[test]
    fn options_screen_layout_places_sound_rows_inside_content_panel() {
        let reg = options_registry();
        let header = rect_by_name(&reg, "OptionsDragHandle");
        let tabs = rect_by_name(&reg, "OptionsTabPanel");
        let content = rect_by_name(&reg, "OptionsContentPanel");
        let row = rect_by_name(&reg, "SliderRowmaster_volume");

        assert_eq!(header.y + header.height + 18.0, content.y);
        assert_eq!(tabs.y, content.y);
        assert!(row.y >= content.y);
        assert!(row.y + row.height <= content.y + content.height);
    }

    #[test]
    fn selected_options_tab_uses_list_style_accent_and_plain_label() {
        let reg = options_registry();
        let tab_id = reg.get_by_name("OptionsTabsound").expect("OptionsTabsound");
        let tab = reg.get(tab_id).expect("sound tab");
        let label_id = reg
            .get_by_name("OptionsTabsoundLabel")
            .expect("OptionsTabsoundLabel");
        let label = reg.get(label_id).expect("sound tab label");

        assert!(reg.get_by_name("OptionsTabsoundAccent").is_some());
        assert!(reg.get_by_name("OptionsTabgraphicsAccent").is_none());
        let border = tab.border.as_ref().expect("selected tab border");
        assert_eq!(border.width, 1.0);
        assert_eq!(border.color, [0.42, 0.33, 0.12, 0.65]);

        let Some(WidgetData::FontString(font)) = label.widget_data.as_ref() else {
            panic!("expected selected tab label font string");
        };
        assert_eq!(font.text, "Sound");
        assert_eq!(font.color, [0.96, 0.84, 0.56, 1.0]);
    }

    #[test]
    fn options_screen_uses_shared_slider_and_statusbar_widgets() {
        let reg = options_registry();
        let slider_id = reg
            .get_by_name("Slidermaster_volume")
            .expect("master volume slider");
        let fill_id = reg
            .get_by_name("Slidermaster_volumeFill")
            .expect("master volume fill");

        let slider = reg.get(slider_id).expect("slider frame");
        let fill = reg.get(fill_id).expect("fill frame");

        assert_eq!(slider.widget_type, WidgetType::Slider);
        assert!(matches!(slider.widget_data, Some(WidgetData::Slider(_))));
        assert_eq!(fill.widget_type, WidgetType::StatusBar);
        assert!(matches!(fill.widget_data, Some(WidgetData::StatusBar(_))));
    }

    #[test]
    fn slider_thumb_rect_moves_when_value_changes() {
        let low = options_registry_with_master_volume(0.2);
        let high = options_registry_with_master_volume(0.8);

        let low_thumb = rect_by_name(&low, "Slidermaster_volumeThumbFrame");
        let high_thumb = rect_by_name(&high, "Slidermaster_volumeThumbFrame");

        assert!(
            high_thumb.x > low_thumb.x,
            "thumb should move right as value increases: low={}, high={}",
            low_thumb.x,
            high_thumb.x
        );
    }

    #[test]
    fn slider_thumb_rect_moves_when_existing_screen_rebuilds() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        let mut screen = Screen::new(game_menu_screen);

        let mut low = model(GameMenuView::Options);
        low.options.position = [0.0, 0.0];
        low.options.sound.master_volume = 0.2;
        shared.insert(low);
        screen.sync(&shared, &mut reg);
        recompute_layouts(&mut reg);
        let low_thumb = rect_by_name(&reg, "Slidermaster_volumeThumbFrame");

        let mut high = model(GameMenuView::Options);
        high.options.position = [0.0, 0.0];
        high.options.sound.master_volume = 0.8;
        shared.insert(high);
        screen.sync(&shared, &mut reg);
        recompute_layouts(&mut reg);
        let high_thumb = rect_by_name(&reg, "Slidermaster_volumeThumbFrame");

        assert!(
            high_thumb.x > low_thumb.x,
            "thumb should move right after rebuild: low={}, high={}",
            low_thumb.x,
            high_thumb.x
        );
    }

    #[test]
    fn sound_toggle_switch_moves_active_segment_with_muted_state() {
        let unmuted = options_registry_with_muted(false);
        let muted = options_registry_with_muted(true);

        let unmuted_active = rect_by_name(&unmuted, "ToggleSwitchmutedActive");
        let muted_active = rect_by_name(&muted, "ToggleSwitchmutedActive");

        assert!(
            muted_active.x < unmuted_active.x,
            "mute active segment should move left when muted: unmuted={}, muted={}",
            unmuted_active.x,
            muted_active.x
        );
        assert!(unmuted.get_by_name("ToggleSwitchmutedLeftHit").is_some());
        assert!(unmuted.get_by_name("ToggleSwitchmutedRightHit").is_none());
        assert!(muted.get_by_name("ToggleSwitchmutedLeftHit").is_none());
        assert!(muted.get_by_name("ToggleSwitchmutedRightHit").is_some());
    }

    fn options_registry() -> FrameRegistry {
        options_registry_with_master_volume(0.8)
    }

    fn options_registry_with_master_volume(master_volume: f32) -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        let mut view = model(GameMenuView::Options);
        view.options.position = [0.0, 0.0];
        view.options.sound.master_volume = master_volume;
        shared.insert(view);
        Screen::new(game_menu_screen).sync(&shared, &mut reg);
        recompute_layouts(&mut reg);
        reg
    }

    fn options_registry_with_muted(muted: bool) -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        let mut view = model(GameMenuView::Options);
        view.options.position = [0.0, 0.0];
        view.options.sound.muted = muted;
        shared.insert(view);
        Screen::new(game_menu_screen).sync(&shared, &mut reg);
        recompute_layouts(&mut reg);
        reg
    }

    fn rect_by_name(reg: &FrameRegistry, name: &str) -> crate::ui::layout::LayoutRect {
        reg.get(reg.get_by_name(name).expect(name))
            .and_then(|frame| frame.layout_rect.clone())
            .expect(name)
    }
}
