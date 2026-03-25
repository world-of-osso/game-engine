use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::slider::{SliderWidget, slider_widget};

use crate::ui::anchor::AnchorPoint;

use super::options_menu_component::{CameraOptionsView, HudOptionsView, SoundOptionsView};
use super::options_menu_sections;

const OPTIONS_CONTENT_W: f32 = 716.0;
const OPTIONS_TRACK_W: f32 = 270.0;
const OPTIONS_TRACK_H: f32 = 10.0;
const OPTIONS_TRACK_BG: &str = "0.10,0.09,0.08,1.0";
const OPTIONS_TRACK_FILL: &str = "0.43,0.31,0.10,0.92";
const OPTIONS_THUMB_W: f32 = 18.0;
const OPTIONS_THUMB_H: f32 = 22.0;
const OPTIONS_THUMB_TEXTURE: &str = "data/textures/ui/options_slider_thumb.png";
const BUTTON_ATLAS_UP: &str = "defaultbutton-nineslice-up";
const BUTTON_ATLAS_PRESSED: &str = "defaultbutton-nineslice-pressed";
const BUTTON_ATLAS_HIGHLIGHT: &str = "defaultbutton-nineslice-highlight";
const BUTTON_ATLAS_DISABLED: &str = "defaultbutton-nineslice-disabled";

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
        slider_row(
            "footstep_volume",
            "Footstep Volume",
            sound.footstep_volume,
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
    let state = if enabled { "Enabled" } else { "Disabled" };
    rsx! { r#frame { name: {DynName(format!("ToggleRow{key}"))}, width: {OPTIONS_CONTENT_W - 30.0}, height: 32.0, {row_label(&format!("ToggleLabel{key}"), label)} {small_button(&format!("ToggleButton{key}"), state, &toggle_action(key), 116.0, "566")} } }
}

fn slider_row(key: &str, label: &str, value: f32, min: f32, max: f32) -> Element {
    let pct = normalize(value, min, max).clamp(0.0, 1.0);
    let action = slider_action(key);
    let slider_name = format!("Slider{key}");
    rsx! { r#frame { name: {DynName(format!("SliderRow{key}"))}, width: {OPTIONS_CONTENT_W - 30.0}, height: 44.0, {row_label(&format!("SliderLabel{key}"), label)} {slider_widget(SliderWidget { name: &slider_name, action: &action, value, min, max, width: OPTIONS_TRACK_W, interactive_height: 28.0, track_height: OPTIONS_TRACK_H, thumb_width: OPTIONS_THUMB_W, thumb_height: OPTIONS_THUMB_H, thumb_texture: OPTIONS_THUMB_TEXTURE, track_color: OPTIONS_TRACK_BG, fill_color: OPTIONS_TRACK_FILL, x: "286" })} {small_button(&format!("SliderMinus{key}"), "-", &step_action(key, -1), 30.0, "604")} {small_button(&format!("SliderPlus{key}"), "+", &step_action(key, 1), 30.0, "640")} {slider_value_text(key, &slider_display(min, max, pct))} } }
}

fn row_label(name: &str, text: &str) -> Element {
    rsx! { fontstring { name: {DynName(name.to_string())}, width: 252.0, height: 20.0, text: {text}, font_size: 16.0, color: "0.95,0.90,0.74,1.0", justify_h: "LEFT", anchor { point: AnchorPoint::Left, relative_point: AnchorPoint::Left } } }
}

fn slider_value_text(key: &str, text: &str) -> Element {
    rsx! { fontstring { name: {DynName(format!("SliderValue{key}"))}, width: 76.0, height: 20.0, text: {text}, font_size: 15.0, color: "0.95,0.90,0.74,1.0", justify_h: "RIGHT", anchor { point: AnchorPoint::Right, relative_point: AnchorPoint::Right, x: "-100" } } }
}

fn small_button(name: &str, text: &str, action: &str, width: f32, x: &str) -> Element {
    rsx! { button { name: {DynName(name.to_string())}, width: {width}, height: 30.0, text: {text}, font_size: 14.0, onclick: {action}, button_atlas_up: BUTTON_ATLAS_UP, button_atlas_pressed: BUTTON_ATLAS_PRESSED, button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT, button_atlas_disabled: BUTTON_ATLAS_DISABLED, anchor { point: AnchorPoint::Right, relative_point: AnchorPoint::Right, x: {x} } } }
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

fn step_action(key: &str, delta: i32) -> String {
    format!("options_step:{key}:{delta}")
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
