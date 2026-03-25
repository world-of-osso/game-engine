use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::slider::{SliderWidget, slider_widget};
use ui_toolkit::widgets::toggle::{ToggleWidget, toggle_widget};

use crate::ui::anchor::AnchorPoint;

use super::options_menu_component::{CameraOptionsView, HudOptionsView, SoundOptionsView};
use super::options_menu_sections;

const OPTIONS_CONTENT_W: f32 = 716.0;
const OPTIONS_ROW_W: f32 = OPTIONS_CONTENT_W - 30.0;
const OPTIONS_LABEL_W: f32 = 252.0;
const OPTIONS_LABEL_GAP: f32 = 16.0;
const OPTIONS_VALUE_W: f32 = 76.0;
const OPTIONS_VALUE_PAD: f32 = 8.0;
const OPTIONS_SLIDER_X: f32 = OPTIONS_LABEL_W + OPTIONS_LABEL_GAP;
const OPTIONS_TRACK_W: f32 = OPTIONS_ROW_W - OPTIONS_SLIDER_X - OPTIONS_VALUE_W - OPTIONS_VALUE_PAD;
const OPTIONS_TRACK_H: f32 = 15.0;
const OPTIONS_TRACK_BG: &str = "0.10,0.09,0.08,1.0";
const OPTIONS_TRACK_FILL: &str = "0.43,0.31,0.10,0.92";
const OPTIONS_TOGGLE_W: f32 = 170.0;
const OPTIONS_TOGGLE_H: f32 = 28.0;
const OPTIONS_TOGGLE_BG: &str = "0.10,0.09,0.08,1.0";
const OPTIONS_TOGGLE_FILL: &str = "0.43,0.31,0.10,0.92";
const OPTIONS_TOGGLE_BORDER: &str = "1px solid 0.32,0.24,0.10,0.75";
const OPTIONS_TOGGLE_TEXT_IDLE: &str = "0.70,0.66,0.56,1.0";
const OPTIONS_TOGGLE_TEXT_ACTIVE: &str = "0.95,0.90,0.74,1.0";
const OPTIONS_THUMB_W: f32 = 18.0;
const OPTIONS_THUMB_H: f32 = 22.0;

struct DynName(String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub fn sound_body(sound: &SoundOptionsView) -> Element {
    content_stack(sound_items(sound))
}

pub fn camera_body(camera: &CameraOptionsView) -> Element {
    content_stack(camera_items(camera))
}

pub fn interface_body(hud: &HudOptionsView) -> Element {
    content_stack(
        [toggle_row(
            "show_fps_overlay",
            "Show FPS Overlay",
            hud.show_fps_overlay,
        )]
        .into_iter()
        .flatten()
        .collect(),
    )
}

pub fn hud_body(hud: &HudOptionsView) -> Element {
    content_stack(
        [
            toggle_row("show_minimap", "Show Minimap", hud.show_minimap),
            toggle_row("show_action_bars", "Show Action Bars", hud.show_action_bars),
            toggle_row("show_nameplates", "Show Nameplates", hud.show_nameplates),
            toggle_row("show_health_bars", "Show Health Bars", hud.show_health_bars),
            toggle_row(
                "show_target_marker",
                "Show Target Marker",
                hud.show_target_marker,
            ),
        ]
        .into_iter()
        .flatten()
        .collect(),
    )
}

pub fn advanced_body(hud: &HudOptionsView) -> Element {
    content_stack(
        [
            toggle_row("show_fps_overlay", "Show FPS Overlay", hud.show_fps_overlay),
            options_menu_sections::info_row(
                "advanced_diag",
                "Diagnostics",
                "Runtime debug toggles can expand here",
            ),
            options_menu_sections::ghost_button_row(
                "advanced_dump",
                "UI Dump Tools",
                "Scene and UI dumps stay terminal-driven for now",
            ),
            options_menu_sections::ghost_button_row(
                "advanced_render",
                "Render Debug",
                "Render overlays need more engine hooks",
            ),
        ]
        .into_iter()
        .flatten()
        .collect(),
    )
}

fn content_stack(children: Element) -> Element {
    rsx! { r#frame { width: {OPTIONS_CONTENT_W - 30.0}, height: 0.0, layout: "flex-column", gap: 14.0, {children} } }
}

fn sound_items(sound: &SoundOptionsView) -> Element {
    [
        toggle_row("muted", "Mute All Sound", sound.muted),
        toggle_row("music_enabled", "Enable Music", sound.music_enabled),
        spacer("SoundSpacer", 18.0),
        slider_row(
            "master_volume",
            "Master Volume",
            sound.master_volume,
            0.0,
            1.0,
        ),
        slider_row("music_volume", "Music Volume", sound.music_volume, 0.0, 1.0),
        slider_row(
            "ambient_volume",
            "Ambient Volume",
            sound.ambient_volume,
            0.0,
            1.0,
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn camera_items(camera: &CameraOptionsView) -> Element {
    [
        toggle_row("invert_y", "Invert Vertical Look", camera.invert_y),
        spacer("CameraSpacer", 12.0),
        camera_sensitivity_sliders(camera),
        camera_distance_sliders(camera),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn camera_sensitivity_sliders(camera: &CameraOptionsView) -> Element {
    [
        slider_row(
            "look_sensitivity",
            "Look Sensitivity",
            camera.look_sensitivity,
            0.002,
            0.03,
        ),
        slider_row("zoom_speed", "Zoom Speed", camera.zoom_speed, 2.0, 20.0),
        slider_row(
            "follow_speed",
            "Follow Speed",
            camera.follow_speed,
            2.0,
            20.0,
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn camera_distance_sliders(camera: &CameraOptionsView) -> Element {
    [
        slider_row(
            "min_distance",
            "Min Camera Distance",
            camera.min_distance,
            1.0,
            10.0,
        ),
        slider_row(
            "max_distance",
            "Max Camera Distance",
            camera.max_distance,
            10.0,
            60.0,
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn toggle_row(key: &str, label: &str, enabled: bool) -> Element {
    rsx! {
        r#frame {
            name: {DynName(format!("ToggleRow{key}"))},
            width: {OPTIONS_CONTENT_W - 30.0},
            height: 32.0,
            {row_label(&format!("ToggleLabel{key}"), label)}
            {segmented_toggle(key, enabled)}
        }
    }
}

fn slider_row(key: &str, label: &str, value: f32, min: f32, max: f32) -> Element {
    let pct = normalize(value, min, max).clamp(0.0, 1.0);
    let action = slider_action(key);
    let slider_name = format!("Slider{key}");
    let slider_x = OPTIONS_SLIDER_X.to_string();
    rsx! { r#frame { name: {DynName(format!("SliderRow{key}"))}, width: {OPTIONS_ROW_W}, height: 44.0, {row_label(&format!("SliderLabel{key}"), label)} {slider_widget(SliderWidget { name: &slider_name, action: &action, value, min, max, width: OPTIONS_TRACK_W, interactive_height: 28.0, track_height: OPTIONS_TRACK_H, thumb_width: OPTIONS_THUMB_W, thumb_height: OPTIONS_THUMB_H, thumb_texture: None, track_color: OPTIONS_TRACK_BG, fill_color: OPTIONS_TRACK_FILL, x: &slider_x })} {slider_value_text(key, &slider_display(min, max, pct))} } }
}

fn row_label(name: &str, text: &str) -> Element {
    rsx! { fontstring { name: {DynName(name.to_string())}, width: {OPTIONS_LABEL_W}, height: 20.0, text: {text}, font_size: 16.0, color: "0.95,0.90,0.74,1.0", justify_h: "LEFT", anchor { point: AnchorPoint::Left, relative_point: AnchorPoint::Left } } }
}

fn segmented_toggle(key: &str, enabled: bool) -> Element {
    let action = toggle_action(key);
    let name = format!("ToggleSwitch{key}");
    toggle_widget(ToggleWidget {
        name: &name,
        action: &action,
        right_selected: enabled,
        width: OPTIONS_TOGGLE_W,
        height: OPTIONS_TOGGLE_H,
        left_label: "Off",
        right_label: "On",
        background_color: OPTIONS_TOGGLE_BG,
        active_color: OPTIONS_TOGGLE_FILL,
        border: OPTIONS_TOGGLE_BORDER,
        active_text_color: OPTIONS_TOGGLE_TEXT_ACTIVE,
        idle_text_color: OPTIONS_TOGGLE_TEXT_IDLE,
        x: "-8",
    })
}

fn slider_value_text(key: &str, text: &str) -> Element {
    rsx! { fontstring { name: {DynName(format!("SliderValue{key}"))}, width: 76.0, height: 20.0, text: {text}, font_size: 15.0, color: "0.95,0.90,0.74,1.0", justify_h: "RIGHT", anchor { point: AnchorPoint::Right, relative_point: AnchorPoint::Right, x: "-8" } } }
}

fn spacer(name: &str, height: f32) -> Element {
    rsx! { r#frame { name: {DynName(name.to_string())}, width: {OPTIONS_CONTENT_W - 30.0}, height: {height} } }
}

fn toggle_action(key: &str) -> String {
    format!("options_toggle:{key}")
}

fn slider_action(key: &str) -> String {
    format!("options_slider:{key}")
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
