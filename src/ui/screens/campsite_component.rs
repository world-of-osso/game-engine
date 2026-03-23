//! Campsite selector UI for the char select screen.
//! Top HUD tab (gs-tophud atlas) + grid panel with scene preview cards.

use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;
use crate::ui::widgets::font_string::{FontColor, GameFont, JustifyH};

use super::char_select_component::{CampsiteState, CharSelectAction};

const COLOR_GOLD: FontColor = FontColor::new(1.0, 0.82, 0.0, 1.0);
const COLOR_SUBTITLE: FontColor = FontColor::new(0.92, 0.88, 0.74, 1.0);
const COLOR_DISABLED: FontColor = FontColor::new(0.50, 0.48, 0.42, 0.6);

const CARD_BACKDROP_ATLAS: &str = "glues-characterselect-card-singles";
const MENU_BAR_WIDTH: f32 = 470.0;
const MENU_BAR_HEIGHT: f32 = 44.0;
const MENU_ITEM_HEIGHT: f32 = 44.0;
const MENU_CAMPSITES_WIDTH: f32 = 113.0;
const MENU_ITEM_Y: &str = "-1";
const MENU_DIVIDER_HEIGHT: f32 = 22.0;
const MENU_DIVIDER_Y: &str = "-10";
const MENU_CAMPSITES_X: &str = "357";
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

const DISABLED_ITEMS: &[(&str, &str, f32, &str)] = &[
    ("CampsiteModeTab", "MODE", 100.0, "0"),
    ("CampsiteShopTab", "SHOP", 73.0, "100"),
    ("CampsiteMenuTab", "MENU", 92.0, "173"),
    ("CampsiteRealmsTab", "REALMS", 92.0, "265"),
];
const DIVIDER_POSITIONS: &[(&str, &str)] = &[
    ("CampsiteShopDivider", "100"),
    ("CampsiteMenuDivider", "173"),
    ("CampsiteRealmsDivider", "265"),
    ("CampsiteCampsitesDivider", "357"),
];

struct DynName(String);

fn dyn_name(s: String) -> DynName {
    DynName(s)
}

fn menu_divider(name: &'static str, x: &str) -> Element {
    let name = dyn_name(name.to_string());
    rsx! {
        r#frame {
            name,
            width: 1.0,
            height: MENU_DIVIDER_HEIGHT,
            background_color: "0.95,0.72,0.12,0.55",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x,
                y: MENU_DIVIDER_Y,
            }
        }
    }
}

fn menu_item_underline(name_id: &str, width: f32) -> Element {
    let underline_width = width - 18.0;
    rsx! {
        r#frame {
            name: dyn_name(format!("{name_id}Underline")),
            width: underline_width,
            height: 2.0,
            background_color: "1.0,0.78,0.10,0.95",
            anchor {
                point: AnchorPoint::Bottom,
                relative_point: AnchorPoint::Bottom,
                y: "2",
            }
        }
    }
}

fn menu_item_label(name_id: &str, text: &str, width: f32, color: FontColor) -> Element {
    rsx! {
        fontstring {
            name: dyn_name(format!("{name_id}Label")),
            width,
            height: MENU_ITEM_HEIGHT,
            text,
            font: GameFont::FrizQuadrata,
            font_size: 14.0,
            font_color: color,
            justify_h: JustifyH::Center,
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
            }
        }
    }
}

fn disabled_menu_item(name: &str, text: &str, width: f32, x: &str) -> Element {
    let name_id = name.to_string();
    rsx! {
        r#frame {
            name: dyn_name(name_id.clone()),
            width,
            height: MENU_ITEM_HEIGHT,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x,
                y: MENU_ITEM_Y,
            }
            {menu_item_label(&name_id, text, width, COLOR_DISABLED)}
        }
    }
}

fn campsite_menu_item_selected() -> Element {
    rsx! {
        r#frame {
            name: "CampsiteTab",
            width: MENU_CAMPSITES_WIDTH,
            height: MENU_ITEM_HEIGHT,
            onclick: CharSelectAction::CampsiteToggle,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: MENU_CAMPSITES_X,
                y: MENU_ITEM_Y,
            }
            {menu_item_label("CampsiteTab", "CAMPSITES", MENU_CAMPSITES_WIDTH, COLOR_GOLD)}
            {menu_item_underline("CampsiteTab", MENU_CAMPSITES_WIDTH)}
        }
    }
}

fn campsite_menu_item_unselected() -> Element {
    rsx! {
        r#frame {
            name: "CampsiteTab",
            width: MENU_CAMPSITES_WIDTH,
            height: MENU_ITEM_HEIGHT,
            onclick: CharSelectAction::CampsiteToggle,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: MENU_CAMPSITES_X,
                y: MENU_ITEM_Y,
            }
            {menu_item_label("CampsiteTab", "CAMPSITES", MENU_CAMPSITES_WIDTH, COLOR_SUBTITLE)}
        }
    }
}

fn menu_bar_chrome() -> Element {
    rsx! {
        r#frame {
            name: "CampsiteMenuBarTopShade",
            width: "fill",
            height: 12.0,
            background_color: "0.18,0.14,0.08,0.20",
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::Top,
            }
        }
        r#frame {
            name: "CampsiteMenuBarBottomGlow",
            width: "fill",
            height: 3.0,
            background_color: "0.96,0.74,0.11,0.92",
            anchor {
                point: AnchorPoint::Bottom,
                relative_point: AnchorPoint::Bottom,
            }
        }
    }
}

fn disabled_items_and_dividers() -> Element {
    let items: Element = DISABLED_ITEMS
        .iter()
        .flat_map(|(name, text, width, x)| disabled_menu_item(name, text, *width, x))
        .collect();
    let dividers: Element = DIVIDER_POSITIONS
        .iter()
        .flat_map(|(name, x)| menu_divider(name, x))
        .collect();
    [items, dividers].into_iter().flatten().collect()
}

pub fn campsite_tab(selected: bool) -> Element {
    let campsites = if selected {
        campsite_menu_item_selected()
    } else {
        campsite_menu_item_unselected()
    };
    rsx! {
        r#frame {
            name: "CampsiteMenuBar",
            width: MENU_BAR_WIDTH,
            height: MENU_BAR_HEIGHT,
            background_color: "0.05,0.04,0.03,0.72",
            border: "1px solid 0.22,0.17,0.05,0.55",
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::Top,
            }
            {menu_bar_chrome()}
            {disabled_items_and_dividers()}
            {campsites}
        }
    }
}

fn card_backdrop(id: u32, preview_image: Option<&str>) -> Element {
    if let Some(preview_image) = preview_image {
        rsx! {
            texture {
                name: dyn_name(format!("CampsiteCard_{id}")),
                width: CARD_PREVIEW_WIDTH,
                height: CARD_PREVIEW_HEIGHT,
                texture_file: preview_image,
                anchor {
                    point: AnchorPoint::Top,
                    relative_point: AnchorPoint::Top,
                    y: "-4",
                }
            }
        }
    } else {
        rsx! {
            texture {
                name: dyn_name(format!("CampsiteCard_{id}")),
                width: CARD_PREVIEW_WIDTH,
                height: 80.0,
                texture_atlas: CARD_BACKDROP_ATLAS,
                anchor {
                    point: AnchorPoint::Center,
                    relative_point: AnchorPoint::Center,
                }
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
            anchor {
                point: AnchorPoint::Bottom,
                relative_point: AnchorPoint::Bottom,
                y: "4",
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
            width: CARD_LABEL_TEXT_WIDTH,
            height: CARD_LABEL_TEXT_HEIGHT,
            text: name,
            font: GameFont::FrizQuadrata,
            font_size: 14.0,
            font_color: color,
            justify_h: JustifyH::Center,
            anchor {
                point: AnchorPoint::Bottom,
                relative_point: AnchorPoint::Bottom,
                y: "6",
            }
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
    campsite_panel_with_anchor(state, AnchorPoint::Top, AnchorPoint::Top, "-58", PANEL_WIDTH)
}

fn campsite_panel_with_anchor(
    state: &CampsiteState,
    point: AnchorPoint,
    relative_point: AnchorPoint,
    y: &'static str,
    width: f32,
) -> Element {
    let hide = !state.panel_visible;
    let cards: Element = state
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
        .collect();
    let height = campsite_panel_height(state.scenes.len());
    rsx! {
        r#frame {
            name: "CampsitePanel",
            width,
            height,
            hidden: hide,
            background_color: "0.04,0.03,0.02,0.98",
            border: "1px solid 0.62,0.46,0.10,0.75",
            layout: "flex-row-wrap",
            justify: "center",
            gap: PANEL_GAP,
            padding: PANEL_PADDING,
            anchor {
                point,
                relative_point,
                y,
            }
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
