//! Options > HUD "Layout Settings": the active Edit Mode layout's per-system settings
//! (docs/specs/hud-edit-mode.md "Customisable layout settings"). One system shows at a
//! time, as in Retail's `EditModeSystemSettingsDialog`
//! (`Blizzard_EditMode/Shared/EditModeDialogs.lua:537-540,598`).
use super::nameplates_section::{cell_row, slider_cell};
use super::*;
use crate::hud_layout::layout_with;
use crate::options_menu_data::{LAYOUT_FONTS, LayoutSlider, SliderField, slider_key};
use crate::ui::screens::options_menu_component::{ACTION_RESET_LAYOUT_SETTINGS, LayoutSystem};
use game_engine_core::ui_layout_data::{LayoutFont, UnitFrameSettings};

pub(super) fn layout_settings_rows(layout: &LayoutOptionsView) -> Element {
    let settings = &layout.settings;
    let system_rows = match layout.system {
        LayoutSystem::PlayerFrame => unit_frame_rows(settings.player_frame),
        LayoutSystem::TargetFrame => unit_frame_rows(settings.target_frame),
        LayoutSystem::FocusFrame => unit_frame_rows(settings.focus_frame),
        LayoutSystem::PetFrame => unit_frame_rows(settings.pet_frame),
        LayoutSystem::ChatFrame => {
            let (width, height) = layout_with(layout.skin, settings).chat_size;
            // `HUD_EDIT_MODE_SETTING_CHAT_FRAME_WIDTH` / `_HEIGHT`
            // (`EditModeSettingDisplayInfo.lua:565,581`).
            size_row(
                (LayoutSlider::ChatWidth, "Width", width),
                (LayoutSlider::ChatHeight, "Height", height),
            )
        }
        LayoutSystem::DamageMeter => {
            let (width, height) = layout_with(layout.skin, settings).damage_meter_size;
            // `HUD_EDIT_MODE_SETTING_DAMAGE_METER_FRAME_WIDTH` / `_HEIGHT` (`:1279,1293`).
            size_row(
                (LayoutSlider::MeterWidth, "Frame Width", width),
                (LayoutSlider::MeterHeight, "Frame Height", height),
            )
        }
    };
    [
        system_row(layout.system),
        system_rows,
        options_menu_sections::action_button_row(
            "reset_layout_settings",
            "Reset to Preset",
            "Clears this layout's sizes and fonts",
            "Reset",
            ACTION_RESET_LAYOUT_SETTINGS,
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn system_row(selected: LayoutSystem) -> Element {
    let choices: Vec<(u8, &str)> = (0u8..)
        .zip(LayoutSystem::ALL.map(LayoutSystem::label))
        .collect();
    let selected = LayoutSystem::ALL
        .iter()
        .position(|&system| system == selected)
        .expect("every system is listed") as u8;
    choice_row(LAYOUT_SYSTEM_KEY, "Layout Settings", &choices, selected)
}

/// Frame Size (`HUD_EDIT_MODE_SETTING_UNIT_FRAME_FRAME_SIZE`, a percentage,
/// `EditModeSettingDisplayInfo.lua:357-362`), then this client's Text Size and Font.
fn unit_frame_rows(frame: UnitFrameSettings) -> Element {
    let percent = |slider: LayoutSlider, label, setting: Option<u16>| {
        let value = slider.range().clamp(setting.unwrap_or(100));
        slider_setting_cell(slider, label, f32::from(value), &format!("{value}%"))
    };
    let sizes = cell_row(
        "LayoutUnitFrameSizes",
        percent(LayoutSlider::FrameSize, "Frame Size", frame.frame_size),
        percent(LayoutSlider::TextSize, "Text Size", frame.text_size),
    );
    [sizes, font_row(frame.font)].into_iter().flatten().collect()
}

/// The two faces the client ships; an unset font is the authored Friz Quadrata.
fn font_row(font: Option<LayoutFont>) -> Element {
    let label = |font| match font {
        LayoutFont::FrizQuadrata => "Friz Quadrata",
        LayoutFont::ArialNarrow => "Arial Narrow",
    };
    let choices: Vec<(u8, &str)> = (0u8..).zip(LAYOUT_FONTS.map(label)).collect();
    let selected = LAYOUT_FONTS
        .iter()
        .position(|&choice| choice == font.unwrap_or(LayoutFont::FrizQuadrata))
        .expect("every font is listed") as u8;
    choice_row(LAYOUT_FONT_KEY, "Font", &choices, selected)
}

type SizeSlider = (LayoutSlider, &'static str, f32);

fn size_row(width: SizeSlider, height: SizeSlider) -> Element {
    let cell = |(slider, label, value): SizeSlider| {
        slider_setting_cell(slider, label, value, &format!("{value:.0}"))
    };
    cell_row("LayoutFrameSize", cell(width), cell(height))
}

fn slider_setting_cell(slider: LayoutSlider, label: &str, value: f32, text: &str) -> Element {
    let range = slider.range();
    slider_cell(
        &slider_key(SliderField::Layout(slider)),
        label,
        (value, (f32::from(range.min), f32::from(range.max))),
        text,
    )
}
