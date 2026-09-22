use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{GameFont, JustifyH};

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
) -> Element {
    let color = if !choice.enabled {
        COLOR_DISABLED
    } else if selected {
        COLOR_GOLD
    } else {
        COLOR_WHITE
    };
    let label = choice_label(choice, index);
    let width = if choice.swatch.is_some() { 92.0 } else { 144.0 };
    let first = choice
        .swatch
        .map(|color| {
            let vertex_color = tint(color);
            rsx! {
                texture { name: DynName(format!("{name}_Swatch")), width: 42.0, height: 10.0,
                    texture_atlas: "charactercreate-customize-palette", vertex_color,
                    pos_type: "absolute", left: 100.0, top: 5.0,
                }
            }
        })
        .unwrap_or_default();
    let second = choice.secondary_swatch.map(|color| {
        let vertex_color = tint(color);
        rsx! {
            texture { name: DynName(format!("{name}_SecondarySwatch")), width: 36.0, height: 8.0,
                texture_atlas: "charactercreate-customize-palette-half", vertex_color,
                pos_type: "absolute", left: 118.0, top: 7.0,
            }
        }
    }).unwrap_or_default();
    rsx! {
        fontstring { name: DynName(format!("{name}_Text")), width, height: CHOICE_HEIGHT,
            text: label, font: GameFont::FrizQuadrata, font_size: 12.0, font_color: color, justify_h: JustifyH::Left,
            pos_type: "absolute", left: 0.0, top: 0.0,
        }
        {first}
        {second}
    }
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
    let value = option.choices.iter().enumerate().find(|(_, choice)| choice.id == option.selected_choice_id)
        .map(|(index, choice)| choice_details(&format!("OptionValue_{}", option.id), choice, index, false))
        .unwrap_or_else(|| rsx! {
            fontstring { name: DynName(format!("OptionValue_{}_Text", option.id)), width: 144.0, height: 20.0,
                text: "Choose", font: GameFont::FrizQuadrata, font_size: 12.0, font_color: COLOR_WHITE,
            }
        });
    rsx! {
        {stepper(option, -1)}
        button { name: DynName(name), width: 150.0, height: OPTION_HEIGHT, disabled,
            onclick,
            button_atlas_up: atlas, button_atlas_pressed: "charactercreate-customize-dropdownbox-open",
            button_atlas_highlight: "charactercreate-customize-dropdownbox-hover",
            pos_type: "absolute", left: 36.5, top: 0.0,
            r#frame { name: DynName(format!("OptionValue_{}", option.id)), width: 144.0, height: 20.0,
                pos_type: "absolute", left: 3.0, top: 9.0,
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
            onclick,
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
    rows: usize,
) -> Element {
    let name = format!("OptionChoice_{}_{}", option.id, choice.id);
    let disabled = !option.enabled || !choice.enabled;
    let selected = choice.id == option.selected_choice_id;
    let marker_hidden = !selected;
    let x = (index / rows) as f32 * CHOICE_WIDTH;
    let y = (index % rows) as f32 * CHOICE_HEIGHT;
    let onclick =
        CharCreateAction::SelectOptionChoice(option.id, choice.id).when_enabled(!disabled);
    rsx! {
        button { name: DynName(name.clone()), width: CHOICE_WIDTH, height: CHOICE_HEIGHT, disabled,
            onclick,
            button_atlas_highlight: "charactercreate-customize-dropdown-linemouseover-middle",
            pos_type: "absolute", left: x, top: y,
            texture { name: DynName(format!("{name}_Selected")), width: CHOICE_WIDTH, height: CHOICE_HEIGHT,
                texture_atlas: "charactercreate-customize-dropdown-linemouseover-middle", hidden: marker_hidden,
                pos_type: "absolute", left: 0.0, top: 0.0,
            }
            r#frame { name: DynName(format!("{name}_Details")), width: 144.0, height: 20.0,
                pos_type: "absolute", left: 14.0, top: 0.0,
                {choice_details(&name, choice, index, selected)}
            }
        }
    }
}

fn border_piece(name: &str, atlas: &str, rect: [f32; 4]) -> Element {
    rsx! {
        texture { name: DynName(name.to_string()), width: rect[2], height: rect[3], texture_atlas: atlas,
            pos_type: "absolute", left: rect[0], top: rect[1],
        }
    }
}

fn dropdown_border(id: u32, width: f32, height: f32) -> Element {
    // Local NineSliceLayouts.lua:206-216: corners extend 30px horizontally and20px vertically.
    let prefix = format!("Dropdown_{id}");
    let parts = [
        (
            "Center",
            "charactercreatedropdown-nineslice-center",
            [0.0, 0.0, width, height],
        ),
        (
            "TL",
            "charactercreatedropdown-nineslice-cornertopleft",
            [-30.0, -20.0, 62.0, 52.0],
        ),
        (
            "TR",
            "charactercreatedropdown-nineslice-cornertopright",
            [width - 32.0, -20.0, 62.0, 52.0],
        ),
        (
            "BL",
            "charactercreatedropdown-nineslice-cornerbottomleft",
            [-30.0, height - 52.0, 62.0, 72.0],
        ),
        (
            "BR",
            "charactercreatedropdown-nineslice-cornerbottomright",
            [width - 32.0, height - 52.0, 62.0, 72.0],
        ),
        (
            "T",
            "_charactercreatedropdown-nineslice-edgetop",
            [32.0, -20.0, (width - 64.0).max(0.0), 52.0],
        ),
        (
            "B",
            "_charactercreatedropdown-nineslice-edgebottom",
            [32.0, height - 52.0, (width - 64.0).max(0.0), 72.0],
        ),
        (
            "L",
            "!charactercreatedropdown-nineslice-edgeleft",
            [-30.0, 32.0, 62.0, (height - 84.0).max(0.0)],
        ),
        (
            "R",
            "!charactercreatedropdown-nineslice-edgeright",
            [width - 32.0, 32.0, 62.0, (height - 84.0).max(0.0)],
        ),
    ];
    parts
        .into_iter()
        .flat_map(|(part, atlas, rect)| border_piece(&format!("{prefix}_{part}"), atlas, rect))
        .collect()
}

pub(super) fn dropdown_panel(
    option: &CustomizationOptionUi,
    viewport: [u32; 2],
    anchor_bottom: f32,
) -> Element {
    if !option.enabled || option.ui_type != 0 || option.choices.is_empty() {
        return Element::default();
    }
    let layout = popup_layout(viewport, option.choices.len(), anchor_bottom);
    let choices: Element = option
        .choices
        .iter()
        .enumerate()
        .flat_map(|(index, choice)| dropdown_choice(option, choice, index, layout.rows))
        .collect();
    rsx! {
        r#frame { name: DynName(format!("Dropdown_{}", option.id)), width: layout.width, height: layout.height,
            strata: FrameStrata::Dialog, mouse_enabled: true,
            pos_type: "absolute", anchor: "screen", left: layout.x, top: layout.y,
            {dropdown_border(option.id, layout.width, layout.height)}
            {choices}
        }
    }
}
