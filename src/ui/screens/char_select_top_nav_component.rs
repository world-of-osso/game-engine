//! Character-select top navigation: gold-ruled tab row (MODE, SHOP, MENU, REALMS, CAMPSITES).
//! Hovered, pressed and active tabs get a dark box with a gold border and a white label.

use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::button::ButtonState;

use crate::ui::anchor::FrameName;
use crate::ui::widgets::font_string::{FontColor, GameFont, JustifyH};

use super::char_select_component::CharSelectAction;

pub const TOP_NAV: FrameName = FrameName("CharSelectTopNav");
pub const MODE_TAB: FrameName = FrameName("CharSelectModeTab");
pub const SHOP_TAB: FrameName = FrameName("CharSelectShopTab");
pub const MENU_TAB: FrameName = FrameName("CharSelectMenuTab");
pub const REALMS_TAB: FrameName = FrameName("CharSelectRealmsTab");
pub const CAMPSITES_TAB: FrameName = FrameName("CharSelectCampsitesTab");

/// Frame whose visibility marks the CAMPSITES tab active.
const CAMPSITE_PANEL: &str = "CampsitePanel";

const NAV_WIDTH: f32 = 620.0;
const NAV_TOP: f32 = 10.0;
const RULE_HEIGHT: f32 = 1.0;
const TAB_HEIGHT: f32 = 34.0;
const TAB_GAP: f32 = 10.0;
const TAB_ROW_TOP: f32 = 5.0;
const NAV_HEIGHT: f32 = TAB_ROW_TOP * 2.0 + TAB_HEIGHT;
const LABEL_FONT_SIZE: f32 = 17.0;

const RULE_COLOR: &str = "1.0,0.82,0.0,0.75";
const BOX_FILL: &str = "0.02,0.015,0.01,0.82";
const BOX_BORDER: &str = "1px solid 1.0,0.82,0.0,0.9";
const LABEL_GOLD: FontColor = FontColor::new(1.0, 0.82, 0.0, 1.0);
const LABEL_GOLD_RGBA: [f32; 4] = [1.0, 0.82, 0.0, 1.0];
const LABEL_ACTIVE_RGBA: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

/// Left-to-right tabs: frame, label, width, action (`None` renders inert).
const TABS: [(FrameName, &str, f32, Option<CharSelectAction>); 5] = [
    (MODE_TAB, "MODE", 84.0, None),
    (SHOP_TAB, "SHOP", 80.0, None),
    (MENU_TAB, "MENU", 82.0, Some(CharSelectAction::Menu)),
    (REALMS_TAB, "REALMS", 104.0, Some(CharSelectAction::Back)),
    (
        CAMPSITES_TAB,
        "CAMPSITES",
        134.0,
        Some(CharSelectAction::CampsiteToggle),
    ),
];

struct DynName(String);

fn tab_part(tab: FrameName, part: &str) -> DynName {
    DynName(format!("{}{part}", tab.0))
}

fn tab(name: FrameName, text: &str, width: f32, action: Option<&CharSelectAction>) -> Element {
    let onclick = action.map(ToString::to_string).unwrap_or_default();
    rsx! {
        button {
            name,
            width,
            height: TAB_HEIGHT,
            onclick,
            button_default_skin: false,
            r#frame {
                name: tab_part(name, "Box"),
                stretch: true,
                background_color: BOX_FILL,
                border: BOX_BORDER,
                hidden: true,
            }
            fontstring {
                name: tab_part(name, "Label"),
                width,
                height: TAB_HEIGHT,
                text,
                font: GameFont::FrizQuadrata,
                font_size: LABEL_FONT_SIZE,
                font_color: LABEL_GOLD,
                justify_h: JustifyH::Center,
                pos_type: "absolute",
                left: "0%",
                top: "0%",
            }
        }
    }
}

fn rule(name: FrameName, top: f32) -> Element {
    rsx! {
        r#frame {
            name,
            width: NAV_WIDTH,
            height: RULE_HEIGHT,
            background_color: RULE_COLOR,
            pos_type: "absolute",
            left: "0%",
            top,
        }
    }
}

pub fn char_select_top_nav() -> Element {
    let tabs: Element = TABS
        .iter()
        .flat_map(|(name, text, width, action)| tab(*name, text, *width, action.as_ref()))
        .collect();
    rsx! {
        r#frame {
            name: TOP_NAV,
            width: NAV_WIDTH,
            height: NAV_HEIGHT,
            pos_type: "absolute",
            left: "50%",
            top: "0%",
            translate_x: "-50%",
            margin_top: {NAV_TOP},
            {rule(FrameName("CharSelectTopNavRuleTop"), 0.0)}
            r#frame {
                name: "CharSelectTopNavTabs",
                width: NAV_WIDTH,
                height: TAB_HEIGHT,
                layout: "flex-row",
                justify: "center",
                gap: TAB_GAP,
                pos_type: "absolute",
                left: "0%",
                top: TAB_ROW_TOP,
                {tabs}
            }
            {rule(FrameName("CharSelectTopNavRuleBottom"), NAV_HEIGHT - RULE_HEIGHT)}
        }
    }
}

/// Box and label state of every tab from its live button state and the open campsite panel.
pub fn sync_top_nav_tabs(registry: &mut FrameRegistry) {
    let campsites_open = registry
        .get_by_name(CAMPSITE_PANEL)
        .and_then(|id| registry.get(id))
        .is_some_and(|panel| !panel.hidden);
    for (name, ..) in &TABS {
        let selected = *name == CAMPSITES_TAB && campsites_open;
        sync_tab(registry, *name, selected);
    }
}

fn sync_tab(registry: &mut FrameRegistry, tab: FrameName, selected: bool) {
    let Some(WidgetData::Button(button)) = registry
        .get_by_name(tab.0)
        .and_then(|id| registry.get(id))
        .and_then(|frame| frame.widget_data.as_ref())
    else {
        return;
    };
    let active = selected || button.hovered || button.state == ButtonState::Pushed;
    if let Some(id) = registry.get_by_name(&tab_part(tab, "Box").0)
        && registry.get(id).is_some_and(|frame| frame.hidden == active)
    {
        registry.set_hidden(id, !active);
    }
    let color = if active {
        LABEL_ACTIVE_RGBA
    } else {
        LABEL_GOLD_RGBA
    };
    if let Some(id) = registry.get_by_name(&tab_part(tab, "Label").0)
        && let Some(frame) = registry.get_mut(id)
        && let Some(WidgetData::FontString(label)) = &mut frame.widget_data
        && label.color != color
    {
        label.color = color;
    }
}
