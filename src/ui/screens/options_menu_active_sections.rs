use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::text_measure::measure_text;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::font_string::GameFont;
use ui_toolkit::widgets::slider::{SliderWidget, slider_widget};
use ui_toolkit::widgets::toggle::{ToggleWidget, toggle_widget};

use crate::ui::anchor::AnchorPoint;

use super::options_menu_component::{
    CameraOptionsView, GraphicsOptionsView, HudOptionsView, KeybindingRowView, KeybindingsView,
    SoundOptionsView, keybinding_clear_action, keybinding_rebind_action, keybinding_section_action,
};
use super::options_menu_sections;
use crate::input_bindings::BindingSection;

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
const BINDING_VALUE_W: f32 = 180.0;
const KEYBINDING_TAB_LEAD_X: f32 = 11.0;
const KEYBINDING_TAB_H: f32 = 32.0;
const KEYBINDING_TAB_GAP: f32 = 1.0;
const KEYBINDING_TAB_SIDE_PADDING: f32 = 20.0;
const KEYBINDING_TAB_FONT_SIZE: f32 = 10.0;
const KEYBINDING_TAB_LABEL_Y_IDLE: f32 = 2.0;
const KEYBINDING_TAB_LABEL_Y_ACTIVE: f32 = -3.0;
const KEYBINDING_TAB_TEXT_IDLE: &str = "0.85,0.78,0.61,1.0";
const KEYBINDING_TAB_TEXT_ACTIVE: &str = "0.96,0.84,0.56,1.0";

struct DynName(String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub fn sound_body(sound: &SoundOptionsView) -> Element {
    content_stack(sound_items(sound))
}

pub fn graphics_body(graphics: &GraphicsOptionsView) -> Element {
    content_stack(
        [
            slider_row(
                "render_scale",
                "Render Scale",
                graphics.render_scale,
                0.5,
                1.0,
            ),
            options_menu_sections::info_row(
                "render_scale_presets",
                "Presets",
                "Native 1.00 • Quality 0.75 • Balanced 0.67 • Performance 0.50",
            ),
            toggle_row("bloom_enabled", "Enable Bloom", graphics.bloom_enabled),
            slider_row(
                "bloom_intensity",
                "Bloom Intensity",
                graphics.bloom_intensity,
                0.0,
                1.0,
            ),
            slider_row(
                "particle_density",
                "Particle Density",
                graphics.particle_density,
                10.0,
                100.0,
            ),
        ]
        .into_iter()
        .flatten()
        .collect(),
    )
}

pub fn camera_body(camera: &CameraOptionsView) -> Element {
    content_stack(camera_items(camera))
}

pub fn interface_body(hud: &HudOptionsView) -> Element {
    let _ = hud;
    content_stack(
        [options_menu_sections::info_row(
            "interface_status",
            "Interface",
            "More interface-specific controls can land here",
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

pub fn keybindings_body(bindings: &KeybindingsView) -> Element {
    content_stack(
        [
            keybinding_section_tabs(bindings.section),
            spacer("KeybindingsSpacer", 6.0),
            keybinding_rows(bindings),
        ]
        .into_iter()
        .flatten()
        .collect(),
    )
}

fn content_stack(children: Element) -> Element {
    rsx! {
        r#frame {
            width: {OPTIONS_CONTENT_W - 30.0},
            height: 0.0,
            layout: "flex-column",
            gap: 14.0,
            {children}
        }
    }
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

fn keybinding_section_tabs(active: BindingSection) -> Element {
    let buttons: Element = BindingSection::ALL
        .iter()
        .flat_map(|section| keybinding_section_button(*section, *section == active))
        .collect();
    rsx! {
        r#frame {
            name: "KeybindingSectionTabs",
            width: {OPTIONS_ROW_W},
            height: KEYBINDING_TAB_H,
            layout: "flex-row",
            gap: KEYBINDING_TAB_GAP,
            r#frame {
                name: "KeybindingSectionTabsLeadSpacer",
                width: KEYBINDING_TAB_LEAD_X,
                height: KEYBINDING_TAB_H,
            }
            {buttons}
        }
    }
}

fn keybinding_section_button(section: BindingSection, active: bool) -> Element {
    let label = section.title().to_string();
    let action = keybinding_section_action(section);
    let names = keybinding_section_tab_names(section);
    let visuals = keybinding_section_tab_visuals(active);
    let width = keybinding_section_tab_width(&label);
    rsx! {
        r#frame {
            name: {names.frame},
            width: {width},
            height: KEYBINDING_TAB_H,
            button {
                name: {names.button},
                stretch: true,
                text: "",
                font_size: KEYBINDING_TAB_FONT_SIZE,
                onclick: {&action},
                button_atlas_up: visuals.atlas_up,
                button_atlas_pressed: visuals.atlas_pressed,
                button_atlas_highlight: visuals.atlas_highlight,
                button_atlas_disabled: "defaultbutton-nineslice-disabled",
            }
            fontstring {
                name: {names.label},
                width: {width},
                height: KEYBINDING_TAB_H,
                text: {&label},
                font: "FrizQuadrata",
                font_size: KEYBINDING_TAB_FONT_SIZE,
                font_color: visuals.text_color,
                shadow_color: "0.0,0.0,0.0,1.0",
                shadow_offset: "1,-1",
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::Center,
                    relative_point: AnchorPoint::Center,
                    y: {visuals.text_y},
                }
            }
        }
    }
}

struct KeybindingSectionTabNames {
    frame: DynName,
    button: DynName,
    label: DynName,
}

fn keybinding_section_tab_names(section: BindingSection) -> KeybindingSectionTabNames {
    KeybindingSectionTabNames {
        frame: DynName(format!("KeybindingSection{}", section.key())),
        button: DynName(format!("KeybindingSection{}Button", section.key())),
        label: DynName(format!("KeybindingSection{}Label", section.key())),
    }
}

struct KeybindingSectionTabVisuals {
    text_y: f32,
    atlas_up: &'static str,
    atlas_pressed: &'static str,
    atlas_highlight: &'static str,
    text_color: &'static str,
}

fn keybinding_section_tab_visuals(active: bool) -> KeybindingSectionTabVisuals {
    KeybindingSectionTabVisuals {
        text_y: if active {
            KEYBINDING_TAB_LABEL_Y_ACTIVE
        } else {
            KEYBINDING_TAB_LABEL_Y_IDLE
        },
        atlas_up: if active {
            "defaultbutton-nineslice-pressed"
        } else {
            "defaultbutton-nineslice-up"
        },
        atlas_pressed: "defaultbutton-nineslice-pressed",
        atlas_highlight: if active {
            "defaultbutton-nineslice-pressed"
        } else {
            "defaultbutton-nineslice-highlight"
        },
        text_color: if active {
            KEYBINDING_TAB_TEXT_ACTIVE
        } else {
            KEYBINDING_TAB_TEXT_IDLE
        },
    }
}

fn keybinding_section_tab_width(label: &str) -> f32 {
    let (text_width, _) =
        measure_text(label, GameFont::FrizQuadrata, KEYBINDING_TAB_FONT_SIZE).unwrap_or((0.0, 0.0));
    (text_width + KEYBINDING_TAB_SIDE_PADDING).ceil().max(10.0)
}

fn keybinding_rows(bindings: &KeybindingsView) -> Element {
    bindings.rows.iter().flat_map(keybinding_row).collect()
}

fn keybinding_row(row: &KeybindingRowView) -> Element {
    rsx! {
        r#frame {
            name: {DynName(format!("KeybindingRow{}", row.action.key()))},
            width: {OPTIONS_ROW_W},
            height: 34.0,
            {row_label(&format!("KeybindingLabel{}", row.action.key()), &row.label)}
            {keybinding_value(row)}
            {keybinding_clear_button(row)}
            {keybinding_rebind_button(row)}
        }
    }
}

fn keybinding_value(row: &KeybindingRowView) -> Element {
    let text = if row.capturing {
        "Press a key or mouse button...".to_string()
    } else {
        row.binding_text.clone()
    };
    rsx! {
        fontstring {
            name: {DynName(format!("KeybindingValue{}", row.action.key()))},
            width: {BINDING_VALUE_W},
            height: 20.0,
            text: {&text},
            font_size: 14.0,
            color: "0.95,0.90,0.74,1.0",
            justify_h: "RIGHT",
            anchor {
                point: AnchorPoint::Right,
                relative_point: AnchorPoint::Right,
                x: "-176",
            }
        }
    }
}

fn keybinding_clear_button(row: &KeybindingRowView) -> Element {
    let action = keybinding_clear_action(row.action);
    rsx! {
        button {
            name: {DynName(format!("KeybindingClear{}", row.action.key()))},
            width: 72.0,
            height: 28.0,
            text: "Clear",
            font_size: 13.0,
            onclick: {&action},
            disabled: {!row.can_clear},
            button_atlas_up: "defaultbutton-nineslice-up",
            button_atlas_pressed: "defaultbutton-nineslice-pressed",
            button_atlas_highlight: "defaultbutton-nineslice-highlight",
            button_atlas_disabled: "defaultbutton-nineslice-disabled",
            anchor { point: AnchorPoint::Right, relative_point: AnchorPoint::Right, x: "-84" }
        }
    }
}

fn keybinding_rebind_button(row: &KeybindingRowView) -> Element {
    let action = keybinding_rebind_action(row.action);
    rsx! {
        button {
            name: {DynName(format!("KeybindingRebind{}", row.action.key()))},
            width: 72.0,
            height: 28.0,
            text: "Rebind",
            font_size: 13.0,
            onclick: {&action},
            button_atlas_up: "defaultbutton-nineslice-up",
            button_atlas_pressed: "defaultbutton-nineslice-pressed",
            button_atlas_highlight: "defaultbutton-nineslice-highlight",
            button_atlas_disabled: "defaultbutton-nineslice-disabled",
            anchor { point: AnchorPoint::Right, relative_point: AnchorPoint::Right }
        }
    }
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
    rsx! {
        r#frame {
            name: {DynName(format!("SliderRow{key}"))},
            width: {OPTIONS_ROW_W},
            height: 44.0,
            {row_label(&format!("SliderLabel{key}"), label)}
            {
                slider_widget(SliderWidget {
                    name: &slider_name,
                    action: &action,
                    value,
                    min,
                    max,
                    width: OPTIONS_TRACK_W,
                    interactive_height: 28.0,
                    track_height: OPTIONS_TRACK_H,
                    thumb_width: OPTIONS_THUMB_W,
                    thumb_height: OPTIONS_THUMB_H,
                    thumb_texture: None,
                    track_color: OPTIONS_TRACK_BG,
                    fill_color: OPTIONS_TRACK_FILL,
                    x: &slider_x,
                })
            }
            {slider_value_text(key, &slider_display(min, max, pct))}
        }
    }
}

fn row_label(name: &str, text: &str) -> Element {
    rsx! {
        fontstring {
            name: {DynName(name.to_string())},
            width: {OPTIONS_LABEL_W},
            height: 20.0,
            text: {text},
            font_size: 16.0,
            color: "0.95,0.90,0.74,1.0",
            justify_h: "LEFT",
            anchor { point: AnchorPoint::Left, relative_point: AnchorPoint::Left }
        }
    }
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
    rsx! {
        fontstring {
            name: {DynName(format!("SliderValue{key}"))},
            width: 76.0,
            height: 20.0,
            text: {text},
            font_size: 15.0,
            color: "0.95,0.90,0.74,1.0",
            justify_h: "RIGHT",
            anchor {
                point: AnchorPoint::Right,
                relative_point: AnchorPoint::Right,
                x: "-8",
            }
        }
    }
}

fn spacer(name: &str, height: f32) -> Element {
    rsx! {
        r#frame {
            name: {DynName(name.to_string())},
            width: {OPTIONS_CONTENT_W - 30.0},
            height: {height},
        }
    }
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
