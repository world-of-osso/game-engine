use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::ui::frame::NineSlice;
use crate::ui::registry::FrameRegistry;
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{GameFont, JustifyH};
use crate::ui::widgets::texture::TextureSource;
use ui_toolkit::text_measure::measure_text;

use super::reference_layout::*;
use super::{
    COLOR_DISABLED, COLOR_GOLD, COLOR_WHITE, CharCreateAction, CustomizationChoiceUi,
    CustomizationOptionUi, DynName,
};

fn choice_label(choice: &CustomizationChoiceUi, index: usize) -> String {
    if choice.label.is_empty() {
        (index + 1).to_string()
    } else {
        choice.label.clone()
    }
}

fn fit_closed_label(label: &str) -> (String, f32) {
    const MAX_WIDTH: f32 = 126.0;
    let width = |text: &str| {
        measure_text(text, GameFont::FrizQuadrata, 12.0)
            .expect("closed customization font must be available")
            .0
    };
    let full_width = width(label);
    if full_width <= MAX_WIDTH {
        return (label.to_string(), full_width);
    }
    let mut characters: Vec<char> = label.chars().collect();
    loop {
        characters.pop();
        let shortened = format!("{}…", characters.iter().collect::<String>());
        let shortened_width = width(&shortened);
        if shortened_width <= MAX_WIDTH {
            return (shortened, shortened_width);
        }
    }
}

fn tint(color: [u8; 3]) -> String {
    format!(
        "{},{},{},1",
        color[0] as f32 / 255.0,
        color[1] as f32 / 255.0,
        color[2] as f32 / 255.0
    )
}

fn choice_details(
    name: &str,
    choice: &CustomizationChoiceUi,
    index: usize,
    selected: bool,
    selectable: bool,
    details_width: f32,
    label_override: Option<&str>,
) -> Element {
    let color = if !choice.enabled {
        COLOR_DISABLED
    } else if selected {
        COLOR_GOLD
    } else {
        COLOR_WHITE
    };
    let has_colors = choice.swatch.is_some() || choice.secondary_swatch.is_some();
    let label = label_override.map(str::to_owned).unwrap_or_else(|| {
        if selectable && has_colors {
            (index + 1).to_string()
        } else {
            choice_label(choice, index)
        }
    });
    let hide_label = !selectable && has_colors;
    let swatches = choice_swatches(name, choice, selected, selectable);
    rsx! {
        fontstring { name: DynName(format!("{name}_Text")), width: details_width, height: CHOICE_HEIGHT,
            text: label, hidden: hide_label,
            font: GameFont::FrizQuadrata, font_size: 12.0, font_color: color, justify_h: JustifyH::Left,
            pos_type: "absolute", left: 0.0, top: 0.0,
        }
        {swatches}
    }
}

fn primary_swatch(name: &str, color: [u8; 3], dual_color: bool, x: f32) -> Element {
    let vertex_color = tint(color);
    let atlas = if dual_color {
        "charactercreate-customize-palette-half"
    } else {
        "charactercreate-customize-palette"
    };
    rsx! {
        texture { name: DynName(format!("{name}_Swatch")), width: 42.0, height: 10.0,
            texture_atlas: atlas, vertex_color,
            pos_type: "absolute", left: x, top: 5.0,
        }
    }
}

fn swatch_selection(name: &str, visible: bool, x: f32) -> Element {
    if !visible {
        return Element::default();
    }
    let x = x - 4.0;
    rsx! {
        texture { name: DynName(format!("{name}_SelectedSwatch")), width: 51.0, height: 20.0,
            texture_atlas: "charactercreate-customize-palette-selected",
            pos_type: "absolute", left: x, top: 0.0,
        }
    }
}

fn choice_swatches(
    name: &str,
    choice: &CustomizationChoiceUi,
    selected: bool,
    selectable: bool,
) -> Element {
    let Some(first_color) = choice.swatch.or(choice.secondary_swatch) else {
        return Element::default();
    };
    let second_color = choice
        .swatch
        .zip(choice.secondary_swatch)
        .map(|(_, color)| color);
    let x = if selectable { 25.0 } else { 0.0 };
    let first = primary_swatch(name, first_color, second_color.is_some(), x);
    let second_x = x + 18.0;
    let second = second_color.map(|color| {
        let vertex_color = tint(color);
        rsx! {
            texture { name: DynName(format!("{name}_SecondarySwatch")), width: 36.0, height: 8.0,
                texture_atlas: "charactercreate-customize-palette", vertex_color,
                pos_type: "absolute", left: second_x, top: 7.0,
            }
        }
    }).unwrap_or_default();
    let selection = swatch_selection(name, selected && choice.enabled, x);
    rsx! { {first} {second} {selection} }
}

fn option_label(option: &CustomizationOptionUi, checkbox: bool) -> Element {
    let x = if checkbox { -68.0 } else { -275.0 };
    let color = if option.enabled {
        COLOR_WHITE
    } else {
        COLOR_DISABLED
    };
    rsx! {
        fontstring { name: DynName(format!("OptionLabel_{}", option.id)), width: 225.0, height: OPTION_HEIGHT,
            text: option.label.clone(), font: GameFont::FrizQuadrata, font_size: 15.0,
            font_color: color, justify_h: JustifyH::Right,
            pos_type: "absolute", left: x, top: 0.0,
        }
    }
}

fn stepper(option: &CustomizationOptionUi, delta: i8) -> Element {
    let increment = delta > 0;
    let name = format!(
        "Option{}_{}",
        if increment { "Inc" } else { "Dec" },
        option.id
    );
    let atlas = if increment {
        "common-dropdown-icon-next"
    } else {
        "common-dropdown-icon-back"
    };
    let disabled_atlas = if increment {
        "common-dropdown-icon-next-disabled"
    } else {
        "common-dropdown-icon-back-disabled"
    };
    let disabled = !option.enabled
        || option
            .choices
            .iter()
            .filter(|choice| choice.enabled)
            .count()
            < 2;
    let x = if increment { OPTION_WIDTH - 38.0 } else { 0.0 };
    let onclick = CharCreateAction::AdjustOption(option.id, delta).when_enabled(!disabled);
    rsx! {
        button { name: DynName(name), width: 38.0, height: OPTION_HEIGHT, disabled,
            onclick,
            button_atlas_up: atlas, button_atlas_pressed: atlas,
            button_atlas_highlight: atlas, button_atlas_disabled: disabled_atlas,
            pos_type: "absolute", left: x, top: 0.0,
        }
    }
}

fn dropdown_control(option: &CustomizationOptionUi, open: bool) -> Element {
    let name = format!("OptionToggle_{}", option.id);
    let atlas = if open {
        "charactercreate-customize-dropdownbox-open"
    } else {
        "charactercreate-customize-dropdownbox"
    };
    let disabled = !option.enabled || option.choices.is_empty();
    let onclick = CharCreateAction::ToggleOption(option.id).when_enabled(!disabled);
    let selected = option
        .choices
        .iter()
        .enumerate()
        .find(|(_, choice)| choice.id == option.selected_choice_id);
    let label = selected
        .map(|(index, choice)| choice_label(choice, index))
        .unwrap_or_else(|| "Choose".to_string());
    let (displayed_label, text_width) = fit_closed_label(&label);
    let value_width = match selected {
        Some((_, choice)) if choice.swatch.is_some() && choice.secondary_swatch.is_some() => 54.0,
        Some((_, choice)) if choice.swatch.is_some() || choice.secondary_swatch.is_some() => 42.0,
        _ => text_width,
    };
    let value_left = (150.0 - value_width) / 2.0;
    let value = selected
        .map(|(index, choice)| choice_details(&format!("OptionValue_{}", option.id), choice, index, false, false, value_width, Some(&displayed_label)))
        .unwrap_or_else(|| rsx! {
            fontstring { name: DynName(format!("OptionValue_{}_Text", option.id)), width: value_width, height: 20.0,
                text: displayed_label, font: GameFont::FrizQuadrata, font_size: 12.0, font_color: COLOR_WHITE,
            }
        });
    rsx! {
        {stepper(option, -1)}
        button { name: DynName(name), width: 150.0, height: OPTION_HEIGHT, disabled,
            onclick,
            button_atlas_up: atlas, button_atlas_pressed: "charactercreate-customize-dropdownbox-open",
            button_atlas_highlight: "charactercreate-customize-dropdownbox-hover",
            pos_type: "absolute", left: 36.5, top: 0.0,
            r#frame { name: DynName(format!("OptionValue_{}", option.id)), width: value_width, height: 20.0,
                pos_type: "absolute", left: value_left, top: 9.0,
                {value}
            }
        }
        {stepper(option, 1)}
    }
}

fn checkbox_control(option: &CustomizationOptionUi) -> Element {
    let checked = option.selected_choice_id == option.choices[1].id;
    let next = &option.choices[usize::from(!checked)];
    let disabled = !option.enabled || !next.enabled;
    let checked_hidden = !checked;
    let check_fdid = if disabled { 130_750u32 } else { 130_751u32 };
    let x = OPTION_WIDTH - 32.0;
    let onclick = CharCreateAction::SelectOptionChoice(option.id, next.id).when_enabled(!disabled);
    rsx! {
        button { name: DynName(format!("OptionCheck_{}", option.id)), width: 32.0, height: 32.0, disabled,
            onclick, button_default_skin: false,
            pos_type: "absolute", left: x, top: 3.0,
            texture { name: DynName(format!("OptionCheck_{}_Background", option.id)), width: 32.0, height: 32.0,
                texture_fdid: 130_755u32,
            }
            texture { name: DynName(format!("OptionCheck_{}_Mark", option.id)), width: 32.0, height: 32.0,
                texture_fdid: check_fdid, hidden: checked_hidden,
                pos_type: "absolute", left: 0.0, top: 0.0,
            }
        }
    }
}

fn unavailable(option: &CustomizationOptionUi, reason: &str) -> Element {
    rsx! {
        fontstring { name: DynName(format!("OptionReason_{}", option.id)), width: OPTION_WIDTH, height: OPTION_HEIGHT,
            text: reason, font: GameFont::FrizQuadrata, font_size: 11.0, font_color: COLOR_DISABLED,
            justify_h: JustifyH::Left,
        }
    }
}

pub(super) fn customization_row(option: &CustomizationOptionUi, open: bool, y: f32) -> Element {
    let control = match (option.enabled, option.ui_type, option.choices.len()) {
        (false, _, _) => unavailable(
            option,
            option.disabled_reason.as_deref().unwrap_or("Unavailable"),
        ),
        (true, 0, _) => dropdown_control(option, open),
        (true, 1, 2) => checkbox_control(option),
        (true, 1, _) => unavailable(option, "Checkbox requires two choices"),
        _ => unavailable(option, "This control type is not supported"),
    };
    rsx! {
        r#frame { name: DynName(format!("Option_{}", option.id)), width: OPTION_WIDTH, height: OPTION_HEIGHT,
            pos_type: "absolute", left: 0.0, top: y,
            {option_label(option, option.ui_type == 1 && option.enabled)}
            {control}
        }
    }
}

fn dropdown_choice(
    option: &CustomizationOptionUi,
    choice: &CustomizationChoiceUi,
    index: usize,
    layout: &PopupLayout,
) -> Element {
    let name = format!("OptionChoice_{}_{}", option.id, choice.id);
    let disabled = !option.enabled || !choice.enabled;
    let selected = choice.id == option.selected_choice_id;
    let x = POPUP_INSET_LEFT + (index / layout.rows) as f32 * layout.choice_width;
    let y = POPUP_INSET_TOP + (index % layout.rows) as f32 * CHOICE_HEIGHT;
    let details_width = if layout.columns == 1 {
        SINGLE_CHOICE_WIDTH - CHOICE_WIDTH_PADDING
    } else {
        multiple_column_details_width(choice)
    };
    let choice_width = layout.choice_width;
    let onclick =
        CharCreateAction::SelectOptionChoice(option.id, choice.id).when_enabled(!disabled);
    rsx! {
        button { name: DynName(name.clone()), width: choice_width, height: CHOICE_HEIGHT, disabled,
            onclick, button_default_skin: false, button_highlight_alpha: "0.15",
            button_atlas_highlight: "common-dropdown-customize-mouseover",
            pos_type: "absolute", left: x, top: y,
            r#frame { name: DynName(format!("{name}_Details")), width: details_width, height: 20.0,
                pos_type: "absolute", left: 14.0, top: 0.0,
                {choice_details(&name, choice, index, selected, true, details_width, None)}
            }
        }
    }
}

pub(super) fn apply_dropdown_background_style(registry: &mut FrameRegistry, open: Option<u32>) {
    let Some(id) = open else { return };
    let Some(frame_id) = registry.get_by_name(&format!("Dropdown_{id}_Background")) else {
        return;
    };
    let slice = NineSlice {
        edge_size: 23.0,
        edge_sizes: Some([23.0, 18.0, 23.0, 28.0]),
        uv_edge_sizes: Some([23.0, 18.0, 23.0, 28.0]),
        bg_color: [1.0; 4],
        border_color: [1.0; 4],
        texture: Some(TextureSource::Atlas("common-dropdown-c-bg".into())),
        ..Default::default()
    };
    if registry
        .get(frame_id)
        .is_some_and(|frame| frame.nine_slice.as_ref() == Some(&slice))
    {
        return;
    }
    if let Some(frame) = registry.get_mut(frame_id) {
        frame.nine_slice = Some(slice);
    }
}

fn dropdown_background(id: u32, width: f32, height: f32) -> Element {
    // MenuStyle2Mixin insets the atlas from the menu's content bounds.
    let name = format!("Dropdown_{id}_Background");
    let width = width + 34.0;
    let height = height + 34.0;
    rsx! {
        texture { name: DynName(name), width, height,
            texture_atlas: "common-dropdown-c-bg",
            pos_type: "absolute", left: -17.0, top: -12.0,
        }
    }
}

fn multiple_column_details_width(choice: &CustomizationChoiceUi) -> f32 {
    if choice.swatch.is_some() || choice.secondary_swatch.is_some() {
        MULTI_COLOR_DETAILS_WIDTH
    } else if !choice.label.is_empty() {
        MULTI_TEXT_DETAILS_WIDTH
    } else {
        MULTI_NUMBER_DETAILS_WIDTH
    }
}

pub(super) fn dropdown_panel(
    option: &CustomizationOptionUi,
    viewport: [u32; 2],
    anchor_bottom: f32,
) -> Element {
    if !option.enabled || option.ui_type != 0 || option.choices.is_empty() {
        return Element::default();
    }
    let multi_choice_width = option
        .choices
        .iter()
        .map(multiple_column_details_width)
        .fold(0.0_f32, f32::max)
        + CHOICE_WIDTH_PADDING;
    let layout = popup_layout(
        viewport,
        option.choices.len(),
        anchor_bottom,
        multi_choice_width,
    );
    let choices: Element = option
        .choices
        .iter()
        .enumerate()
        .flat_map(|(index, choice)| dropdown_choice(option, choice, index, &layout))
        .collect();
    rsx! {
        r#frame { name: DynName(format!("Dropdown_{}", option.id)), width: layout.width, height: layout.height,
            strata: FrameStrata::Dialog, mouse_enabled: true,
            pos_type: "absolute", anchor: "screen", left: layout.x, top: layout.y,
            {dropdown_background(option.id, layout.width, layout.height)}
            {choices}
        }
    }
}
