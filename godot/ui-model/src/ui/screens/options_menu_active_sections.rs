use crate::options_dropdown::OptionsDropdownMenu;
use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::slider::{SliderWidget, slider_widget};

use super::options_menu_component::{
    CameraOptionsView, GraphicsOptionsView, HudOptionsView, KeybindingRowView, KeybindingsView,
    LayoutOptionsView, OptionsPage, SoundOptionsView, keybinding_button_name,
    keybinding_rebind_action, keybinding_section_action,
};
use super::options_menu_sections;
use crate::input_bindings::BindingSection;
use crate::soft_target_data::InteractKeyIcons;
use crate::status_text_data::StatusTextDisplay;

/// `choice_row` key of the HUD "Layout" dropdown.
pub const LAYOUT_CHOICE_KEY: &str = "ui_layout";
/// `choice_row` keys of the "Layout Settings" system selector and its "Font" dropdown.
pub const LAYOUT_SYSTEM_KEY: &str = "layout_system";
pub const LAYOUT_FONT_KEY: &str = "layout_font";

#[path = "options_menu_active_sections_keybindings.rs"]
mod keybindings_section;
#[path = "options_menu_active_sections_layout.rs"]
mod layout_section;
#[path = "options_menu_active_sections_nameplates.rs"]
mod nameplates_section;

const OPTIONS_CONTENT_W: f32 = 665.0;
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
const OPTIONS_THUMB_W: f32 = 18.0;
const OPTIONS_THUMB_H: f32 = 22.0;
const UI_SCALE_MIN: f32 = 0.75;
const UI_SCALE_MAX: f32 = 1.5;
const FRAME_RATE_LIMIT_MIN: f32 = 30.0;
const FRAME_RATE_LIMIT_MAX: f32 = 240.0;
const MOUSE_SENSITIVITY_MIN: f32 = 0.001;
const MOUSE_SENSITIVITY_MAX: f32 = 0.01;
const NAMEPLATE_DISTANCE_MIN: f32 = 20.0;
const NAMEPLATE_DISTANCE_MAX: f32 = 80.0;
const CHAT_FONT_SIZE_MIN: f32 = 8.0;
const CHAT_FONT_SIZE_MAX: f32 = 16.0;
const BINDING_BUTTON_W: f32 = 160.0;
const BINDING_BUTTON_H: f32 = 22.0;
const BINDING_OUTPUT_H: f32 = 18.0;
/// `GameFontHighlightSmall` white; `GRAY_FONT_COLOR` at the button's 0.8 alpha.
const BINDING_TEXT_COLOR: &str = "1.0,1.0,1.0,1.0";
const BINDING_NOT_BOUND_COLOR: &str = "0.5,0.5,0.5,0.8";
/// `OutputText` is `GameFontWhite` (`Blizzard_SettingsPanel.xml:18`); `KEY_UNBOUND_ERROR`
/// is wrapped in `|cffff0000`.
const BINDING_OUTPUT_COLOR: &str = "1.0,1.0,1.0,1.0";
const BINDING_OUTPUT_ERROR_COLOR: &str = "1.0,0.0,0.0,1.0";

#[derive(Clone)]
struct DynName(String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub fn sound_body(sound: &SoundOptionsView) -> OptionsPage {
    content_stack(sound_items(sound))
}

pub fn graphics_body(graphics: &GraphicsOptionsView) -> OptionsPage {
    content_stack(graphics_items(graphics))
}

pub fn camera_body(camera: &CameraOptionsView) -> OptionsPage {
    content_stack(camera_items(camera))
}

/// Retail Gameplay > Action Bars (ActionBars.lua:29–47): supported bars in
/// numeric order. 2/3 retain the client's authored per-skin defaults.
pub fn action_bars_body(
    hud: &HudOptionsView,
    skin: game_engine_core::ui_layout_data::LayoutSkin,
) -> OptionsPage {
    let bars = hud.extra_action_bars;
    let bottom_default = skin == game_engine_core::ui_layout_data::LayoutSkin::Forever;
    let rows = [
        (
            "action_bar_2",
            "Action Bar 2",
            bars.action_bar_2.unwrap_or(bottom_default),
        ),
        (
            "action_bar_3",
            "Action Bar 3",
            bars.action_bar_3.unwrap_or(bottom_default),
        ),
        ("action_bar_4", "Action Bar 4", bars.action_bar_4),
        ("action_bar_5", "Action Bar 5", bars.action_bar_5),
    ];
    content_stack(
        rows.into_iter()
            .flat_map(|(key, label, checked)| {
                let name = format!("ExtraActionBar{key}");
                let action = format!("options_toggle:{key}");
                let checkbox =
                    super::bank_art::checkbox(&name, label, checked, &action, (8.0, 10.0));
                rsx! {
                    r#frame {
                        name: {DynName(format!("ExtraActionBarRow{key}"))},
                        width: OPTIONS_ROW_W, height: 44.0,
                        mouse_enabled: true, onclick: {action.as_str()},
                        {checkbox}
                    }
                }
            })
            .collect(),
    )
}

pub fn interface_body(hud: &HudOptionsView, open: Option<&OptionsDropdownMenu>) -> OptionsPage {
    content_stack(
        [
            toggle_row(
                "encounter_timeline",
                "Encounter Timeline",
                hud.encounter_timeline,
            ),
            choice_row(
                "encounter_view",
                "Encounter View",
                &[(1, "Timeline"), (2, "Bars")],
                if hud.encounter_bars { 2 } else { 1 },
                open,
            ),
            toggle_row(
                "encounter_warnings",
                "Encounter Warnings",
                hud.encounter_warnings,
            ),
            toggle_row(
                "encounter_sounds",
                "Encounter Warning Sounds",
                hud.encounter_sounds,
            ),
            status_text_row(hud.status_text_display, open),
            slider_row(
                "chat_font_size",
                "Chat Font Size",
                hud.chat_font_size,
                CHAT_FONT_SIZE_MIN,
                CHAT_FONT_SIZE_MAX,
            ),
            options_menu_sections::info_row(
                "interface_status",
                "Communities Chat",
                "Applies to the existing communities chat tab without changing world render scale",
            ),
            options_menu_sections::action_button_row(
                "reset_window_positions",
                "Reset Window Positions",
                "Moved windows reopen in their default slots",
                "Reset",
                super::options_menu_component::ACTION_RESET_WINDOW_POSITIONS,
            ),
        ]
        .into_iter()
        .flatten()
        .collect(),
    )
}

pub fn accessibility_body(
    graphics: &GraphicsOptionsView,
    hud: &HudOptionsView,
    open: Option<&OptionsDropdownMenu>,
) -> OptionsPage {
    content_stack(accessibility_items(graphics, hud, open))
}

fn accessibility_ui_scale_item(ui_scale: f32) -> Element {
    slider_row("ui_scale", "UI Scale", ui_scale, UI_SCALE_MIN, UI_SCALE_MAX)
}

fn accessibility_items(
    graphics: &GraphicsOptionsView,
    hud: &HudOptionsView,
    open: Option<&OptionsDropdownMenu>,
) -> Element {
    [
        accessibility_ui_scale_item(graphics.ui_scale),
        accessibility_colorblind_item(graphics.colorblind_mode),
        interact_key_icons_row(hud.interact_key_icons, open),
        accessibility_info_rows(),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn accessibility_colorblind_item(colorblind_mode: bool) -> Element {
    toggle_row("colorblind_mode", "Colorblind Mode", colorblind_mode)
}

/// Retail Accessibility "Interact Key Icons" dropdown (Accessibility.lua:176-221), its
/// three choices side by side.
fn interact_key_icons_row(
    selected: InteractKeyIcons,
    open: Option<&OptionsDropdownMenu>,
) -> Element {
    let choices = InteractKeyIcons::ALL.map(|choice| (choice.value(), choice.label()));
    choice_row(
        "interact_key_icons",
        "Interact Key Icons",
        &choices,
        selected.value(),
        open,
    )
}

/// Retail Interface → Display "Status Text" dropdown (Interface.lua:57-105), its four
/// choices side by side.
fn status_text_row(selected: StatusTextDisplay, open: Option<&OptionsDropdownMenu>) -> Element {
    let choices = StatusTextDisplay::ALL.map(|choice| (choice.value(), choice.label()));
    choice_row(
        "status_text_display",
        "Status Text",
        &choices,
        selected.value(),
        open,
    )
}

/// Retail Edit Mode "Layout" dropdown (`Blizzard_EditMode/Shared/EditModeManager.xml:50`,
/// `EditModeManager.lua:1277-1310`): the system presets, then the saved player layouts.
/// This client has no Edit Mode window yet, so the dropdown heads HUD options.
fn layout_row(layout: &LayoutOptionsView, open: Option<&OptionsDropdownMenu>) -> Element {
    let choices: Vec<(u8, &str)> = (0u8..)
        .zip(layout.names.iter().map(String::as_str))
        .collect();
    let selected = choices
        .iter()
        .find_map(|&(index, name)| (name == layout.active).then_some(index))
        .unwrap_or(u8::MAX);
    choice_row(LAYOUT_CHOICE_KEY, "Layout", &choices, selected, open)
}

/// A dropdown's `(value, label)` choices as one segmented row; the selected one is lit.
fn choice_row(
    key: &str,
    label: &str,
    choices: &[(u8, &str)],
    selected: u8,
    open: Option<&OptionsDropdownMenu>,
) -> Element {
    use super::menu_primitives::{
        DropdownButton, DropdownState, DropdownStyle, dropdown_button_with_state,
    };
    let selected_text = choices
        .iter()
        .find_map(|(value, text)| (*value == selected).then_some(*text))
        .unwrap_or("");
    let menu = open.filter(|menu| menu.key == key);
    let action = format!("options_dropdown:{key}");
    // SettingsDropdownControlMixin width220, LEFT at CENTER-80; WowStyle2 height25.
    let dropdown = dropdown_button_with_state(
        DropdownButton {
            frame_name: &format!("Choice{key}"),
            label_name: &format!("Choice{key}Text"),
            arrow_name: &format!("Choice{key}Arrow"),
            text: selected_text,
            width: 220.0,
            height: 25.0,
            x: OPTIONS_ROW_W / 2.0 - 80.0,
            y: -6.5,
            background_color: "",
            text_color: "1,1,1,1",
            arrow_color: "",
            onclick: Some(&action),
        },
        DropdownState {
            style: DropdownStyle::WowStyle2,
            open: menu.is_some(),
            enabled: !choices.is_empty(),
            reset_action: None,
            resize_to_text: true,
        },
    );
    let popup = menu
        .map(|menu| {
            crate::options_dropdown::radio_popup(key, label, choices, selected, menu.position)
        })
        .unwrap_or_default();
    rsx! { r#frame { name: DynName(format!("ChoiceRow{key}")), width: OPTIONS_ROW_W, height: 44.0,
        {row_label(&format!("ChoiceLabel{key}"),label)} {dropdown} {popup}
    } }
}

fn accessibility_info_rows() -> Element {
    [
        options_menu_sections::info_row(
            "access_text",
            "Readable Text",
            "Scales the full HUD, menus, and overlays without changing 3D render resolution",
        ),
        options_menu_sections::info_row(
            "access_colorblind",
            "Nameplates and Debuffs",
            "Swaps red/green status cues to higher-contrast colors for nameplates and debuff borders",
        ),
        options_menu_sections::info_row(
            "access_motion",
            "Reduced Motion",
            "Animation dampening hooks reserved",
        ),
        options_menu_sections::info_row(
            "access_subtitles",
            "Subtitles",
            "Dialog subtitle pipeline not landed yet",
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

pub fn hud_body(
    hud: &HudOptionsView,
    layout: &LayoutOptionsView,
    open: Option<&OptionsDropdownMenu>,
) -> OptionsPage {
    content_stack(
        [
            layout_row(layout, open),
            layout_section::layout_settings_rows(layout, open),
            toggle_row("show_minimap", "Show Minimap", hud.show_minimap),
            toggle_row("show_action_bars", "Show Action Bars", hud.show_action_bars),
            toggle_row(
                "cooldown_viewer_enabled",
                "Enable Cooldown Manager",
                hud.cooldown_assist.viewer_enabled,
            ),
            toggle_row(
                "assisted_highlight",
                "Assisted Highlight",
                hud.cooldown_assist.highlight_action_bar,
            ),
            toggle_row(
                "cooldown_assisted_highlight",
                "Highlight Cooldown Icons (World of Osso)",
                hud.cooldown_assist.highlight_viewer,
            ),
            options_menu_sections::action_button_row(
                "cooldown_settings",
                "Cooldown Manager Settings",
                "Choose and reorder specialization spells",
                "Open",
                "cooldown_settings",
            ),
            toggle_row("show_nameplates", "Show Nameplates", hud.show_nameplates),
            slider_row(
                "nameplate_distance",
                "Nameplate Distance",
                hud.nameplate_distance,
                NAMEPLATE_DISTANCE_MIN,
                NAMEPLATE_DISTANCE_MAX,
            ),
            thickness_row(
                "nameplate_health_thickness",
                "Nameplate Health Thickness",
                hud.nameplate_style.health_preset(),
                open,
            ),
            thickness_row(
                "nameplate_spellbar_thickness",
                "Nameplate Spellbar Thickness",
                hud.nameplate_style.cast_preset(),
                open,
            ),
            toggle_row("show_health_bars", "Show Health Bars", hud.show_health_bars),
            toggle_row(
                "show_target_marker",
                "Show Target Marker",
                hud.show_target_marker,
            ),
            // `nameplateShowSelf` (Combat: Personal Resource Display, Combat.lua:14-17).
            toggle_row(
                "personal_resource_display",
                "Personal Resource Display",
                hud.personal_resource_display,
            ),
            // `autoLootDefault` (Controls: Auto Loot); Shift inverts it.
            toggle_row("auto_loot", "Auto Loot", hud.auto_loot),
            // `ENABLE_INTERACT_TEXT` (Controls: Enable Interact Key, Controls.lua:72-86).
            toggle_row(
                "soft_target_interact",
                "Enable Interact Key",
                hud.soft_target_interact,
            ),
        ]
        .into_iter()
        .flatten()
        .collect(),
    )
}

pub fn nameplates_body(hud: &HudOptionsView) -> OptionsPage {
    nameplates_section::nameplates_body(hud)
}

pub fn advanced_body(hud: &HudOptionsView) -> OptionsPage {
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

pub fn keybindings_body(bindings: &KeybindingsView) -> OptionsPage {
    keybindings_section::keybindings_body(bindings)
}

fn content_stack(items: Element) -> OptionsPage {
    OptionsPage { items, gap: 14.0 }
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
            "effects_volume",
            "Effects Volume",
            sound.effects_volume,
            0.0,
            1.0,
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn graphics_items(graphics: &GraphicsOptionsView) -> Element {
    [
        toggle_row(
            "ssao_enabled",
            "SSAO (Requires Restart)",
            graphics.ssao_enabled,
        ),
        frame_pacing_items(graphics),
        render_scale_items(graphics),
        bloom_items(graphics),
        options_menu_sections::info_row(
            "ssao_detail",
            "Contact Shading",
            "Requires antiAlias: None or Taa; takes effect next launch",
        ),
        particle_density_item(graphics),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn frame_pacing_items(graphics: &GraphicsOptionsView) -> Element {
    [
        toggle_row("vsync_enabled", "Vertical Sync", graphics.vsync_enabled),
        toggle_row(
            "frame_rate_limit_enabled",
            "Limit Frame Rate",
            graphics.frame_rate_limit_enabled,
        ),
        slider_row(
            "frame_rate_limit",
            "Frame Rate Cap",
            graphics.frame_rate_limit,
            FRAME_RATE_LIMIT_MIN,
            FRAME_RATE_LIMIT_MAX,
        ),
        options_menu_sections::info_row(
            "frame_pacing_detail",
            "Presentation",
            "VSync switches swap pacing; the frame cap adds a CPU-side limit when enabled",
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn render_scale_items(graphics: &GraphicsOptionsView) -> Element {
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
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn bloom_items(graphics: &GraphicsOptionsView) -> Element {
    [
        toggle_row("bloom_enabled", "Enable Bloom", graphics.bloom_enabled),
        slider_row(
            "bloom_intensity",
            "Bloom Intensity",
            graphics.bloom_intensity,
            0.0,
            1.0,
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn particle_density_item(graphics: &GraphicsOptionsView) -> Element {
    slider_row(
        "particle_density",
        "Particle Density",
        graphics.particle_density,
        10.0,
        100.0,
    )
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
    camera_sensitivity_slider_elements(camera)
        .into_iter()
        .flatten()
        .collect()
}

type CameraSensitivitySliderSpec = (&'static str, &'static str, f32, f32, f32);

fn camera_sensitivity_slider_elements(camera: &CameraOptionsView) -> [Element; 5] {
    camera_sensitivity_slider_specs(camera).map(camera_sensitivity_slider_row)
}

fn camera_sensitivity_slider_specs(camera: &CameraOptionsView) -> [CameraSensitivitySliderSpec; 5] {
    [
        mouse_sensitivity_slider_spec(camera),
        fov_degrees_slider_spec(camera),
        look_sensitivity_slider_spec(camera),
        zoom_speed_slider_spec(camera),
        follow_speed_slider_spec(camera),
    ]
}

fn camera_sensitivity_slider_row(spec: CameraSensitivitySliderSpec) -> Element {
    let (key, label, value, min, max) = spec;
    slider_row(key, label, value, min, max)
}

fn mouse_sensitivity_slider_spec(camera: &CameraOptionsView) -> CameraSensitivitySliderSpec {
    (
        "mouse_sensitivity",
        "Mouse Sensitivity",
        camera.mouse_sensitivity,
        MOUSE_SENSITIVITY_MIN,
        MOUSE_SENSITIVITY_MAX,
    )
}

fn fov_degrees_slider_spec(camera: &CameraOptionsView) -> CameraSensitivitySliderSpec {
    (
        "fov_degrees",
        "Field of View",
        camera.fov_degrees,
        90.0,
        120.0,
    )
}

fn look_sensitivity_slider_spec(camera: &CameraOptionsView) -> CameraSensitivitySliderSpec {
    (
        "look_sensitivity",
        "Look Sensitivity",
        camera.look_sensitivity,
        0.002,
        0.03,
    )
}

fn zoom_speed_slider_spec(camera: &CameraOptionsView) -> CameraSensitivitySliderSpec {
    ("zoom_speed", "Zoom Speed", camera.zoom_speed, 2.0, 20.0)
}

fn follow_speed_slider_spec(camera: &CameraOptionsView) -> CameraSensitivitySliderSpec {
    (
        "follow_speed",
        "Follow Speed",
        camera.follow_speed,
        2.0,
        20.0,
    )
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
            pos_type: "absolute",
            left: 0.0,
            top: "50%",
            translate_y: "-50%",
        }
    }
}

fn thickness_row(
    key: &str,
    label: &str,
    thickness: super::options_menu_component::NameplateBarThickness,
    open: Option<&OptionsDropdownMenu>,
) -> Element {
    let selected = if thickness == super::options_menu_component::NameplateBarThickness::Thick {
        1
    } else {
        0
    };
    choice_row(key, label, &[(0, "Thin"), (1, "Thick")], selected, open)
}

fn segmented_toggle(key: &str, enabled: bool) -> Element {
    let hide_check = !enabled;
    let box_art =
        ui_toolkit::atlas::get_region("checkbox-minimal").expect("SettingsCheckboxTemplate normal");
    let check_art = ui_toolkit::atlas::get_region("checkmark-minimal")
        .expect("SettingsCheckboxTemplate checked");
    rsx! { button {
        name: DynName(format!("ToggleSwitch{key}")), width: 30.0, height: 29.0,
        onclick: toggle_action(key), button_default_skin: false,
        pos_type: "absolute", right: 8.0, top: "50%", translate_y: "-50%",
        texture { name: DynName(format!("ToggleSwitch{key}Box")), texture_atlas: "checkbox-minimal",
            width: box_art.width, height: box_art.height, pos_type: "absolute", left: "50%", top: "50%", translate_x: "-50%", translate_y: "-50%", }
        texture { name: DynName(format!("ToggleSwitch{key}Checked")), texture_atlas: "checkmark-minimal", hidden: hide_check,
            width: check_art.width, height: check_art.height, pos_type: "absolute", left: "50%", top: "50%", translate_x: "-50%", translate_y: "-50%", }
    } }
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
            pos_type: "absolute",
            right: 8.0,
            top: "50%",
            translate_y: "-50%",
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
