use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;
use crate::ui::widgets::font_string::{FontColor, GameFont};

use super::{
    BACK_BUTTON, BUTTON_ATLAS_DISABLED, BUTTON_ATLAS_HIGHLIGHT, BUTTON_ATLAS_PRESSED,
    BUTTON_ATLAS_UP, COLOR_DISABLED, COLOR_GOLD, COLOR_SELECTED, COLOR_SUBTITLE, CREATE_BUTTON,
    CREATE_NAME_INPUT, CharCreateAction, CharCreateMode, DynName, ERROR_TEXT, NEXT_BUTTON,
    RANDOMIZE_BUTTON, SEX_TOGGLE_BUTTON,
};

fn dyn_name(s: String) -> DynName {
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

fn wrap_tile_label(name: &str) -> String {
    if name.contains('\n') || !name.contains(' ') {
        return name.to_string();
    }
    let midpoint = name.len() / 2;
    let Some((split, _)) = name
        .match_indices(' ')
        .min_by_key(|(idx, _)| idx.abs_diff(midpoint))
    else {
        return name.to_string();
    };
    let (first, second) = (name[..split].trim_end(), name[split + 1..].trim_start());
    if first.is_empty() || second.is_empty() {
        name.to_string()
    } else {
        format!("{first}\n{second}")
    }
}

fn race_name_label(race_id: u8, name: &str, color: FontColor) -> Element {
    let text = wrap_tile_label(name);
    rsx! {
        fontstring {
            name: dyn_name(format!("Race_{race_id}_Label")),
            width: 72.0,
            height: 24.0,
            text,
            font: GameFont::FrizQuadrata,
            font_size: 8.0,
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
            width: 56.0,
            height: 72.0,
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
    let text = wrap_tile_label(name);
    rsx! {
        fontstring {
            name: dyn_name(format!("Class_{class_id}_Label")),
            width: 72.0,
            height: 24.0,
            text,
            font: GameFont::FrizQuadrata,
            font_size: 8.0,
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
            width: 56.0,
            height: 72.0,
            onclick,
            border,
            background_color: bg,
            {class_icon_widget(class_id, icon, alpha)}
            {class_name_label(class_id, name, color)}
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
