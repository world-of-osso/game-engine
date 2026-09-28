//! Campsite selector UI for the char select screen.
//! Grid panel with scene preview cards, opened from the top navigation CAMPSITES tab.

use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont, JustifyH};

use super::char_select_component::{CampsiteState, CharSelectAction};

const COLOR_GOLD: FontColor = FontColor::new(1.0, 0.82, 0.0, 1.0);
const COLOR_SUBTITLE: FontColor = FontColor::new(0.92, 0.88, 0.74, 1.0);

const CARD_BACKDROP_ATLAS: &str = "glues-characterselect-card-singles";
const CARD_WIDTH: f32 = 215.0;
const CARD_HEIGHT: f32 = 173.0;
const CARD_PREVIEW_WIDTH: f32 = 206.0;
const CARD_PREVIEW_HEIGHT: f32 = 165.0;
const CARD_LABEL_WIDTH: f32 = 192.0;
const CARD_LABEL_HEIGHT: f32 = 34.0;
const CARD_LABEL_TEXT_WIDTH: f32 = 176.0;
const CARD_LABEL_TEXT_HEIGHT: f32 = 28.0;
const PANEL_GAP: f32 = 10.0;
const PANEL_PADDING: f32 = 15.0;
const PANEL_WIDTH: f32 = 470.0;
pub const CAMPSITE_PANEL_WIDTH: f32 = PANEL_WIDTH;
pub const CAMPSITE_PANEL_TOP_OFFSET: f32 = 58.0;

struct DynName(String);

fn dyn_name(s: String) -> DynName {
    DynName(s)
}

fn card_backdrop(id: u32, preview_image: Option<&str>) -> Element {
    if let Some(preview_image) = preview_image {
        rsx! {
            texture {
                name: dyn_name(format!("CampsiteCard_{id}")),
                width: CARD_PREVIEW_WIDTH,
                height: CARD_PREVIEW_HEIGHT,
                texture_file: preview_image,
                pos_type: "absolute",
                left: "50%",
                translate_x: "-50%",
                top: 4.0,
            }
        }
    } else {
        rsx! {
            texture {
                name: dyn_name(format!("CampsiteCard_{id}")),
                width: CARD_PREVIEW_WIDTH,
                height: 80.0,
                texture_atlas: CARD_BACKDROP_ATLAS,
                pos_type: "absolute",
                left: "50%",
                translate_x: "-50%",
                top: "50%",
                translate_y: "-50%",
            }
        }
    }
}

fn card_label_bar(id: u32) -> Element {
    rsx! {
        r#frame {
            name: dyn_name(format!("CampsiteLabelBar_{id}")),
            width: CARD_LABEL_WIDTH,
            height: CARD_LABEL_HEIGHT,
            background_color: "0.05,0.04,0.03,0.88",
            border: "1px solid 0.36,0.28,0.08,0.75",
            pos_type: "absolute",
            left: "50%",
            translate_x: "-50%",
            bottom: 4.0,
        }
    }
}

fn card_label(id: u32, name: &str, is_selected: bool) -> Element {
    let color = if is_selected {
        COLOR_GOLD
    } else {
        COLOR_SUBTITLE
    };
    rsx! {
        fontstring {
            name: dyn_name(format!("CampsiteLabel_{id}")),
            width: CARD_LABEL_TEXT_WIDTH,
            height: CARD_LABEL_TEXT_HEIGHT,
            text: name,
            font: GameFont::FrizQuadrata,
            font_size: 14.0,
            font_color: color,
            justify_h: JustifyH::Center,
            pos_type: "absolute",
            left: "50%",
            translate_x: "-50%",
            bottom: 6.0,
        }
    }
}

fn campsite_card(id: u32, name: &str, preview_image: Option<&str>, is_selected: bool) -> Element {
    let border = if is_selected {
        "2px solid 0.95,0.78,0.14,0.95"
    } else {
        "1px solid 0.30,0.24,0.09,0.70"
    };
    let background = if is_selected {
        "0.13,0.10,0.03,0.96"
    } else {
        "0.03,0.03,0.02,0.94"
    };
    rsx! {
        r#frame {
            name: dyn_name(format!("CampsiteScene_{id}")),
            width: CARD_WIDTH,
            height: CARD_HEIGHT,
            background_color: background,
            border,
            onclick: CharSelectAction::SelectCampsite(id),
            {card_backdrop(id, preview_image)}
            {card_label_bar(id)}
            {card_label(id, name, is_selected)}
        }
    }
}

pub fn campsite_panel(state: &CampsiteState) -> Element {
    campsite_panel_at_top(state, 58.0, PANEL_WIDTH)
}

fn build_campsite_cards(state: &CampsiteState) -> Element {
    state
        .scenes
        .iter()
        .flat_map(|e| {
            campsite_card(
                e.id,
                &e.name,
                e.preview_image.as_deref(),
                state.selected_id == Some(e.id),
            )
        })
        .collect()
}

fn campsite_panel_at_top(state: &CampsiteState, top: f32, width: f32) -> Element {
    let hide = !state.panel_visible;
    let cards = build_campsite_cards(state);
    let height = campsite_panel_height(state.scenes.len());
    rsx! {
        r#frame {
            name: "CampsitePanel",
            width,
            height,
            strata: FrameStrata::Dialog,
            hidden: hide,
            background_color: "0.04,0.03,0.02,0.98",
            border: "1px solid 0.62,0.46,0.10,0.75",
            layout: "flex-row-wrap",
            justify: "center",
            gap: PANEL_GAP,
            padding: PANEL_PADDING,
            pos_type: "absolute",
            left: "50%",
            translate_x: "-50%",
            top,
            {cards}
        }
    }
}

pub fn campsite_panel_height(scene_count: usize) -> f32 {
    let rows = (scene_count as f32 / 2.0).ceil();
    let vertical_padding = PANEL_PADDING * 2.0;
    (rows * CARD_HEIGHT + (rows - 1.0).max(0.0) * PANEL_GAP + vertical_padding)
        .max(CARD_HEIGHT + vertical_padding)
}
