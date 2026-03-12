use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;
use crate::ui::widgets::font_string::{FontColor, GameFont, JustifyH};

use super::{
    AppearanceField, BACK_BUTTON, BUTTON_ATLAS_DISABLED, BUTTON_ATLAS_HIGHLIGHT,
    BUTTON_ATLAS_PRESSED, BUTTON_ATLAS_UP, COLOR_DISABLED, COLOR_GOLD, COLOR_SELECTED,
    COLOR_SUBTITLE, COLOR_WHITE, CREATE_BUTTON, CREATE_NAME_INPUT, CharCreateAction,
    CharCreateMode, DynName, ERROR_TEXT, NEXT_BUTTON, SEX_TOGGLE_BUTTON,
};

pub(super) fn dyn_name(s: String) -> DynName {
    DynName(s)
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
                    y: "-2",
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
            width: 50.0,
            height: 16.0,
            text: name,
            font: GameFont::FrizQuadrata,
            font_size: 9.0,
            font_color: color,
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
                y: "-2",
            }
        }
    }
}

fn class_name_label(class_id: u8, name: &str, color: FontColor) -> Element {
    rsx! {
        fontstring {
            name: dyn_name(format!("Class_{class_id}_Label")),
            width: 50.0,
            height: 16.0,
            text: name,
            font: GameFont::FrizQuadrata,
            font_size: 9.0,
            font_color: color,
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

pub(super) fn appearance_dec_button(field: AppearanceField) -> Element {
    rsx! {
        button {
            name: dyn_name(format!("AppDec_{}", field.as_str())),
            width: 32.0,
            height: 28.0,
            text: "<",
            font_size: 14.0,
            onclick: CharCreateAction::AppearanceDec(field),
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            anchor {
                point: AnchorPoint::Right,
                relative_point: AnchorPoint::Right,
                x: "-70",
            }
        }
    }
}

pub(super) fn appearance_inc_button(field: AppearanceField) -> Element {
    rsx! {
        button {
            name: dyn_name(format!("AppInc_{}", field.as_str())),
            width: 32.0,
            height: 28.0,
            text: ">",
            font_size: 14.0,
            onclick: CharCreateAction::AppearanceInc(field),
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            anchor {
                point: AnchorPoint::Right,
                relative_point: AnchorPoint::Right,
                x: "-10",
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

fn appearance_row_value(field: AppearanceField, value: u8) -> Element {
    let val_text = format!("{}", value + 1);
    rsx! {
        fontstring {
            name: dyn_name(format!("AppVal_{}", field.as_str())),
            width: 30.0,
            height: 24.0,
            text: val_text,
            font: GameFont::FrizQuadrata,
            font_size: 13.0,
            font_color: COLOR_WHITE,
            anchor {
                point: AnchorPoint::Right,
                relative_point: AnchorPoint::Right,
                x: "-40",
            }
        }
    }
}

fn appearance_swatch(field: AppearanceField, color: [u8; 3]) -> Element {
    let bg = format!(
        "{},{},{},1.0",
        color[0] as f32 / 255.0,
        color[1] as f32 / 255.0,
        color[2] as f32 / 255.0
    );
    rsx! {
        r#frame {
            name: dyn_name(format!("AppSwatch_{}", field.as_str())),
            width: 26.0,
            height: 20.0,
            background_color: bg,
            border: "1px solid 0.6,0.5,0.3,0.8",
            anchor {
                point: AnchorPoint::Right,
                relative_point: AnchorPoint::Right,
                x: "-42",
            }
        }
    }
}

pub(super) fn color_swatch_row(
    label: &str,
    value: u8,
    swatch_color: Option<[u8; 3]>,
    field: AppearanceField,
) -> Element {
    let center = if let Some(color) = swatch_color {
        appearance_swatch(field, color)
    } else {
        appearance_row_value(field, value)
    };
    rsx! {
        r#frame {
            name: dyn_name(format!("Appearance_{}", field.as_str())),
            width: 280.0,
            height: 32.0,
            {appearance_row_label(field, label)}
            {appearance_dec_button(field)}
            {center}
            {appearance_inc_button(field)}
        }
    }
}

pub(super) fn appearance_row(label: &str, value: u8, field: AppearanceField) -> Element {
    rsx! {
        r#frame {
            name: dyn_name(format!("Appearance_{}", field.as_str())),
            width: 280.0,
            height: 32.0,
            {appearance_row_label(field, label)}
            {appearance_dec_button(field)}
            {appearance_row_value(field, value)}
            {appearance_inc_button(field)}
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
                y: "-90",
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
                y: "60",
            }
        }
    }
}

pub(super) fn bottom_buttons(mode: CharCreateMode) -> Element {
    let hide_next = mode != CharCreateMode::RaceClass;
    [back_button(), next_button(hide_next), sex_toggle_button()]
        .into_iter()
        .flatten()
        .collect()
}
