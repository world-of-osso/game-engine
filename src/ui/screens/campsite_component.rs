//! Campsite selector UI for the char select screen.
//! Top HUD tab (gs-tophud atlas) + grid panel with scene preview cards.

use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;
use crate::ui::widgets::font_string::{FontColor, GameFont};

use super::char_select_component::{CampsiteState, CharSelectAction};

const COLOR_GOLD: FontColor = FontColor::new(1.0, 0.82, 0.0, 1.0);
const COLOR_SUBTITLE: FontColor = FontColor::new(0.92, 0.88, 0.74, 1.0);

const GS_TAB_LEFT: &str = "glues-characterselect-gs-tophud-left";
const GS_TAB_MID: &str = "glues-characterselect-gs-tophud-middle";
const GS_TAB_RIGHT: &str = "glues-characterselect-gs-tophud-right";
const GS_TAB_LEFT_SEL: &str = "glues-characterselect-gs-tophud-left-selected";
const GS_TAB_MID_SEL: &str = "glues-characterselect-gs-tophud-middle-selected";
const GS_TAB_RIGHT_SEL: &str = "glues-characterselect-gs-tophud-right-selected";

const CARD_BACKDROP_ATLAS: &str = "glues-characterselect-card-singles";
const CARD_SELECTED_ATLAS: &str = "glues-characterselect-card-selected";

struct DynName(String);

fn dyn_name(s: String) -> DynName {
    DynName(s)
}

fn tab_atlas_set(selected: bool) -> (&'static str, &'static str, &'static str) {
    if selected {
        (GS_TAB_LEFT_SEL, GS_TAB_MID_SEL, GS_TAB_RIGHT_SEL)
    } else {
        (GS_TAB_LEFT, GS_TAB_MID, GS_TAB_RIGHT)
    }
}

fn tab_texture_strip(left: &str, mid: &str, right: &str) -> Element {
    rsx! {
        texture {
            name: "CampsiteTabLeft",
            width: 50.0,
            height: 43.0,
            texture_atlas: left,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
            }
        }
        texture {
            name: "CampsiteTabMid",
            width: 80.0,
            height: 43.0,
            texture_atlas: mid,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "50",
            }
        }
        texture {
            name: "CampsiteTabRight",
            width: 50.0,
            height: 43.0,
            texture_atlas: right,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "130",
            }
        }
    }
}

fn tab_label() -> Element {
    rsx! {
        fontstring {
            name: "CampsiteTabLabel",
            width: 180.0,
            height: 43.0,
            text: "Campsites",
            font: GameFont::FrizQuadrata,
            font_size: 14.0,
            font_color: COLOR_GOLD,
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
            }
        }
    }
}

pub fn campsite_tab(selected: bool) -> Element {
    let (left, mid, right) = tab_atlas_set(selected);
    rsx! {
        r#frame {
            name: "CampsiteTab",
            width: 180.0,
            height: 43.0,
            onclick: CharSelectAction::CampsiteToggle,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "18",
                y: "-24",
            }
            {tab_texture_strip(left, mid, right)}
            {tab_label()}
        }
    }
}

fn card_backdrop(id: u32, is_selected: bool) -> Element {
    let atlas = if is_selected {
        CARD_SELECTED_ATLAS
    } else {
        CARD_BACKDROP_ATLAS
    };
    rsx! {
        texture {
            name: dyn_name(format!("CampsiteCard_{id}")),
            width: 220.0,
            height: 80.0,
            texture_atlas: atlas,
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
            }
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
            width: 200.0,
            height: 24.0,
            text: name,
            font: GameFont::FrizQuadrata,
            font_size: 14.0,
            font_color: color,
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
            }
        }
    }
}

fn campsite_card(id: u32, name: &str, is_selected: bool) -> Element {
    rsx! {
        r#frame {
            name: dyn_name(format!("CampsiteScene_{id}")),
            width: 220.0,
            height: 80.0,
            onclick: CharSelectAction::SelectCampsite(id),
            {card_backdrop(id, is_selected)}
            {card_label(id, name, is_selected)}
        }
    }
}

pub fn campsite_grid(state: &CampsiteState) -> Element {
    let hide = !state.panel_visible;
    let cards: Element = state
        .scenes
        .iter()
        .flat_map(|e| campsite_card(e.id, &e.name, state.selected_id == Some(e.id)))
        .collect();
    let rows = (state.scenes.len() as f32 / 2.0).ceil();
    let height = (rows * 90.0 + 24.0).max(100.0);
    rsx! {
        r#frame {
            name: "CampsitePanel",
            width: 470.0,
            height,
            hidden: hide,
            nine_slice: "12.0,0.03,0.03,0.04,0.94,0.65,0.48,0.16,1.0",
            layout: "flex-row-wrap",
            gap: 10.0,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "18",
                y: "-74",
            }
            {cards}
        }
    }
}
