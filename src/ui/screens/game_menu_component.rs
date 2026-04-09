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
mod tests {
    use std::sync::{Mutex, OnceLock};

    use super::*;
    use crate::input_bindings::{BindingSection, InputAction, InputBinding};
    use crate::ui::screens::options_menu_component::{
        CameraOptionsView, GraphicsOptionsView, HudOptionsView, KeybindingRowView, KeybindingsView,
        OptionsCategory, SoundOptionsView,
    };
    use ui_toolkit::layout::recompute_layouts;
    use ui_toolkit::screen::Screen;
    use ui_toolkit::text_measure::measure_text;
    use ui_toolkit::widgets::font_string::GameFont;

    use crate::ui::anchor::AnchorPoint;
    use crate::ui::frame::{WidgetData, WidgetType};
    use crate::ui::registry::FrameRegistry;
    use ui_toolkit::widgets::font_string::Outline;

    static OPTIONS_REGISTRY_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn model(view: GameMenuView) -> GameMenuViewModel {
        GameMenuViewModel {
            logged_in: true,
            view,
            options: OptionsViewModel {
                category: OptionsCategory::Sound,
                position: [500.0, 180.0],
                graphics: graphics_view(),
                sound: sound_view(),
                camera: camera_view(),
                hud: hud_view(),
                bindings: bindings_view(),
            },
        }
    }

    fn sound_view() -> SoundOptionsView {
        SoundOptionsView {
            muted: false,
            music_enabled: true,
            master_volume: 0.8,
            music_volume: 0.4,
            ambient_volume: 0.3,
            effects_volume: 0.8,
        }
    }

    fn graphics_view() -> GraphicsOptionsView {
        GraphicsOptionsView {
            particle_density: 100.0,
            render_scale: 1.0,
            ui_scale: 1.0,
            colorblind_mode: false,
            bloom_enabled: false,
            bloom_intensity: 0.08,
        }
    }

    fn camera_view() -> CameraOptionsView {
        CameraOptionsView {
            mouse_sensitivity: 0.003,
            look_sensitivity: 0.01,
            invert_y: false,
            zoom_speed: 8.0,
            follow_speed: 10.0,
            min_distance: 2.0,
            max_distance: 40.0,
        }
    }

    fn hud_view() -> HudOptionsView {
        HudOptionsView {
            show_minimap: true,
            show_action_bars: true,
            show_nameplates: true,
            nameplate_distance: 40.0,
            show_health_bars: true,
            show_target_marker: true,
            show_fps_overlay: true,
            chat_font_size: 10.0,
        }
    }

    fn bindings_view() -> KeybindingsView {
        KeybindingsView {
            section: BindingSection::Movement,
            capture_action: None,
            rows: [
                InputAction::MoveForward,
                InputAction::MoveBackward,
                InputAction::StrafeLeft,
                InputAction::StrafeRight,
                InputAction::Jump,
                InputAction::RunToggle,
                InputAction::AutoRun,
            ]
            .into_iter()
            .map(make_binding_row)
            .collect(),
        }
    }

    fn make_binding_row(action: InputAction) -> KeybindingRowView {
        KeybindingRowView {
            action,
            label: action.label().to_string(),
            binding_text: action
                .default_binding()
                .map(InputBinding::display)
                .unwrap_or_else(|| "Unbound".to_string()),
            capturing: false,
            can_clear: action.default_binding().is_some(),
        }
    }

    #[test]
    fn game_menu_builds_all_buttons() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(model(GameMenuView::MainMenu));
        Screen::new(game_menu_screen).sync(&shared, &mut reg);

        for name in [
            "MenuBtnOptions",
            "MenuBtnSupport",
            "MenuBtnAddons",
            "MenuBtnLogout",
            "MenuBtnExit",
            "MenuBtnResume",
        ] {
            assert!(reg.get_by_name(name).is_some(), "{name} missing");
        }
    }

    #[test]
    fn game_menu_button_atlas_names_resolve() {
        use ui_toolkit::atlas::get_region;
        for atlas in [
            BUTTON_ATLAS_UP,
            BUTTON_ATLAS_PRESSED,
            BUTTON_ATLAS_HIGHLIGHT,
            BUTTON_ATLAS_DISABLED,
        ] {
            assert!(get_region(atlas).is_some(), "atlas {atlas} not found");
        }
    }

    #[test]
    fn coord_mount_centered_on_screen() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(model(GameMenuView::MainMenu));
        Screen::new(game_menu_screen).sync(&shared, &mut reg);
        recompute_layouts(&mut reg);

        let mount_id = reg.get_by_name(MENU_MOUNT.0).expect("mount");
        let mount = reg.get(mount_id).expect("data");
        let lr = mount.layout_rect.as_ref().expect("layout_rect");
        let expected_x = (1920.0 - PANEL_W) / 2.0;
        assert!(
            (lr.x - expected_x).abs() < 1.0,
            "x: expected {expected_x}, got {}",
            lr.x
        );
        assert!((lr.width - PANEL_W).abs() < 1.0);
    }

    #[test]
    fn coord_button_dimensions() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(model(GameMenuView::MainMenu));
        Screen::new(game_menu_screen).sync(&shared, &mut reg);
        recompute_layouts(&mut reg);

        let id = reg.get_by_name("MenuBtnOptions").expect("options btn");
        let frame = reg.get(id).expect("data");
        let lr = frame.layout_rect.as_ref().expect("layout_rect");
        assert!((lr.width - BUTTON_W).abs() < 1.0);
        assert!((lr.height - BUTTON_H).abs() < 1.0);
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
    fn options_screen_uses_shared_slider_widget_structure() {
        let reg = options_registry();
        let slider_id = reg
            .get_by_name("Slidermaster_volume")
            .expect("master volume slider");
        let slider = reg.get(slider_id).expect("slider frame");
        let track = reg
            .get_by_name("Slidermaster_volumeTrack")
            .and_then(|id| reg.get(id))
            .expect("master volume track");
        let handle = reg
            .get_by_name("Slidermaster_volumeHandle")
            .and_then(|id| reg.get(id))
            .expect("master volume handle");

        assert_eq!(slider.widget_type, WidgetType::Slider);
        assert!(matches!(slider.widget_data, Some(WidgetData::Slider(_))));
        assert_eq!(track.parent_id, Some(slider_id));
        assert_eq!(track.widget_type, WidgetType::Frame);
        assert_eq!(
            handle.parent_id,
            Some(reg.get_by_name("Slidermaster_volumeTrack").unwrap())
        );
        assert_eq!(handle.widget_type, WidgetType::Texture);
    }

    #[test]
    fn accessibility_screen_includes_ui_scale_slider() {
        let reg = options_registry_for_category(OptionsCategory::Accessibility);

        assert!(reg.get_by_name("SliderRowui_scale").is_some());
        assert!(reg.get_by_name("Sliderui_scale").is_some());
        assert!(reg.get_by_name("Sliderui_scaleHandle").is_some());
    }

    #[test]
    fn accessibility_screen_includes_colorblind_toggle() {
        let reg = options_registry_for_category(OptionsCategory::Accessibility);

        assert!(reg.get_by_name("ToggleRowcolorblind_mode").is_some());
        assert!(reg.get_by_name("ToggleSwitchcolorblind_mode").is_some());
    }

    #[test]
    fn camera_screen_includes_mouse_sensitivity_slider() {
        let reg = options_registry_for_category(OptionsCategory::Camera);

        assert!(reg.get_by_name("SliderRowmouse_sensitivity").is_some());
        assert!(reg.get_by_name("Slidermouse_sensitivity").is_some());
        assert!(reg.get_by_name("Slidermouse_sensitivityHandle").is_some());
    }

    #[test]
    fn interface_screen_includes_chat_font_size_slider() {
        let reg = options_registry_for_category(OptionsCategory::Interface);

        assert!(reg.get_by_name("SliderRowchat_font_size").is_some());
        assert!(reg.get_by_name("Sliderchat_font_size").is_some());
        assert!(reg.get_by_name("Sliderchat_font_sizeHandle").is_some());
    }

    #[test]
    fn hud_screen_includes_nameplate_distance_slider() {
        let reg = options_registry_for_category(OptionsCategory::Hud);

        assert!(reg.get_by_name("SliderRownameplate_distance").is_some());
        assert!(reg.get_by_name("Slidernameplate_distance").is_some());
        assert!(reg.get_by_name("Slidernameplate_distanceHandle").is_some());
    }

    #[test]
    fn slider_thumb_rect_moves_when_value_changes() {
        let low = options_registry_with_master_volume(0.2);
        let high = options_registry_with_master_volume(0.8);

        let low_thumb = rect_by_name(&low, "Slidermaster_volumeHandle");
        let high_thumb = rect_by_name(&high, "Slidermaster_volumeHandle");

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
        let low_thumb = rect_by_name(&reg, "Slidermaster_volumeHandle");

        let mut high = model(GameMenuView::Options);
        high.options.position = [0.0, 0.0];
        high.options.sound.master_volume = 0.8;
        shared.insert(high);
        screen.sync(&shared, &mut reg);
        recompute_layouts(&mut reg);
        let high_thumb = rect_by_name(&reg, "Slidermaster_volumeHandle");

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
            muted_active.x > unmuted_active.x,
            "mute active segment should move right when muted: unmuted={}, muted={}",
            unmuted_active.x,
            muted_active.x
        );
        assert!(unmuted.get_by_name("ToggleSwitchmutedLeftHit").is_none());
        assert!(unmuted.get_by_name("ToggleSwitchmutedRightHit").is_some());
        assert!(muted.get_by_name("ToggleSwitchmutedLeftHit").is_some());
        assert!(muted.get_by_name("ToggleSwitchmutedRightHit").is_none());
    }

    #[test]
    fn options_footer_only_shows_defaults_and_done_buttons() {
        let reg = options_registry();

        assert!(reg.get_by_name("OptionsDefaultsButton").is_some());
        assert!(reg.get_by_name("OptionsDoneButton").is_some());
        assert!(reg.get_by_name("OptionsBackButton").is_none());
        assert!(reg.get_by_name("OptionsApplyButton").is_none());
        assert!(reg.get_by_name("OptionsCancelButton").is_none());
        assert!(reg.get_by_name("OptionsOkayButton").is_none());
    }

    #[test]
    fn keybindings_tab_lists_movement_bindings() {
        let reg = options_registry_for_category(OptionsCategory::Keybindings);

        assert!(reg.get_by_name("KeybindingSectionmovement").is_some());
        assert!(reg.get_by_name("KeybindingRowmove_forward").is_some());
        assert!(reg.get_by_name("KeybindingRowmove_backward").is_some());
        assert!(reg.get_by_name("KeybindingRowstrafe_left").is_some());
        assert!(reg.get_by_name("KeybindingRowstrafe_right").is_some());
        assert!(reg.get_by_name("KeybindingRowjump").is_some());
        assert!(reg.get_by_name("KeybindingRowrun_toggle").is_some());
        assert!(reg.get_by_name("KeybindingRowauto_run").is_some());
    }

    #[test]
    fn keybinding_section_tabs_layout_left_to_right_without_overlap() {
        let reg = options_registry_for_category(OptionsCategory::Keybindings);
        let tabs = [
            ("KeybindingSectionmovement", "Movement"),
            ("KeybindingSectioncamera", "Camera"),
            ("KeybindingSectiontargeting", "Targeting"),
            ("KeybindingSectionaction_bar", "Action Bar"),
            ("KeybindingSectionaudio", "Audio"),
        ];
        let rects: Vec<_> = tabs
            .iter()
            .map(|(name, _)| rect_by_name(&reg, name))
            .collect();

        for (rect, (_, label)) in rects.iter().zip(tabs.iter()) {
            let expected_width = measure_text(label, GameFont::FrizQuadrata, 10.0)
                .unwrap()
                .0
                .ceil()
                + 20.0;
            assert_eq!(
                rect.width, expected_width,
                "tab width should match WoW width formula"
            );
            assert_eq!(rect.height, 32.0, "tab height should match WoW tab height");
        }

        for pair in rects.windows(2) {
            let left = &pair[0];
            let right = &pair[1];
            assert!(
                (right.x - (left.x + left.width) - 1.0).abs() < f32::EPSILON,
                "section tabs should keep WoW 1px spacing: left={left:?} right={right:?}"
            );
        }
    }

    #[test]
    fn keybinding_active_tab_label_uses_wow_font_treatment() {
        let reg = options_registry_for_category(OptionsCategory::Keybindings);
        let label = reg
            .get(reg.get_by_name("KeybindingSectionmovementLabel").unwrap())
            .unwrap();
        let Some(WidgetData::FontString(font)) = label.widget_data.as_ref() else {
            panic!("expected active tab label font string");
        };

        assert_eq!(font.font, GameFont::FrizQuadrata);
        assert_eq!(font.font_size, 10.0);
        assert_eq!(font.color, [0.96, 0.84, 0.56, 1.0]);
        assert_eq!(font.shadow_color, Some([0.0, 0.0, 0.0, 1.0]));
        assert_eq!(font.shadow_offset, [1.0, -1.0]);
        assert_eq!(font.outline, Outline::None);
    }

    #[test]
    fn keybinding_capturing_row_shows_listening_prompt() {
        let mut view = model(GameMenuView::Options);
        view.options.category = OptionsCategory::Keybindings;
        view.options.bindings.capture_action = Some(InputAction::Jump);
        for row in &mut view.options.bindings.rows {
            row.capturing = row.action == InputAction::Jump;
        }

        let reg = options_registry_with_view(view);
        let value = reg
            .get(reg.get_by_name("KeybindingValuejump").unwrap())
            .unwrap();
        let Some(WidgetData::FontString(font)) = value.widget_data.as_ref() else {
            panic!("expected jump keybinding value font string");
        };

        assert_eq!(font.text, "Press a key or mouse button...");
    }

    #[test]
    fn keybinding_unbound_row_omits_clear_action() {
        let mut view = model(GameMenuView::Options);
        view.options.category = OptionsCategory::Keybindings;
        let row = view
            .options
            .bindings
            .rows
            .iter_mut()
            .find(|row| row.action == InputAction::MoveForward)
            .expect("move forward row");
        row.binding_text = "Unbound".to_string();
        row.can_clear = false;

        let reg = options_registry_with_view(view);
        let clear = reg
            .get(reg.get_by_name("KeybindingClearmove_forward").unwrap())
            .unwrap();
        assert!(clear.onclick.is_none());
    }

    fn options_registry() -> FrameRegistry {
        options_registry_with_master_volume(0.8)
    }

    fn options_registry_for_category(category: OptionsCategory) -> FrameRegistry {
        let mut view = model(GameMenuView::Options);
        view.options.position = [0.0, 0.0];
        view.options.category = category;
        options_registry_with_view(view)
    }

    fn options_registry_with_view(view: GameMenuViewModel) -> FrameRegistry {
        let _guard = options_registry_test_lock().lock().unwrap();
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(view);
        Screen::new(game_menu_screen).sync(&shared, &mut reg);
        recompute_layouts(&mut reg);
        reg
    }

    fn options_registry_with_master_volume(master_volume: f32) -> FrameRegistry {
        let mut view = model(GameMenuView::Options);
        view.options.position = [0.0, 0.0];
        view.options.sound.master_volume = master_volume;
        options_registry_with_view(view)
    }

    fn options_registry_with_muted(muted: bool) -> FrameRegistry {
        let mut view = model(GameMenuView::Options);
        view.options.position = [0.0, 0.0];
        view.options.sound.muted = muted;
        options_registry_with_view(view)
    }

    fn rect_by_name(reg: &FrameRegistry, name: &str) -> crate::ui::layout::LayoutRect {
        reg.get(reg.get_by_name(name).expect(name))
            .and_then(|frame| frame.layout_rect.clone())
            .expect(name)
    }

    fn options_registry_test_lock() -> &'static Mutex<()> {
        OPTIONS_REGISTRY_TEST_LOCK.get_or_init(|| Mutex::new(()))
    }
}
