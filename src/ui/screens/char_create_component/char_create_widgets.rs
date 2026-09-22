use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

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

fn race_top_widget(race_id: u8, icon_fdid: u32) -> Element {
    rsx! {
        texture {
            name: dyn_name(format!("Race_{race_id}_Icon")),
            width: 36.0,
            height: 36.0,
            texture_fdid: icon_fdid,
            pos_type: "absolute",
            left: "50%",
            top: "0%",
            translate_x: "-50%",
            margin_top: {4.0},
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

const TILE_LABEL_WIDTH: f32 = 72.0;
const TILE_LABEL_HEIGHT: f32 = 24.0;
const TILE_LABEL_FONT_SIZE: f32 = 8.0;
const RACE_LABEL_Y: f32 = 4.0;
const CLASS_LABEL_Y: f32 = 2.0;

fn tile_name_label(name_id: String, name: &str, color: FontColor, y: f32) -> Element {
    let text = wrap_tile_label(name);
    rsx! {
        fontstring {
            name: dyn_name(name_id),
            width: TILE_LABEL_WIDTH,
            height: TILE_LABEL_HEIGHT,
            text,
            font: GameFont::FrizQuadrata,
            font_size: TILE_LABEL_FONT_SIZE,
            font_color: color,
            pos_type: "absolute",
            left: "50%",
            top: "100%",
            translate_x: "-50%",
            translate_y: "-100%",
            margin_top: {-(y)},
        }
    }
}

fn race_name_label(race_id: u8, name: &str, color: FontColor) -> Element {
    tile_name_label(format!("Race_{race_id}_Label"), name, color, RACE_LABEL_Y)
}

pub(super) fn race_buttons_for_faction(
    faction: crate::char_create_data::Faction,
    selected_race: u8,
) -> Element {
    use crate::char_create_data::RACES;
    RACES
        .iter()
        .filter(|r| r.faction == faction)
        .flat_map(|r| race_button(r.id, r.name, r.icon_fdid, r.id == selected_race))
        .collect()
}

pub(super) fn race_button(race_id: u8, name: &str, icon_fdid: u32, is_selected: bool) -> Element {
    let color = if is_selected {
        COLOR_SELECTED
    } else {
        COLOR_SUBTITLE
    };
    let (border, bg) = race_button_style(is_selected);
    let top = race_top_widget(race_id, icon_fdid);
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
            pos_type: "absolute",
            left: "0%",
            top: "0%",
            margin_left: {x_offset},
            margin_top: {4.0},
        }
        r#frame {
            name: dyn_name(format!("{col_name}Races")),
            width: 150.0,
            height: 400.0,
            layout: "flex-row-wrap",
            gap: 6.0,
            pos_type: "absolute",
            left: "0%",
            top: "0%",
            margin_left: {x_offset},
            margin_top: {30.0},
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

fn class_icon_widget(class_id: u8, icon_fdid: u32, alpha: &str) -> Element {
    rsx! {
        texture {
            name: dyn_name(format!("Class_{class_id}_Icon")),
            width: 36.0,
            height: 36.0,
            texture_fdid: icon_fdid,
            alpha,
            pos_type: "absolute",
            left: "50%",
            top: "0%",
            translate_x: "-50%",
            margin_top: {4.0},
        }
    }
}

fn class_name_label(class_id: u8, name: &str, color: FontColor) -> Element {
    tile_name_label(
        format!("Class_{class_id}_Label"),
        name,
        color,
        CLASS_LABEL_Y,
    )
}

pub(super) fn class_button(
    class_id: u8,
    name: &str,
    icon_fdid: u32,
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
            {class_icon_widget(class_id, icon_fdid, alpha)}
            {class_name_label(class_id, name, color)}
        }
    }
}

// --- Name input + create button ---

fn input_border_textures(center_texture: &str) -> [String; 9] {
    let base = "data/ui/Common-Input-Border-";
    [
        format!("{base}TL.blp"),
        format!("{base}T.blp"),
        format!("{base}TR.blp"),
        format!("{base}L.blp"),
        center_texture.to_string(),
        format!("{base}R.blp"),
        format!("{base}BL.blp"),
        format!("{base}B.blp"),
        format!("{base}BR.blp"),
    ]
}

fn focused_name_editbox() -> Element {
    let bg_color = "0.14,0.10,0.07,0.5";
    let textures = input_border_textures("data/textures/editbox-white-fill.ktx2");
    rsx! {
        editbox {
            name: CREATE_NAME_INPUT,
            width: 300.0,
            height: 38.0,
            background_color: {bg_color},
            font: GameFont::ArialNarrow,
            font_size: 16.0,
            font_color: COLOR_GOLD,
            text_insets: "12,5,8,8",
            nine_slice {
                edge_size: 8,
                bg_color: {bg_color},
                border_color: "1.0,0.82,0.0,1.0",
                textures: {textures},
            }
            pos_type: "absolute",
            left: "50%",
            top: "0%",
            translate_x: "-50%",
            margin_top: {28.0},
        }
    }
}

fn unfocused_name_editbox() -> Element {
    let textures = input_border_textures("data/ui/Common-Input-Border-M.blp");
    rsx! {
        editbox {
            name: CREATE_NAME_INPUT,
            width: 300.0,
            height: 38.0,
            font: GameFont::ArialNarrow,
            font_size: 16.0,
            font_color: COLOR_GOLD,
            text_insets: "12,5,8,8",
            nine_slice {
                edge_size: 8,
                bg_color: "1,1,1,1",
                border_color: "1,1,1,1",
                textures: {textures},
            }
            pos_type: "absolute",
            left: "50%",
            top: "0%",
            translate_x: "-50%",
            margin_top: {28.0},
        }
    }
}

pub(super) fn name_input_field(focused: bool) -> Element {
    let editbox = if focused {
        focused_name_editbox()
    } else {
        unfocused_name_editbox()
    };
    rsx! {
        fontstring {
            name: "NameLabel",
            width: 300.0,
            height: 24.0,
            text: "Character Name",
            font: GameFont::FrizQuadrata,
            font_size: 14.0,
            font_color: COLOR_GOLD,
            pos_type: "absolute",
            left: "50%",
            top: "0%",
            translate_x: "-50%",
        }
        {editbox}
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
            pos_type: "absolute",
            left: "50%",
            top: "0%",
            translate_x: "-50%",
            margin_top: {70.0},
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
            pos_type: "absolute",
            left: "50%",
            top: "0%",
            translate_x: "-50%",
            margin_top: {96.0},
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
            pos_type: "absolute",
            left: "0%",
            top: "100%",
            translate_y: "-100%",
            margin_left: {12},
            margin_top: {-60.0},
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
            pos_type: "absolute",
            left: "100%",
            top: "100%",
            translate_x: "-100%",
            translate_y: "-100%",
            margin_left: {-12},
            margin_top: {-60.0},
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
            pos_type: "absolute",
            left: "50%",
            top: "100%",
            translate_x: "-50%",
            translate_y: "-100%",
            margin_left: {-96},
            margin_top: {-60.0},
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
            pos_type: "absolute",
            left: "50%",
            top: "100%",
            translate_x: "-50%",
            translate_y: "-100%",
            margin_left: {96},
            margin_top: {-60.0},
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
