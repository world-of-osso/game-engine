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
        LayoutSystem::PartyFrames => party_rows(layout),
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
        toggle_row(
            "layout_show_micro_menu",
            "Show Micro Menu",
            settings.show_micro_menu.unwrap_or(false),
        ),
        system_row(layout.system),
        system_settings(system_rows),
        options_menu_sections::action_button_row(
            "reset_layout_settings",
            "Reset to Preset",
            "Clears this layout's settings",
            "Reset",
            ACTION_RESET_LAYOUT_SETTINGS,
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// The shown system's controls in one container, as Retail's dialog keeps them in its
/// `Settings` frame (`EditModeDialogs.xml:286`). It also keeps the controls under the selector
/// when the system changes: the Screen diff appends new frames after the siblings it keeps
/// (ui-toolkit-core `widget_def_diff.rs` `diff_roots`), so the rows must not be siblings of
/// the rows around them.
fn system_settings(rows: Element) -> Element {
    rsx! {
        r#frame {
            name: "LayoutSystemSettings",
            width: {OPTIONS_ROW_W},
            height: "auto",
            layout: "flex-column",
            gap: 14.0,
            {rows}
        }
    }
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
    [sizes, font_row(frame.font)]
        .into_iter()
        .flatten()
        .collect()
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

fn party_rows(layout: &LayoutOptionsView) -> Element {
    use crate::options_menu_component::PartyDropdown;
    use crate::options_menu_data::PartyToggle;
    use game_engine_core::ui_layout_data::{PartyAuraOrganization, PartySort};
    let settings = layout.settings;
    let party = settings.party;
    let (width, height) = party.size();
    let mut rows = Element::new();
    for (toggle, label) in [
        (PartyToggle::Compact, "Use Raid-Style Party Frames"),
        (PartyToggle::Background, "Show Party Frame Background"),
        (PartyToggle::Horizontal, "Use Horizontal Layout"),
        (PartyToggle::Border, "Display Border"),
        (PartyToggle::Pets, "Show Pets"),
    ] {
        let action = toggle_action(toggle.key());
        rows.extend(rsx! { r#frame {
            name: {DynName(format!("PartySetting{}", toggle.key()))},
            width: OPTIONS_ROW_W, height: 32.0,
            {row_label(&format!("PartyLabel{}", toggle.key()), label)}
            {super::super::bank_art::checkbox(toggle.key(), "", toggle.value(&settings), &action, (620.0, 4.0))}
        } });
    }
    rows.extend(party_size_row(
        (LayoutSlider::PartyWidth, "Frame Width", width),
        (LayoutSlider::PartyHeight, "Frame Height", height),
    ));
    rows.extend(party_size_row(
        (
            LayoutSlider::PartySize,
            "Frame Size (%)",
            party.scale() * 100.0,
        ),
        (
            LayoutSlider::PartyOpacity,
            "Opacity (%)",
            party.alpha() * 100.0,
        ),
    ));
    rows.extend(party_dropdown(
        "party_sort",
        "Sort By",
        &["Role", "Group", "Alphabetical"],
        match party.sort.unwrap_or_default() {
            PartySort::Role => 0,
            PartySort::Group => 1,
            PartySort::Alphabetical => 2,
        },
        layout.party_dropdown == Some(PartyDropdown::Sort),
    ));
    rows.extend(party_dropdown(
        "party_aura",
        "Aura Organization",
        &[
            "Legacy",
            "Buffs Top / Debuffs Bottom",
            "Buffs Right / Debuffs Left",
        ],
        match party.aura_organization.unwrap_or_default() {
            PartyAuraOrganization::Legacy => 0,
            PartyAuraOrganization::BuffsTop => 1,
            PartyAuraOrganization::BuffsRight => 2,
        },
        layout.party_dropdown == Some(PartyDropdown::Aura),
    ));
    for (slider, label, value) in [
        (
            LayoutSlider::PartyDebuff,
            "Debuff Icon Size (%)",
            party.debuff_size.unwrap_or(100),
        ),
        (
            LayoutSlider::PartyBuff,
            "Buff Icon Size (%)",
            party.buff_size.unwrap_or(100),
        ),
        (
            LayoutSlider::PartyDefensive,
            "Big Defensive Icon Size (%)",
            party.defensive_size.unwrap_or(75),
        ),
    ] {
        let range = slider.range();
        rows.extend(slider_row(
            &slider_key(SliderField::Layout(slider)),
            label,
            f32::from(range.clamp(value)),
            f32::from(range.min),
            f32::from(range.max),
        ));
    }
    rows
}

fn party_dropdown(key: &str, label: &str, labels: &[&str], selected: usize, open: bool) -> Element {
    let action = toggle_action(&format!("{key}_open"));
    let items: Element = if open {
        labels.iter().enumerate().flat_map(|(index, text)| {
            let action = toggle_action(&format!("{key}:{index}"));
            rsx! { button {
                name: {DynName(format!("PartyChoice{key}{index}"))},
                width: CHOICE_ROW_W, height: 28.0, text: {text.to_string()},
                onclick: {action.as_str()}, pos_type: "absolute", left: 0.0, top: {28.0 + index as f32 * 28.0},
            } }
        }).collect()
    } else {
        Element::new()
    };
    rsx! { r#frame {
        name: {DynName(format!("PartyDropdown{key}"))}, width: OPTIONS_ROW_W,
        height: {if open { 28.0 * (labels.len() + 1) as f32 } else { 32.0 }},
        {row_label(&format!("PartyDropdownLabel{key}"), label)}
        r#frame {
            width: CHOICE_ROW_W, height: "auto", pos_type: "absolute", left: 258.0, top: 0.0,
            button { name: {DynName(format!("PartyDropdownButton{key}"))}, width: CHOICE_ROW_W, height: 28.0,
                text: {format!("{}  ▾", labels[selected])}, onclick: {action.as_str()}, }
            {items}
        }
    } }
}

type SizeSlider = (LayoutSlider, &'static str, f32);

fn party_size_row(width: SizeSlider, height: SizeSlider) -> Element {
    let key = format!("PartySizeRow{}", slider_key(SliderField::Layout(width.0)));
    let cell = |(slider, label, value): SizeSlider| {
        slider_setting_cell(slider, label, value, &format!("{value:.0}"))
    };
    cell_row(&key, cell(width), cell(height))
}

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
