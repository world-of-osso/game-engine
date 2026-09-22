use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::char_create_data::{Faction, RACES, RaceInfo};
use crate::ui::widgets::font_string::{FontColor, GameFont, JustifyH};

use super::navigation_art::navigation_layers;
use super::reference_layout::*;
use super::{
    BACK_BUTTON, COLOR_DISABLED, COLOR_GOLD, COLOR_WHITE, CREATE_BUTTON, CREATE_NAME_INPUT,
    CameraControl, CharCreateAction, CharCreateMode, CharCreateUiState, CustomizationCategoryUi,
    DynName, ERROR_TEXT, NEXT_BUTTON,
};

pub(super) fn atlas_centered(
    name: String,
    atlas: &str,
    width: f32,
    height: f32,
    hidden: bool,
) -> Element {
    rsx! {
        texture { name: DynName(name), width, height, texture_atlas: atlas, hidden,
            pos_type: "absolute", left: "50%", top: "50%", translate_x: "-50%", translate_y: "-50%",
        }
    }
}

fn ring(name: &str, atlas: &str, size: [f32; 2]) -> Element {
    atlas_centered(format!("{name}_Ring"), atlas, size[0], size[1], false)
}

fn selection_ring(name: &str, size: f32, selected: bool) -> Element {
    atlas_centered(
        format!("{name}_Selected"),
        "charactercreate-ring-select",
        size,
        size,
        !selected,
    )
}

fn icon(name: &str, fdid: u32, size: f32, disabled: bool) -> Element {
    let alpha = if disabled { 0.25 } else { 1.0 };
    rsx! {
        texture { name: DynName(format!("{name}_Icon")), width: size, height: size,
            texture_fdid: fdid, alpha,
            pos_type: "absolute", left: "50%", top: "50%", translate_x: "-50%", translate_y: "-50%",
        }
    }
}

fn tile_label(
    frame_name: &str,
    label: &str,
    width: f32,
    y: f32,
    color: FontColor,
    hidden: bool,
) -> Element {
    rsx! {
        fontstring { name: DynName(format!("{frame_name}_Label")), width, height: 40.0,
            text: label, hidden, font: GameFont::FrizQuadrata, font_size: 12.0, font_color: color,
            pos_type: "absolute", left: "50%", top: y, translate_x: "-50%",
        }
    }
}

fn race_button(race: &RaceInfo, selected: bool, position: [f32; 2]) -> Element {
    let frame_name = format!("Race_{}", race.id);
    let ring_atlas = match race.faction {
        Faction::Alliance => "charactercreate-ring-alliance",
        Faction::Horde => "charactercreate-ring-horde",
    };
    rsx! {
        button { name: DynName(frame_name.clone()), width: 79.0, height: 79.0,
            button_default_skin: "false",
            onclick: CharCreateAction::SelectRace(race.id),
            button_atlas_highlight: "charactercreate-ring-select",
            pos_type: "absolute", left: position[0], top: position[1],
            {icon(&frame_name, race.icon_fdid, 79.0, false)}
            {ring(&frame_name, ring_atlas, [139.0, 140.0])}
            {selection_ring(&frame_name, 118.0, selected)}
            {tile_label(&frame_name, race.name, 112.0, 72.0, COLOR_GOLD, !selected)}
        }
    }
}

fn calculate_race_button_position(
    faction: Faction,
    allied: bool,
    index: usize,
    step: f32,
) -> [f32; 2] {
    let x = match (faction, allied) {
        (Faction::Alliance, false) => 68.0,
        (Faction::Alliance, true) => 165.0,
        (Faction::Horde, false) => 103.0,
        (Faction::Horde, true) => 6.0,
    };
    let top = if allied { 174.0 } else { 106.0 };
    [x, top + index as f32 * step]
}

fn build_faction_race_buttons(faction: Faction, state: &CharCreateUiState) -> Element {
    let races: Vec<_> = RACES
        .iter()
        .filter(|race| race.faction == faction)
        .collect();
    let base_count = races.iter().filter(|race| race.id < 27).count();
    let space = state.viewport_height as f32 - 106.0 - NAV_HEIGHT - NAV_BOTTOM - 20.0;
    let step = 79.0 + fit_spacing(space, base_count, 79.0, 18.0);
    let mut counts = [0, 0];
    races
        .into_iter()
        .flat_map(|race| {
            let allied = race.id >= 27;
            let column = usize::from(allied);
            let position = calculate_race_button_position(faction, allied, counts[column], step);
            counts[column] += 1;
            race_button(race, race.id == state.selected_race, position)
        })
        .collect()
}

fn build_faction_header(name: &str, alliance: bool) -> Element {
    let (atlas, icon_x, text_x, justify) = if alliance {
        ("charactercreate-icon-alliance", 3.0, 77.0, JustifyH::Left)
    } else {
        ("charactercreate-icon-horde", 155.0, 6.0, JustifyH::Right)
    };
    rsx! {
        texture { name: DynName(format!("{name}Emblem")), width: 92.0, height: 100.0,
            texture_atlas: atlas, pos_type: "absolute", left: icon_x, top: 10.0,
        }
        fontstring { name: DynName(format!("{name}Label")), width: 167.0, height: 26.0,
            text: name, font: GameFont::FrizQuadrata, font_size: 20.0, font_color: COLOR_GOLD, justify_h: justify,
            pos_type: "absolute", left: text_x, top: 47.0,
        }
    }
}

pub(super) fn faction_column(faction: Faction, state: &CharCreateUiState) -> Element {
    let alliance = faction == Faction::Alliance;
    let name = if alliance { "Alliance" } else { "Horde" };
    let x = if alliance {
        0.0
    } else {
        state.viewport_width as f32 - 250.0
    };
    let buttons = build_faction_race_buttons(faction, state);
    rsx! {
        r#frame { name: DynName(format!("{name}Races")), width: 250.0, height: "fill",
            pos_type: "absolute", left: x, top: 0.0,
            {build_faction_header(name, alliance)}
            {buttons}
        }
    }
}

pub(super) fn class_button(
    id: u8,
    name: &str,
    fdid: u32,
    selected: bool,
    available: bool,
    bounds: [f32; 3],
) -> Element {
    let frame_name = format!("Class_{id}");
    let disabled = !available;
    let ring_atlas = if disabled {
        "charactercreate-ring-metaldark-disabled"
    } else {
        "charactercreate-ring-metaldark"
    };
    let color = if disabled {
        COLOR_DISABLED
    } else if selected {
        COLOR_GOLD
    } else {
        COLOR_WHITE
    };
    let scale = bounds[2] / 67.0;
    let onclick = CharCreateAction::SelectClass(id).when_enabled(available);
    rsx! {
        button { name: DynName(frame_name.clone()), width: bounds[2], height: bounds[2], disabled,
            button_default_skin: "false",
            onclick,
            button_atlas_highlight: "charactercreate-ring-select",
            pos_type: "absolute", left: bounds[0], top: bounds[1],
            {icon(&frame_name, fdid, bounds[2], disabled)}
            {ring(&frame_name, ring_atlas, [116.0 * scale, 117.0 * scale])}
            {selection_ring(&frame_name, 99.0 * scale, selected && available)}
            {tile_label(&frame_name, name, 85.0 * scale, bounds[2] - 3.0, color, false)}
        }
    }
}

pub(super) fn category_button(
    category: &CustomizationCategoryUi,
    selected: bool,
    x: f32,
) -> Element {
    let frame_name = format!("Category_{}", category.id);
    let normal = category
        .icon_atlas
        .as_deref()
        .map(|atlas| {
            atlas_centered(
                format!("{frame_name}_Icon"),
                atlas,
                104.0,
                105.0,
                selected && category.selected_icon_atlas.is_some(),
            )
        })
        .unwrap_or_default();
    let active = category
        .selected_icon_atlas
        .as_deref()
        .map(|atlas| {
            atlas_centered(
                format!("{frame_name}_SelectedIcon"),
                atlas,
                104.0,
                105.0,
                !selected,
            )
        })
        .unwrap_or_default();
    rsx! {
        button { name: DynName(frame_name.clone()), width: CATEGORY_WIDTH, height: CATEGORY_HEIGHT,
            button_default_skin: "false",
            onclick: CharCreateAction::SelectCategory(category.id),
            hit_rect_insets: "15,15,15,15",
            button_atlas_highlight: "charactercreate-ring-select",
            pos_type: "absolute", left: x, top: 0.0,
            {normal}
            {active}
            {ring(&frame_name, "charactercreate-ring-metallight", [108.0, 109.0])}
            {selection_ring(&frame_name, 93.0, selected)}
        }
    }
}

pub(super) fn small_button(
    name: &str,
    icon_atlas: &str,
    action: CharCreateAction,
    x: f32,
    y: f32,
) -> Element {
    rsx! {
        button { name: DynName(name.to_string()), width: 48.0, height: 48.0, onclick: action,
            button_atlas_up: "common-button-square-gray-up",
            button_atlas_pressed: "common-button-square-gray-down",
            button_atlas_highlight: "common-button-square-gray-up",
            pos_type: "absolute", left: x, top: y,
            {atlas_centered(format!("{name}_Icon"), icon_atlas, 24.0, 23.0, false)}
        }
    }
}

pub(super) fn camera_controls() -> Element {
    let controls = [
        (CameraControl::Reset, "common-icon-undo", 0.0),
        (CameraControl::ZoomOut, "common-icon-zoomout", 43.0),
        (CameraControl::ZoomIn, "common-icon-zoomin", 86.0),
        (CameraControl::RotateLeft, "common-icon-rotateleft", 159.0),
        (CameraControl::RotateRight, "common-icon-rotateright", 202.0),
    ];
    let buttons: Element = controls
        .into_iter()
        .flat_map(|(control, atlas, x)| {
            small_button(
                &format!("Camera_{}", control.as_str()),
                atlas,
                CharCreateAction::Camera(control),
                x,
                0.0,
            )
        })
        .collect();
    rsx! {
        r#frame { name: "CharCreateCameraControls", width: 250.0, height: 48.0,
            pos_type: "absolute", left: 40.0, top: 30.0,
            {buttons}
        }
    }
}

fn body_type_button(sex: u8, selected: bool, x: f32) -> Element {
    let name = format!("CharCreateSex_{sex}");
    let atlas = match (sex, selected) {
        (0, false) => "charactercreate-gendericon-male",
        (0, true) => "charactercreate-gendericon-male-selected",
        (_, false) => "charactercreate-gendericon-female",
        (_, true) => "charactercreate-gendericon-female-selected",
    };
    rsx! {
        button { name: DynName(name.clone()), width: 46.0, height: 46.0,
            button_default_skin: "false",
            onclick: CharCreateAction::SelectSex(sex),
            button_atlas_highlight: "charactercreate-ring-select",
            pos_type: "absolute", left: x, top: 0.0,
            {atlas_centered(format!("{name}_Icon"), atlas, 46.0, 46.0, false)}
            {ring(&name, "charactercreate-ring-metaldark", [99.0, 100.0])}
            {selection_ring(&name, 84.0, selected)}
        }
    }
}

pub(super) fn body_type_buttons(state: &CharCreateUiState) -> Element {
    let x = match state.mode {
        CharCreateMode::RaceClass => (state.viewport_width as f32 - 114.0) / 2.0,
        CharCreateMode::Customize => state.viewport_width as f32 - 41.0 - 114.0,
    };
    let y = if state.mode == CharCreateMode::RaceClass {
        27.0
    } else {
        37.0
    };
    rsx! {
        r#frame { name: "CharCreateBodyTypes", width: 114.0, height: 46.0,
            pos_type: "absolute", left: x, top: y,
            {body_type_button(0, state.selected_sex == 0, 0.0)}
            {body_type_button(1, state.selected_sex == 1, 68.0)}
        }
    }
}

pub(super) fn name_input_field(state: &CharCreateUiState) -> Element {
    let base = "data/ui/Common-Input-Border-";
    let center = if state.name_input_focused {
        "data/textures/editbox-white-fill.ktx2"
    } else {
        "data/ui/Common-Input-Border-M.blp"
    };
    let textures = [
        format!("{base}TL.blp"),
        format!("{base}T.blp"),
        format!("{base}TR.blp"),
        format!("{base}L.blp"),
        center.to_string(),
        format!("{base}R.blp"),
        format!("{base}BL.blp"),
        format!("{base}B.blp"),
        format!("{base}BR.blp"),
    ];
    let bg = if state.name_input_focused {
        "0.14,0.10,0.07,0.5"
    } else {
        "1,1,1,1"
    };
    let error_hidden = state.error_text.is_none();
    let error = state.error_text.as_deref().unwrap_or("");
    rsx! {
        r#frame { name: "NamePanel", width: 400.0, height: 100.0,
            pos_type: "absolute", left: "50%", top: 34.0, translate_x: "-50%",
            fontstring { name: "NameLabel", width: 300.0, height: 24.0, text: "Name",
                font: GameFont::FrizQuadrata, font_size: 20.0, font_color: COLOR_WHITE,
                pos_type: "absolute", left: "50%", translate_x: "-50%", top: 0.0,
            }
            editbox { name: CREATE_NAME_INPUT, width: 300.0, height: 38.0, text: state.name.clone(),
                font: GameFont::ArialNarrow, font_size: 16.0, font_color: COLOR_GOLD,
                text_insets: "12,5,8,8",
                nine_slice { edge_size: 8, bg_color: bg, border_color: "1,1,1,1", textures: textures, }
                pos_type: "absolute", left: "50%", translate_x: "-50%", top: 26.0,
            }
            fontstring { name: ERROR_TEXT, width: 400.0, height: 30.0, text: error, hidden: error_hidden,
                font: GameFont::FrizQuadrata, font_size: 12.0,
                font_color: FontColor::new(1.0, 0.2, 0.2, 1.0),
                pos_type: "absolute", left: 0.0, top: 70.0,
            }
        }
    }
}

pub(super) fn bottom_buttons(mode: CharCreateMode) -> Element {
    let (forward_name, text, action) = match mode {
        CharCreateMode::RaceClass => (NEXT_BUTTON, "Customize", CharCreateAction::NextMode),
        CharCreateMode::Customize => (
            CREATE_BUTTON,
            "Create Character",
            CharCreateAction::CreateConfirm,
        ),
    };
    rsx! {
        button { name: BACK_BUTTON, width: NAV_WIDTH, height: NAV_HEIGHT,
            text: "", onclick: CharCreateAction::Back,
            button_default_skin: false,
            pos_type: "absolute", left: NAV_SIDE, bottom: NAV_BOTTOM,
            {navigation_layers(BACK_BUTTON.0, "Back", NAV_WIDTH, NAV_HEIGHT)}
        }
        button { name: forward_name, width: NAV_WIDTH, height: NAV_HEIGHT,
            text: "", onclick: action,
            button_default_skin: false,
            pos_type: "absolute", right: NAV_SIDE, bottom: NAV_BOTTOM,
            {navigation_layers(forward_name.0, text, NAV_WIDTH, NAV_HEIGHT)}
        }
    }
}
