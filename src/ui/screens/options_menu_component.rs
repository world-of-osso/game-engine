use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::{AnchorPoint, FrameName};
use crate::ui::strata::FrameStrata;

struct DynName(String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub const OPTIONS_ROOT: FrameName = FrameName("OptionsRoot");
pub const OPTIONS_DRAG_HANDLE: FrameName = FrameName("OptionsDragHandle");
const OPTIONS_TITLE: FrameName = FrameName("OptionsTitle");
const OPTIONS_TAB_PANEL: FrameName = FrameName("OptionsTabPanel");
const OPTIONS_CONTENT_PANEL: FrameName = FrameName("OptionsContentPanel");
const OPTIONS_FOOTER: FrameName = FrameName("OptionsFooter");

const BUTTON_ATLAS_UP: &str = "defaultbutton-nineslice-up";
const BUTTON_ATLAS_PRESSED: &str = "defaultbutton-nineslice-pressed";
const BUTTON_ATLAS_HIGHLIGHT: &str = "defaultbutton-nineslice-highlight";
const BUTTON_ATLAS_DISABLED: &str = "defaultbutton-nineslice-disabled";

const OPTIONS_W: f32 = 860.0;
const OPTIONS_H: f32 = 580.0;
const OPTIONS_HEADER_H: f32 = 54.0;
const OPTIONS_TAB_W: f32 = 170.0;
const OPTIONS_CONTENT_W: f32 = 610.0;
const OPTIONS_CONTENT_H: f32 = 404.0;
const OPTIONS_TRACK_W: f32 = 240.0;
const OPTIONS_TRACK_H: f32 = 10.0;
const OPTIONS_THUMB_W: f32 = 14.0;

pub const ACTION_OPTIONS_BACK: &str = "options_back";
pub const ACTION_OPTIONS_APPLY: &str = "options_apply";
pub const ACTION_OPTIONS_CANCEL: &str = "options_cancel";
pub const ACTION_OPTIONS_OKAY: &str = "options_okay";
pub const ACTION_OPTIONS_DEFAULTS: &str = "options_defaults";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionsCategory {
    Graphics,
    Sound,
    Camera,
    Interface,
    Hud,
    Controls,
    Accessibility,
    Keybindings,
    Macros,
    SocialAddons,
    Advanced,
    Support,
}

impl OptionsCategory {
    pub const ALL: [Self; 12] = [
        Self::Graphics,
        Self::Sound,
        Self::Camera,
        Self::Interface,
        Self::Hud,
        Self::Controls,
        Self::Accessibility,
        Self::Keybindings,
        Self::Macros,
        Self::SocialAddons,
        Self::Advanced,
        Self::Support,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::Graphics => "graphics",
            Self::Sound => "sound",
            Self::Camera => "camera",
            Self::Interface => "interface",
            Self::Hud => "hud",
            Self::Controls => "controls",
            Self::Accessibility => "accessibility",
            Self::Keybindings => "keybindings",
            Self::Macros => "macros",
            Self::SocialAddons => "socialaddons",
            Self::Advanced => "advanced",
            Self::Support => "support",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Graphics => "Graphics",
            Self::Sound => "Sound",
            Self::Camera => "Camera",
            Self::Interface => "Interface",
            Self::Hud => "HUD",
            Self::Controls => "Controls",
            Self::Accessibility => "Accessibility",
            Self::Keybindings => "Keybindings",
            Self::Macros => "Macros",
            Self::SocialAddons => "Social / AddOns",
            Self::Advanced => "Advanced / Debug",
            Self::Support => "Support / About",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SoundOptionsView {
    pub muted: bool,
    pub music_enabled: bool,
    pub master_volume: f32,
    pub music_volume: f32,
    pub ambient_volume: f32,
    pub footstep_volume: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CameraOptionsView {
    pub look_sensitivity: f32,
    pub invert_y: bool,
    pub zoom_speed: f32,
    pub follow_speed: f32,
    pub min_distance: f32,
    pub max_distance: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudOptionsView {
    pub show_minimap: bool,
    pub show_action_bars: bool,
    pub show_nameplates: bool,
    pub show_health_bars: bool,
    pub show_target_marker: bool,
    pub show_fps_overlay: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionsViewModel {
    pub category: OptionsCategory,
    pub position: [f32; 2],
    pub sound: SoundOptionsView,
    pub camera: CameraOptionsView,
    pub hud: HudOptionsView,
}

pub fn cat_action(category: OptionsCategory) -> String {
    format!("options_category:{}", category.key())
}

pub fn toggle_action(key: &str) -> String {
    format!("options_toggle:{key}")
}

pub fn slider_action(key: &str) -> String {
    format!("options_slider:{key}")
}

pub fn step_action(key: &str, delta: i32) -> String {
    format!("options_step:{key}:{delta}")
}

pub fn options_view(model: &OptionsViewModel) -> Element {
    let x = model.position[0].to_string();
    let y = model.position[1].to_string();
    rsx! {
        panel {
            name: OPTIONS_ROOT,
            width: {OPTIONS_W},
            height: {OPTIONS_H},
            strata: FrameStrata::Dialog,
            frame_level: 12.0,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {y},
            }
            {header()}
            {build_tabs(model)}
            {build_content(model)}
            {build_footer()}
        }
    }
}

fn header() -> Element {
    rsx! {
        r#frame {
            name: OPTIONS_DRAG_HANDLE,
            width: {OPTIONS_W},
            height: {OPTIONS_HEADER_H},
            background_color: "0.14,0.10,0.05,0.86",
            mouse_enabled: true,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
            }
        }
        fontstring {
            name: OPTIONS_TITLE,
            width: 280.0,
            height: 24.0,
            text: "Options",
            font_size: 22.0,
            color: "0.96,0.84,0.56,1.0",
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::Left,
                relative_to: OPTIONS_DRAG_HANDLE,
                relative_point: AnchorPoint::Left,
                x: "18",
            }
        }
    }
}

fn build_tabs(model: &OptionsViewModel) -> Element {
    let buttons: Element = OptionsCategory::ALL
        .iter()
        .enumerate()
        .flat_map(|(index, category)| tab_button(*category, model.category == *category, index))
        .collect();
    rsx! {
        panel {
            name: OPTIONS_TAB_PANEL,
            width: {OPTIONS_TAB_W},
            height: {OPTIONS_CONTENT_H},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_to: OPTIONS_DRAG_HANDLE,
                relative_point: AnchorPoint::BottomLeft,
                x: "18",
                y: "-18",
            }
            {buttons}
        }
    }
}

fn tab_button(category: OptionsCategory, selected: bool, index: usize) -> Element {
    let action = cat_action(category);
    let name = DynName(format!("OptionsTab{}", category.key()));
    let label = if selected {
        format!("{} *", category.title())
    } else {
        category.title().to_string()
    };
    let y = (index as f32 * 34.0).to_string();
    rsx! {
        button {
            name: {&name},
            width: 150.0,
            height: 28.0,
            text: {&label},
            font_size: 13.0,
            onclick: {&action},
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_to: OPTIONS_TAB_PANEL,
                relative_point: AnchorPoint::TopLeft,
                x: "10",
                y: {y},
            }
        }
    }
}

fn build_content(model: &OptionsViewModel) -> Element {
    rsx! {
        panel {
            name: OPTIONS_CONTENT_PANEL,
            width: {OPTIONS_CONTENT_W},
            height: {OPTIONS_CONTENT_H},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_to: OPTIONS_DRAG_HANDLE,
                relative_point: AnchorPoint::BottomLeft,
                x: "220",
                y: "-18",
            }
            {content_header(model.category)}
            {category_body(model)}
        }
    }
}

fn content_header(category: OptionsCategory) -> Element {
    rsx! {
        fontstring {
            name: "OptionsSectionTitle",
            width: {OPTIONS_CONTENT_W - 30.0},
            height: 24.0,
            text: {category.title()},
            font_size: 22.0,
            color: "0.96,0.84,0.56,1.0",
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_to: OPTIONS_CONTENT_PANEL,
                relative_point: AnchorPoint::TopLeft,
                x: "15",
                y: "18",
            }
        }
    }
}

fn category_body(model: &OptionsViewModel) -> Element {
    match model.category {
        OptionsCategory::Sound => sound_body(&model.sound),
        OptionsCategory::Camera => camera_body(&model.camera),
        OptionsCategory::Interface => interface_body(&model.hud),
        OptionsCategory::Hud => hud_body(&model.hud),
        _ => placeholder_body(model.category),
    }
}

fn sound_body(sound: &SoundOptionsView) -> Element {
    let rows = [
        toggle_row("muted", "Mute All Sound", sound.muted, 0),
        toggle_row("music_enabled", "Enable Music", sound.music_enabled, 1),
        slider_row("master_volume", "Master Volume", sound.master_volume, 0.0, 1.0, 2),
        slider_row("music_volume", "Music Volume", sound.music_volume, 0.0, 1.0, 3),
        slider_row("ambient_volume", "Ambient Volume", sound.ambient_volume, 0.0, 1.0, 4),
        slider_row("footstep_volume", "Footstep Volume", sound.footstep_volume, 0.0, 1.0, 5),
    ];
    rows.into_iter().flatten().collect()
}

fn camera_body(camera: &CameraOptionsView) -> Element {
    let settings = camera_slider_settings(camera);
    settings.into_iter().flatten().collect()
}

fn camera_slider_settings(camera: &CameraOptionsView) -> [Element; 6] {
    [
        toggle_row("invert_y", "Invert Vertical Look", camera.invert_y, 0),
        slider_row("look_sensitivity", "Look Sensitivity", camera.look_sensitivity, 0.002, 0.03, 1),
        slider_row("zoom_speed", "Zoom Speed", camera.zoom_speed, 2.0, 20.0, 2),
        slider_row("follow_speed", "Follow Speed", camera.follow_speed, 2.0, 20.0, 3),
        slider_row("min_distance", "Min Camera Distance", camera.min_distance, 1.0, 10.0, 4),
        slider_row("max_distance", "Max Camera Distance", camera.max_distance, 10.0, 60.0, 5),
    ]
}

fn interface_body(hud: &HudOptionsView) -> Element {
    [toggle_row("show_fps_overlay", "Show FPS Overlay", hud.show_fps_overlay, 0)]
        .into_iter()
        .flatten()
        .collect()
}

fn hud_body(hud: &HudOptionsView) -> Element {
    let rows = [
        toggle_row("show_minimap", "Show Minimap", hud.show_minimap, 0),
        toggle_row("show_action_bars", "Show Action Bars", hud.show_action_bars, 1),
        toggle_row("show_nameplates", "Show Nameplates", hud.show_nameplates, 2),
        toggle_row("show_health_bars", "Show Health Bars", hud.show_health_bars, 3),
        toggle_row("show_target_marker", "Show Target Marker", hud.show_target_marker, 4),
    ];
    rows.into_iter().flatten().collect()
}

fn placeholder_body(category: OptionsCategory) -> Element {
    let title = format!("{} is planned in this shell.", category.title());
    rsx! {
        r#frame {
            name: "OptionsPlaceholderPanel",
            width: {OPTIONS_CONTENT_W - 30.0},
            height: {OPTIONS_CONTENT_H - 40.0},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_to: OPTIONS_CONTENT_PANEL,
                relative_point: AnchorPoint::TopLeft,
                x: "15",
                y: "54",
            }
            {placeholder_title(&title)}
            {placeholder_detail(placeholder_text(category))}
        }
    }
}

fn placeholder_title(text: &str) -> Element {
    rsx! {
        fontstring {
            name: "OptionsPlaceholderTitle",
            width: {OPTIONS_CONTENT_W - 30.0},
            height: 22.0,
            text: {text},
            font_size: 18.0,
            color: "0.95,0.90,0.74,1.0",
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
            }
        }
    }
}

fn placeholder_detail(text: &str) -> Element {
    rsx! {
        fontstring {
            name: "OptionsPlaceholderDetail",
            width: {OPTIONS_CONTENT_W - 40.0},
            height: 120.0,
            text: {text},
            font_size: 15.0,
            color: "0.72,0.72,0.72,1.0",
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_to: FrameName("OptionsPlaceholderTitle"),
                relative_point: AnchorPoint::BottomLeft,
                y: "-14",
            }
        }
    }
}

fn placeholder_text(category: OptionsCategory) -> &'static str {
    match category {
        OptionsCategory::Graphics => "Display mode, render scale, shadows, textures, and environment quality will land after the drag/input foundation is stable.",
        OptionsCategory::Controls => "Movement and mouse-control details beyond camera sensitivity will be wired here once the dedicated input-settings pass is ready.",
        OptionsCategory::Accessibility => "Color, subtitle, readability, and motion-friendly controls will be added as the surrounding UI stack gains those engine hooks.",
        OptionsCategory::Keybindings => "This panel reserves the Blizzard-style location for a later keybinding editor instead of hiding the category entirely.",
        OptionsCategory::Macros => "Macros are represented here as a future shell entry so the menu structure matches the intended Blizzard-style breadth.",
        OptionsCategory::SocialAddons => "Social and AddOns remain placeholders until addon execution and social panels are a real milestone in the client.",
        OptionsCategory::Advanced => "Advanced and debug controls are partially live elsewhere in the client and can expand here as more toggles are promoted into resources.",
        OptionsCategory::Support => "Support, help, and account/about affordances stay grouped here for parity with the Blizzard menu structure.",
        _ => "This category is intentionally present as part of the broad Blizzard-style shell.",
    }
}

fn toggle_row(key: &str, label: &str, enabled: bool, row: usize) -> Element {
    let y = row_y(row, 44.0);
    let state = if enabled { "Enabled" } else { "Disabled" };
    let color = if enabled {
        "0.85,0.95,0.74,1.0"
    } else {
        "0.92,0.60,0.54,1.0"
    };
    let action = toggle_action(key);
    let button_name = format!("ToggleButton{key}");
    rsx! {
        r#frame {
            name: {DynName(format!("ToggleRow{key}"))},
            width: {OPTIONS_CONTENT_W - 30.0},
            height: 32.0,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_to: OPTIONS_CONTENT_PANEL,
                relative_point: AnchorPoint::TopLeft,
                x: "15",
                y: {y},
            }
            {row_label(&format!("ToggleLabel{key}"), label)}
            {toggle_state_text(key, state, color)}
            {small_button(&button_name, state, &action, 110.0, "-4")}
        }
    }
}

fn toggle_state_text(key: &str, text: &str, color: &str) -> Element {
    let button_name = DynName(format!("ToggleButton{key}"));
    rsx! {
        fontstring {
            name: {DynName(format!("ToggleState{key}"))},
            width: 120.0,
            height: 18.0,
            text: {text},
            font_size: 13.0,
            color: {color},
            justify_h: "RIGHT",
            anchor {
                point: AnchorPoint::Right,
                relative_to: {&button_name},
                relative_point: AnchorPoint::Left,
                x: "-8",
            }
        }
    }
}

fn row_label(name: &str, text: &str) -> Element {
    rsx! {
        fontstring {
            name: {DynName(name.to_string())},
            width: 320.0,
            height: 20.0,
            text: {text},
            font_size: 16.0,
            color: "0.95,0.90,0.74,1.0",
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::Left,
                relative_point: AnchorPoint::Left,
            }
        }
    }
}

fn slider_row(key: &str, label: &str, value: f32, min: f32, max: f32, row: usize) -> Element {
    let y = row_y(row, 56.0);
    let pct = normalize(value, min, max).clamp(0.0, 1.0);
    let action = slider_action(key);
    let minus = step_action(key, -1);
    let plus = step_action(key, 1);
    rsx! {
        r#frame {
            name: {DynName(format!("SliderRow{key}"))},
            width: {OPTIONS_CONTENT_W - 30.0},
            height: 44.0,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_to: OPTIONS_CONTENT_PANEL,
                relative_point: AnchorPoint::TopLeft,
                x: "15",
                y: {y},
            }
            {row_label(&format!("SliderLabel{key}"), label)}
            {slider_track_frames(key, pct)}
            {slider_drag_frame(key, &action)}
            {small_button(&format!("SliderMinus{key}"), "-", &minus, 30.0, "520")}
            {small_button(&format!("SliderPlus{key}"), "+", &plus, 30.0, "554")}
            {slider_value_text(key, &slider_display(min, max, pct))}
        }
    }
}

fn slider_track_frames(key: &str, pct: f32) -> Element {
    let fill_w = (OPTIONS_TRACK_W * pct).to_string();
    let thumb_x = ((OPTIONS_TRACK_W - OPTIONS_THUMB_W) * pct).to_string();
    let track_name = DynName(format!("SliderTrack{key}"));
    rsx! {
        r#frame {
            name: {&track_name},
            width: {OPTIONS_TRACK_W},
            height: {OPTIONS_TRACK_H},
            background_color: "0.10,0.09,0.08,1.0",
            anchor {
                point: AnchorPoint::Left,
                relative_point: AnchorPoint::Left,
                x: "230",
                y: "-2",
            }
        }
        r#frame {
            name: {DynName(format!("SliderFill{key}"))},
            width: {fill_w},
            height: {OPTIONS_TRACK_H},
            background_color: "0.85,0.66,0.18,1.0",
            anchor {
                point: AnchorPoint::Left,
                relative_to: {&track_name},
                relative_point: AnchorPoint::Left,
            }
        }
        {slider_thumb_frame(key, &track_name, &thumb_x)}
    }
}

fn slider_thumb_frame(key: &str, track_name: &DynName, thumb_x: &str) -> Element {
    rsx! {
        r#frame {
            name: {DynName(format!("SliderThumb{key}"))},
            width: {OPTIONS_THUMB_W},
            height: 22.0,
            background_color: "0.92,0.86,0.74,1.0",
            anchor {
                point: AnchorPoint::Left,
                relative_to: {track_name},
                relative_point: AnchorPoint::Left,
                x: {thumb_x},
                y: "-6",
            }
        }
    }
}

fn slider_drag_frame(key: &str, action: &str) -> Element {
    let track_name = DynName(format!("SliderTrack{key}"));
    rsx! {
        r#frame {
            name: {DynName(format!("SliderDrag{key}"))},
            width: {OPTIONS_TRACK_W},
            height: 28.0,
            mouse_enabled: true,
            onclick: {action},
            anchor {
                point: AnchorPoint::Left,
                relative_to: {&track_name},
                relative_point: AnchorPoint::Left,
                y: "-9",
            }
        }
    }
}

fn slider_value_text(key: &str, text: &str) -> Element {
    rsx! {
        fontstring {
            name: {DynName(format!("SliderValue{key}"))},
            width: 70.0,
            height: 20.0,
            text: {text},
            font_size: 15.0,
            color: "0.95,0.90,0.74,1.0",
            justify_h: "RIGHT",
            anchor {
                point: AnchorPoint::Right,
                relative_point: AnchorPoint::Right,
                x: "-72",
            }
        }
    }
}

fn build_footer() -> Element {
    let buttons = [
        ("OptionsBackButton", "Back", ACTION_OPTIONS_BACK, 80.0, "212"),
        ("OptionsDefaultsButton", "Defaults", ACTION_OPTIONS_DEFAULTS, 110.0, "308"),
        ("OptionsApplyButton", "Apply", ACTION_OPTIONS_APPLY, 90.0, "436"),
        ("OptionsCancelButton", "Cancel", ACTION_OPTIONS_CANCEL, 90.0, "538"),
        ("OptionsOkayButton", "Okay", ACTION_OPTIONS_OKAY, 90.0, "640"),
    ];
    let footer_buttons: Element = buttons
        .into_iter()
        .flat_map(|(name, text, action, width, x)| small_button(name, text, action, width, x))
        .collect();
    rsx! {
        r#frame {
            name: OPTIONS_FOOTER,
            width: {OPTIONS_W - 36.0},
            height: 42.0,
            anchor {
                point: AnchorPoint::Bottom,
                relative_point: AnchorPoint::Bottom,
                y: "-14",
            }
            {footer_buttons}
        }
    }
}

fn small_button(name: &str, text: &str, action: &str, width: f32, x: &str) -> Element {
    let n = DynName(name.to_string());
    let text = text.to_string();
    let action = action.to_string();
    rsx! {
        button {
            name: {&n},
            width: {width},
            height: 30.0,
            text: {&text},
            font_size: 14.0,
            onclick: {&action},
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            anchor {
                point: AnchorPoint::Left,
                relative_point: AnchorPoint::Left,
                x: {x},
            }
        }
    }
}

fn row_y(row: usize, spacing: f32) -> String {
    (60.0 + row as f32 * spacing).to_string()
}

fn normalize(value: f32, min: f32, max: f32) -> f32 {
    if (max - min).abs() < f32::EPSILON {
        0.0
    } else {
        (value - min) / (max - min)
    }
}

fn slider_display(min: f32, max: f32, pct: f32) -> String {
    let value = min + (max - min) * pct;
    if max <= 1.0 {
        format!("{value:.2}")
    } else {
        format!("{value:.1}")
    }
}
