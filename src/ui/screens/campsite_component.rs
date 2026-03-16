//! Campsite selector UI for the char select screen.
//! Top HUD tab (gs-tophud atlas) + grid panel with scene preview cards.

use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;
use crate::ui::widgets::font_string::{FontColor, GameFont, JustifyH};

use super::char_select_component::{CampsiteState, CharSelectAction};

const COLOR_GOLD: FontColor = FontColor::new(1.0, 0.82, 0.0, 1.0);
const COLOR_SUBTITLE: FontColor = FontColor::new(0.92, 0.88, 0.74, 1.0);

const CARD_BACKDROP_ATLAS: &str = "glues-characterselect-card-singles";
const CARD_SELECTED_ATLAS: &str = "glues-characterselect-card-selected";
const MENU_BAR_WIDTH: f32 = 470.0;
const MENU_BAR_HEIGHT: f32 = 44.0;
const MENU_ITEM_HEIGHT: f32 = 44.0;
const MENU_MODE_WIDTH: f32 = 100.0;
const MENU_SHOP_WIDTH: f32 = 73.0;
const MENU_MENU_WIDTH: f32 = 92.0;
const MENU_REALMS_WIDTH: f32 = 92.0;
const MENU_CAMPSITES_WIDTH: f32 = 113.0;
const MENU_ITEM_Y: &str = "-1";
const MENU_DIVIDER_HEIGHT: f32 = 22.0;
const MENU_DIVIDER_Y: &str = "-10";
const MENU_SHOP_X: &str = "100";
const MENU_MENU_X: &str = "173";
const MENU_REALMS_X: &str = "265";
const MENU_CAMPSITES_X: &str = "357";

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

fn menu_item(
    name: &'static str,
    text: &'static str,
    width: f32,
    x: &str,
    selected: bool,
) -> Element {
    let name_id = name.to_string();
    let name = dyn_name(name_id.clone());
    let underline_width = width - 18.0;
    let color = if selected { COLOR_GOLD } else { COLOR_SUBTITLE };
    let underline = if selected {
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
    } else {
        Vec::new()
    };
    rsx! {
        r#frame {
            name,
            width,
            height: MENU_ITEM_HEIGHT,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x,
                y: MENU_ITEM_Y,
            }
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
            {underline}
        }
    }
}

pub fn campsite_tab(selected: bool) -> Element {
    let mode = menu_item("CampsiteModeTab", "MODE", MENU_MODE_WIDTH, "0", false);
    let shop = menu_item(
        "CampsiteShopTab",
        "SHOP",
        MENU_SHOP_WIDTH,
        MENU_SHOP_X,
        false,
    );
    let menu = menu_item(
        "CampsiteMenuTab",
        "MENU",
        MENU_MENU_WIDTH,
        MENU_MENU_X,
        false,
    );
    let realms = menu_item(
        "CampsiteRealmsTab",
        "REALMS",
        MENU_REALMS_WIDTH,
        MENU_REALMS_X,
        false,
    );
    let campsites = if selected {
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
                fontstring {
                    name: "CampsiteTabLabel",
                    width: MENU_CAMPSITES_WIDTH,
                    height: MENU_ITEM_HEIGHT,
                    text: "CAMPSITES",
                    font: GameFont::FrizQuadrata,
                    font_size: 14.0,
                    font_color: COLOR_GOLD,
                    justify_h: JustifyH::Center,
                    anchor {
                        point: AnchorPoint::Center,
                        relative_point: AnchorPoint::Center,
                    }
                }
                r#frame {
                    name: "CampsiteTabUnderline",
                    width: 95.0,
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
    } else {
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
                fontstring {
                    name: "CampsiteTabLabel",
                    width: MENU_CAMPSITES_WIDTH,
                    height: MENU_ITEM_HEIGHT,
                    text: "CAMPSITES",
                    font: GameFont::FrizQuadrata,
                    font_size: 14.0,
                    font_color: COLOR_SUBTITLE,
                    justify_h: JustifyH::Center,
                    anchor {
                        point: AnchorPoint::Center,
                        relative_point: AnchorPoint::Center,
                    }
                }
            }
        }
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
            {mode}
            {menu_divider("CampsiteShopDivider", MENU_SHOP_X)}
            {shop}
            {menu_divider("CampsiteMenuDivider", MENU_MENU_X)}
            {menu}
            {menu_divider("CampsiteRealmsDivider", MENU_REALMS_X)}
            {realms}
            {menu_divider("CampsiteCampsitesDivider", MENU_CAMPSITES_X)}
            {campsites}
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

pub fn campsite_panel(state: &CampsiteState) -> Element {
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
            background_color: "0.08,0.07,0.06,0.82",
            border: "1px solid 0.62,0.46,0.10,0.75",
            layout: "flex-row-wrap",
            gap: 10.0,
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::Top,
                y: "-34",
            }
            {cards}
        }
    }
}
