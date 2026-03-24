use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont, JustifyH};

use super::{
    AppearanceField, BACK_BUTTON, BUTTON_ATLAS_DISABLED, BUTTON_ATLAS_HIGHLIGHT,
    BUTTON_ATLAS_PRESSED, BUTTON_ATLAS_UP, COLOR_DISABLED, COLOR_GOLD, COLOR_SELECTED,
    COLOR_SUBTITLE, COLOR_WHITE, CREATE_BUTTON, CREATE_NAME_INPUT, CharCreateAction,
    CharCreateMode, DynName, ERROR_TEXT, NEXT_BUTTON, RANDOMIZE_BUTTON, SEX_TOGGLE_BUTTON,
};

pub(super) fn dyn_name(s: String) -> DynName {
    DynName(s)
}

const APPEARANCE_STEPPER_WIDTH: f32 = 26.0;
const APPEARANCE_DEC_RIGHT_INSET: f32 = 120.0;
const APPEARANCE_INC_RIGHT_INSET: f32 = 10.0;
const APPEARANCE_SWATCH_PREVIEW_WIDTH: f32 = 84.0;
const APPEARANCE_SWATCH_PREVIEW_HEIGHT: f32 = 20.0;
const APPEARANCE_SWATCH_PREVIEW_AREA_HEIGHT: f32 = 40.0;
const APPEARANCE_SWATCH_PREVIEW_SELECTION_WIDTH: f32 = 102.0;
const APPEARANCE_SWATCH_PREVIEW_SELECTION_HEIGHT: f32 = 40.0;
const APPEARANCE_SWATCH_DROPDOWN_WIDTH: f32 = 40.0;
const APPEARANCE_SWATCH_DROPDOWN_HEIGHT: f32 = 20.0;
const APPEARANCE_SWATCH_DROPDOWN_CHOICE_WIDTH: f32 = 44.0;
const APPEARANCE_SWATCH_DROPDOWN_CHOICE_HEIGHT: f32 = 28.0;
const APPEARANCE_SWATCH_DROPDOWN_SELECTION_WIDTH: f32 = 48.0;
const APPEARANCE_SWATCH_DROPDOWN_SELECTION_HEIGHT: f32 = 28.0;
const APPEARANCE_NUMBER_DROPDOWN_CHOICE_WIDTH: f32 = 28.0;
const APPEARANCE_NUMBER_DROPDOWN_CHOICE_HEIGHT: f32 = 22.0;
const APPEARANCE_DROPDOWN_WIDTH: f32 = 282.0;
const APPEARANCE_DROPDOWN_GAP: f32 = 2.0;
const APPEARANCE_DROPDOWN_PADDING: f32 = 4.0;
const SWATCH_SELECTION_PREVIEW_OFFSET_X: f32 = -8.0;
const SWATCH_SELECTION_DROPDOWN_OFFSET_X: f32 = -4.0;

fn right_inset_x(inset: f32) -> String {
    format!("-{inset}")
}

fn x_offset(x: f32) -> String {
    format!("{x}")
}

fn swatch_gap_center_x() -> String {
    let inset =
        (APPEARANCE_DEC_RIGHT_INSET + APPEARANCE_INC_RIGHT_INSET + APPEARANCE_STEPPER_WIDTH) * 0.5;
    right_inset_x(inset)
}

// --- Race grid ---

fn race_button_style(is_selected: bool) -> (&'static str, &'static str) {
    if is_selected {
        ("2px solid 1.0,0.82,0.0,1.0", "0.2,0.16,0.08,0.9")
    } else {
        ("1px solid 0.45,0.38,0.22,0.6", "0.1,0.08,0.05,0.7")
    }
}

fn race_top_widget(race_id: u8, short_name: &str, icon_file: &str, color: FontColor) -> Element {
    if !icon_file.is_empty() {
        rsx! {
            texture {
                name: dyn_name(format!("Race_{race_id}_Icon")),
                width: 36.0,
                height: 36.0,
                texture_file: icon_file,
                anchor {
                    point: AnchorPoint::Top,
                    relative_point: AnchorPoint::Top,
                    y: "-4",
                }
            }
        }
    } else {
        rsx! {
            fontstring {
                name: dyn_name(format!("Race_{race_id}_Short")),
                width: 44.0,
                height: 24.0,
                text: short_name,
                font: GameFont::FrizQuadrata,
                font_size: 16.0,
                font_color: color,
                anchor {
                    point: AnchorPoint::Top,
                    relative_point: AnchorPoint::Top,
                    y: "-4",
                }
            }
        }
    }
}

fn race_name_label(race_id: u8, name: &str, color: FontColor) -> Element {
    rsx! {
        fontstring {
            name: dyn_name(format!("Race_{race_id}_Label")),
            width: 52.0,
            height: 20.0,
            text: name,
            font: GameFont::FrizQuadrata,
            font_size: 8.0,
            font_color: color,
            word_wrap: true,
            max_lines: 2,
            anchor {
                point: AnchorPoint::Bottom,
                relative_point: AnchorPoint::Bottom,
                y: "4",
            }
        }
    }
}

pub(super) fn race_buttons_for_faction(
    faction: crate::char_create_data::Faction,
    selected_race: u8,
) -> Element {
    use crate::char_create_data::RACES;
    RACES
        .iter()
        .filter(|r| r.faction == faction)
        .flat_map(|r| {
            race_button(
                r.id,
                r.short_name,
                r.name,
                r.icon_file,
                r.id == selected_race,
            )
        })
        .collect()
}

pub(super) fn race_button(
    race_id: u8,
    short_name: &str,
    name: &str,
    icon_file: &str,
    is_selected: bool,
) -> Element {
    let color = if is_selected {
        COLOR_SELECTED
    } else {
        COLOR_SUBTITLE
    };
    let (border, bg) = race_button_style(is_selected);
    let top = race_top_widget(race_id, short_name, icon_file, color);
    let label = race_name_label(race_id, name, color);
    rsx! {
        r#frame {
            name: dyn_name(format!("Race_{race_id}")),
            width: 52.0,
            height: 60.0,
            onclick: CharCreateAction::SelectRace(race_id),
            border,
            background_color: bg,
            {top}
            {label}
        }
    }
}

pub(super) fn faction_column(
    label: &str,
    col_name: &str,
    x_offset: &str,
    races: Element,
) -> Element {
    rsx! {
        fontstring {
            name: dyn_name(format!("{col_name}Label")),
            width: 140.0,
            height: 24.0,
            text: label,
            font: GameFont::FrizQuadrata,
            font_size: 16.0,
            font_color: COLOR_GOLD,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: x_offset,
                y: "-4",
            }
        }
        r#frame {
            name: dyn_name(format!("{col_name}Races")),
            width: 150.0,
            height: 400.0,
            layout: "flex-row-wrap",
            gap: 6.0,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: x_offset,
                y: "-30",
            }
            {races}
        }
    }
}

// --- Class grid ---

pub(super) fn class_button_style(
    is_selected: bool,
    available: bool,
) -> (FontColor, &'static str, &'static str) {
    let color = if !available {
        COLOR_DISABLED
    } else if is_selected {
        COLOR_SELECTED
    } else {
        COLOR_SUBTITLE
    };
    let border = if is_selected && available {
        "2px solid 1.0,0.82,0.0,1.0"
    } else {
        "1px solid 0.45,0.38,0.22,0.4"
    };
    let bg = if is_selected && available {
        "0.2,0.16,0.08,0.9"
    } else {
        "0.1,0.08,0.05,0.7"
    };
    (color, border, bg)
}

fn class_icon_widget(class_id: u8, icon: &str, alpha: &str) -> Element {
    rsx! {
        texture {
            name: dyn_name(format!("Class_{class_id}_Icon")),
            width: 36.0,
            height: 36.0,
            texture_file: icon,
            alpha,
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::Top,
                y: "-4",
            }
        }
    }
}

fn class_name_label(class_id: u8, name: &str, color: FontColor) -> Element {
    rsx! {
        fontstring {
            name: dyn_name(format!("Class_{class_id}_Label")),
            width: 52.0,
            height: 20.0,
            text: name,
            font: GameFont::FrizQuadrata,
            font_size: 8.0,
            font_color: color,
            word_wrap: true,
            max_lines: 2,
            anchor {
                point: AnchorPoint::Bottom,
                relative_point: AnchorPoint::Bottom,
                y: "2",
            }
        }
    }
}

pub(super) fn class_button(
    class_id: u8,
    name: &str,
    icon: &str,
    is_selected: bool,
    available: bool,
) -> Element {
    let (color, border, bg) = class_button_style(is_selected, available);
    let onclick = if available {
        CharCreateAction::SelectClass(class_id).to_string()
    } else {
        String::new()
    };
    let alpha = if available { "1.0" } else { "0.3" };
    rsx! {
        r#frame {
            name: dyn_name(format!("Class_{class_id}")),
            width: 52.0,
            height: 60.0,
            onclick,
            border,
            background_color: bg,
            {class_icon_widget(class_id, icon, alpha)}
            {class_name_label(class_id, name, color)}
        }
    }
}

// --- Appearance row helpers ---

fn stepper_dec_button(field: AppearanceField) -> Element {
    let x = right_inset_x(APPEARANCE_DEC_RIGHT_INSET);
    rsx! {
        r#frame {
            name: dyn_name(format!("AppDec_{}", field.as_str())),
            width: APPEARANCE_STEPPER_WIDTH,
            height: 25.0,
            onclick: CharCreateAction::AppearanceDec(field),
            anchor {
                point: AnchorPoint::Right,
                relative_point: AnchorPoint::Right,
                x,
            }
            texture {
                name: dyn_name(format!("AppDecBg_{}", field.as_str())),
                width: APPEARANCE_STEPPER_WIDTH,
                height: 25.0,
                texture_atlas: "common-dropdown-c-button",
            }
            texture {
                name: dyn_name(format!("AppDecIcon_{}", field.as_str())),
                width: 17.0,
                height: 17.0,
                texture_atlas: "common-dropdown-icon-back",
                anchor {
                    point: AnchorPoint::Center,
                    relative_point: AnchorPoint::Center,
                }
            }
        }
    }
}

fn stepper_inc_button(field: AppearanceField) -> Element {
    let x = right_inset_x(APPEARANCE_INC_RIGHT_INSET);
    rsx! {
        r#frame {
            name: dyn_name(format!("AppInc_{}", field.as_str())),
            width: APPEARANCE_STEPPER_WIDTH,
            height: 25.0,
            onclick: CharCreateAction::AppearanceInc(field),
            anchor {
                point: AnchorPoint::Right,
                relative_point: AnchorPoint::Right,
                x,
            }
            texture {
                name: dyn_name(format!("AppIncBg_{}", field.as_str())),
                width: APPEARANCE_STEPPER_WIDTH,
                height: 25.0,
                texture_atlas: "common-dropdown-c-button",
            }
            texture {
                name: dyn_name(format!("AppIncIcon_{}", field.as_str())),
                width: 17.0,
                height: 17.0,
                texture_atlas: "common-dropdown-icon-next",
                anchor {
                    point: AnchorPoint::Center,
                    relative_point: AnchorPoint::Center,
                }
            }
        }
    }
}

fn appearance_row_label(field: AppearanceField, label: &str) -> Element {
    rsx! {
        fontstring {
            name: dyn_name(format!("AppLabel_{}", field.as_str())),
            width: 120.0,
            height: 24.0,
            text: label,
            font: GameFont::FrizQuadrata,
            font_size: 13.0,
            font_color: COLOR_SUBTITLE,
            justify_h: JustifyH::Left,
            anchor {
                point: AnchorPoint::Left,
                relative_point: AnchorPoint::Left,
                x: "10",
            }
        }
    }
}

fn appearance_row_value(field: AppearanceField, value: u8, label: &str) -> Element {
    let val_text = if label.is_empty() {
        format!("{}", value + 1)
    } else {
        label.to_string()
    };
    let x = swatch_gap_center_x();
    rsx! {
        fontstring {
            name: dyn_name(format!("AppVal_{}", field.as_str())),
            width: APPEARANCE_SWATCH_PREVIEW_WIDTH,
            height: 24.0,
            text: val_text,
            font: GameFont::FrizQuadrata,
            font_size: 12.0,
            font_color: COLOR_WHITE,
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Right,
                x,
            }
        }
    }
}

fn rgb_to_vertex_color(color: [u8; 3]) -> String {
    format!(
        "{},{},{},1.0",
        color[0] as f32 / 255.0,
        color[1] as f32 / 255.0,
        color[2] as f32 / 255.0
    )
}

fn swatch_texture(field: AppearanceField, color: [u8; 3]) -> Element {
    let vc = rgb_to_vertex_color(color);
    let x = swatch_gap_center_x();
    let selection_x = x_offset(SWATCH_SELECTION_PREVIEW_OFFSET_X);
    rsx! {
        r#frame {
            name: dyn_name(format!("AppSwatchArea_{}", field.as_str())),
            width: APPEARANCE_SWATCH_PREVIEW_WIDTH,
            height: APPEARANCE_SWATCH_PREVIEW_AREA_HEIGHT,
            onclick: CharCreateAction::ToggleDropdown(field),
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Right,
                x,
            }
            texture {
                name: dyn_name(format!("AppSwatch_{}", field.as_str())),
                width: APPEARANCE_SWATCH_PREVIEW_WIDTH,
                height: APPEARANCE_SWATCH_PREVIEW_HEIGHT,
                texture_atlas: "charactercreate-customize-palette",
                vertex_color: vc,
                anchor {
                    point: AnchorPoint::Center,
                    relative_point: AnchorPoint::Center,
                }
            }
            texture {
                name: dyn_name(format!("AppSwatchSel_{}", field.as_str())),
                width: APPEARANCE_SWATCH_PREVIEW_SELECTION_WIDTH,
                height: APPEARANCE_SWATCH_PREVIEW_SELECTION_HEIGHT,
                texture_atlas: "charactercreate-customize-palette-selected",
                anchor {
                    point: AnchorPoint::Center,
                    relative_point: AnchorPoint::Center,
                    x: selection_x,
                }
            }
        }
    }
}

fn dropdown_color_choice(
    field: AppearanceField,
    i: usize,
    color: [u8; 3],
    selected: bool,
) -> Element {
    let vc = rgb_to_vertex_color(color);
    let sel_hidden = !selected;
    let idx = i as u8;
    let selection_x = x_offset(SWATCH_SELECTION_DROPDOWN_OFFSET_X);
    rsx! {
        r#frame {
            name: dyn_name(format!("DropChoice_{}_{i}", field.as_str())),
            width: APPEARANCE_SWATCH_DROPDOWN_CHOICE_WIDTH,
            height: APPEARANCE_SWATCH_DROPDOWN_CHOICE_HEIGHT,
            onclick: CharCreateAction::SelectChoice(field, idx),
            texture {
                name: dyn_name(format!("DropSwatch_{}_{i}", field.as_str())),
                width: APPEARANCE_SWATCH_DROPDOWN_WIDTH,
                height: APPEARANCE_SWATCH_DROPDOWN_HEIGHT,
                texture_atlas: "charactercreate-customize-palette",
                vertex_color: vc,
                anchor {
                    point: AnchorPoint::Center,
                    relative_point: AnchorPoint::Center,
                }
            }
            texture {
                name: dyn_name(format!("DropSel_{}_{i}", field.as_str())),
                width: APPEARANCE_SWATCH_DROPDOWN_SELECTION_WIDTH,
                height: APPEARANCE_SWATCH_DROPDOWN_SELECTION_HEIGHT,
                hidden: sel_hidden,
                texture_atlas: "charactercreate-customize-palette-selected",
                anchor {
                    point: AnchorPoint::Center,
                    relative_point: AnchorPoint::Center,
                    x: selection_x,
                }
            }
        }
    }
}

fn dropdown_number_choice(field: AppearanceField, i: usize, selected: bool) -> Element {
    let idx = i as u8;
    let val_text = format!("{}", idx + 1);
    let color = if selected {
        COLOR_SELECTED
    } else {
        COLOR_WHITE
    };
    rsx! {
        r#frame {
            name: dyn_name(format!("DropChoice_{}_{i}", field.as_str())),
            width: APPEARANCE_NUMBER_DROPDOWN_CHOICE_WIDTH,
            height: APPEARANCE_NUMBER_DROPDOWN_CHOICE_HEIGHT,
            onclick: CharCreateAction::SelectChoice(field, idx),
            background_color: "0.15,0.12,0.08,0.9",
            border: "1px solid 0.4,0.35,0.2,0.6",
            fontstring {
                name: dyn_name(format!("DropVal_{}_{i}", field.as_str())),
                width: APPEARANCE_NUMBER_DROPDOWN_CHOICE_WIDTH,
                height: APPEARANCE_NUMBER_DROPDOWN_CHOICE_HEIGHT,
                text: val_text,
                font: GameFont::FrizQuadrata,
                font_size: 11.0,
                font_color: color,
            }
        }
    }
}

fn build_dropdown_choices(
    field: AppearanceField,
    swatches: &[Option<[u8; 3]>],
    selected: u8,
) -> Element {
    swatches
        .iter()
        .enumerate()
        .flat_map(|(i, swatch)| {
            let is_sel = i as u8 == selected;
            match swatch {
                Some(c) => dropdown_color_choice(field, i, *c, is_sel),
                None => dropdown_number_choice(field, i, is_sel),
            }
        })
        .collect()
}

pub(super) fn customization_row(
    label: &str,
    value: u8,
    value_label: &str,
    swatches: &[Option<[u8; 3]>],
    field: AppearanceField,
) -> Element {
    let current_swatch = swatches.get(value as usize).copied().flatten();
    let center = match current_swatch {
        Some(color) => swatch_texture(field, color),
        None => appearance_row_value(field, value, value_label),
    };
    rsx! {
        r#frame {
            name: dyn_name(format!("Appearance_{}", field.as_str())),
            width: 280.0,
            height: 44.0,
            {appearance_row_label(field, label)}
            {stepper_dec_button(field)}
            {center}
            {stepper_inc_button(field)}
        }
    }
}

pub(super) fn dropdown_panel(
    field: AppearanceField,
    swatches: &[Option<[u8; 3]>],
    selected_idx: u8,
    y_offset: f32,
) -> Element {
    let choices = build_dropdown_choices(field, swatches, selected_idx);
    let y_str = format!("{y_offset}");
    rsx! {
        r#frame {
            name: dyn_name(format!("Dropdown_{}", field.as_str())),
            width: APPEARANCE_DROPDOWN_WIDTH,
            height: 0.0,
            strata: FrameStrata::Dialog,
            background_color: "0.05,0.05,0.05,1.0",
            border: "1px solid 0.4,0.35,0.2,0.8",
            layout: "flex-row-wrap",
            gap: APPEARANCE_DROPDOWN_GAP,
            padding: APPEARANCE_DROPDOWN_PADDING,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "20",
                y: y_str,
            }
            {choices}
        }
    }
}

// --- Name input + create button ---

pub(super) fn name_input_field() -> Element {
    rsx! {
        fontstring {
            name: "NameLabel",
            width: 300.0,
            height: 24.0,
            text: "Character Name",
            font: GameFont::FrizQuadrata,
            font_size: 14.0,
            font_color: COLOR_GOLD,
            anchor { point: AnchorPoint::Top, relative_point: AnchorPoint::Top }
        }
        editbox {
            name: CREATE_NAME_INPUT,
            width: 300.0,
            height: 38.0,
            font_size: 16.0,
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::Top,
                y: "-28",
            }
        }
    }
}

pub(super) fn error_label(error_text: Option<&str>) -> Element {
    let error_hidden = error_text.is_none();
    let text = error_text.unwrap_or("");
    rsx! {
        fontstring {
            name: ERROR_TEXT,
            width: 300.0,
            height: 20.0,
            text,
            hidden: error_hidden,
            font: GameFont::FrizQuadrata,
            font_size: 12.0,
            font_color: FontColor::new(1.0, 0.2, 0.2, 1.0),
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::Top,
                y: "-70",
            }
        }
    }
}

pub(super) fn create_confirm_button() -> Element {
    rsx! {
        button {
            name: CREATE_BUTTON,
            width: 205.0,
            height: 42.0,
            text: "Create Character",
            font_size: 14.0,
            onclick: CharCreateAction::CreateConfirm,
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::Top,
                y: "-96",
            }
        }
    }
}

// --- Bottom buttons ---

pub(super) fn back_button() -> Element {
    rsx! {
        button {
            name: BACK_BUTTON,
            width: 188.0,
            height: 42.0,
            text: "Back",
            font_size: 14.0,
            onclick: CharCreateAction::Back,
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            anchor {
                point: AnchorPoint::BottomLeft,
                relative_point: AnchorPoint::BottomLeft,
                x: "12",
                y: "60",
            }
        }
    }
}

pub(super) fn next_button(hidden: bool) -> Element {
    rsx! {
        button {
            name: NEXT_BUTTON,
            width: 188.0,
            height: 42.0,
            text: "Next",
            font_size: 14.0,
            hidden,
            onclick: CharCreateAction::NextMode,
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            anchor {
                point: AnchorPoint::BottomRight,
                relative_point: AnchorPoint::BottomRight,
                x: "-12",
                y: "60",
            }
        }
    }
}

pub(super) fn sex_toggle_button() -> Element {
    rsx! {
        button {
            name: SEX_TOGGLE_BUTTON,
            width: 140.0,
            height: 42.0,
            text: "Toggle Sex",
            font_size: 14.0,
            onclick: CharCreateAction::ToggleSex,
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            anchor {
                point: AnchorPoint::Bottom,
                relative_point: AnchorPoint::Bottom,
                x: "-96",
                y: "60",
            }
        }
    }
}

pub(super) fn randomize_button() -> Element {
    rsx! {
        button {
            name: RANDOMIZE_BUTTON,
            width: 140.0,
            height: 42.0,
            text: "Randomize",
            font_size: 14.0,
            onclick: CharCreateAction::Randomize,
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            anchor {
                point: AnchorPoint::Bottom,
                relative_point: AnchorPoint::Bottom,
                x: "96",
                y: "60",
            }
        }
    }
}

pub(super) fn bottom_buttons(mode: CharCreateMode) -> Element {
    let hide_next = mode != CharCreateMode::RaceClass;
    [
        back_button(),
        next_button(hide_next),
        sex_toggle_button(),
        randomize_button(),
    ]
    .into_iter()
    .flatten()
    .collect()
}
