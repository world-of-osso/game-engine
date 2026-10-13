//! CharacterSelectNavBar.xml; end-cap overrides in CharacterSelectNavBarMixin:SetButtonVisuals.
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::button::ButtonState;

use super::char_select_component::CharSelectAction;
use crate::ui::anchor::FrameName;
use crate::ui::widgets::font_string::{FontColor, GameFont, JustifyH};

pub const TOP_NAV: FrameName = FrameName("CharSelectTopNav");
pub const MODE_TAB: FrameName = FrameName("CharSelectModeTab");
pub const SHOP_TAB: FrameName = FrameName("CharSelectShopTab");
pub const MENU_TAB: FrameName = FrameName("CharSelectMenuTab");
pub const REALMS_TAB: FrameName = FrameName("CharSelectRealmsTab");
pub const CAMPSITES_TAB: FrameName = FrameName("CharSelectCampsitesTab");
const CAMPSITE_PANEL: &str = "CampsitePanel";
const NAV_HEIGHT: f32 = 55.0;
const TAB_HEIGHT: f32 = 51.0;
// Lua NavBarButtonWidthBuffer=70: native text measurement plus 35 at each side.
const TEXT_PADDING: f32 = 35.0;
const LABEL_FONT_SIZE: f32 = 16.0;
const LABEL_GOLD: FontColor = FontColor::new(1.0, 0.82, 0.0, 1.0);
const LABEL_GOLD_RGBA: [f32; 4] = [1.0, 0.82, 0.0, 1.0];
const LABEL_ACTIVE_RGBA: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const TABS: [(FrameName, &str, Option<CharSelectAction>); 5] = [
    (MODE_TAB, "MODE", None),
    (SHOP_TAB, "SHOP", None),
    (MENU_TAB, "MENU", Some(CharSelectAction::Menu)),
    (REALMS_TAB, "REALMS", Some(CharSelectAction::Back)),
    (
        CAMPSITES_TAB,
        "CAMPSITES",
        Some(CharSelectAction::CampsiteToggle),
    ),
];
struct DynName(String);
fn tab_part(tab: FrameName, part: &str) -> DynName {
    DynName(format!("{}{part}", tab.0))
}

fn tab_art(name: FrameName) -> Element {
    let end = if name == MODE_TAB {
        "left"
    } else if name == CAMPSITES_TAB {
        "right"
    } else {
        "middle"
    };
    let normal_left = if name == MODE_TAB { -102.0 } else { 0.0 };
    let normal_right = if name == CAMPSITES_TAB { -102.0 } else { 0.0 };
    let highlight_left = if name == MODE_TAB { -45.0 } else { 0.0 };
    let highlight_right = if name == CAMPSITES_TAB { -45.0 } else { 0.0 };
    let hide_divider = name == CAMPSITES_TAB;
    rsx! {
        texture {
            name: tab_part(name, "Normal"), texture_atlas: format!("glues-characterselect-tophud-{end}-bg"),
            pos_type: "absolute", left: normal_left, right: normal_right, top: 0.0, bottom: 0.0,
        }
        r#frame {
            name: tab_part(name, "Box"), hidden: true,
            pos_type: "absolute", left: highlight_left, right: highlight_right, top: 0.0, bottom: 0.0,
            texture {
                name: tab_part(name, "Selected"), texture_atlas: format!("glues-characterselect-tophud-selected-{end}"),
                pos_type: "absolute", left: -2.0, right: 0.0, top: 0.0, bottom: 4.0,
            }
            texture {
                name: tab_part(name, "SelectedLine"), texture_atlas: format!("glues-characterselect-tophud-selected-line-{end}"),
                pos_type: "absolute", left: -2.0, right: 0.0, top: "100%", margin_top: -5.0, height: 4.0,
            }
        }
        texture {
            name: tab_part(name, "Divider"), texture_atlas: "glues-characterselect-tophud-bg-divider",
            hidden: hide_divider, pos_type: "absolute", right: 0.0, top: "50%", translate_y: "-50%",
        }
    }
}

fn tab(name: FrameName, text: &str, action: Option<&CharSelectAction>) -> Element {
    let onclick = action.map(ToString::to_string).unwrap_or_default();
    rsx! {
        button {
            name, width: "auto", height: TAB_HEIGHT, onclick, button_default_skin: false,
            layout: "flex-row", align: "center", padding_left: TEXT_PADDING, padding_right: TEXT_PADDING,
            {tab_art(name)}
            fontstring {
                name: tab_part(name, "Label"), width: "auto", height: TAB_HEIGHT,
                text, font: GameFont::FrizQuadrata, font_size: LABEL_FONT_SIZE,
                font_color: LABEL_GOLD, justify_h: JustifyH::Center,
            }
        }
    }
}

pub fn char_select_top_nav() -> Element {
    let tabs: Element = TABS
        .iter()
        .flat_map(|(name, text, action)| tab(*name, text, action.as_ref()))
        .collect();
    rsx! {
        r#frame {
            name: TOP_NAV, width: "100%", height: NAV_HEIGHT,
            pos_type: "absolute", left: 0.0, top: 0.0,
            r#frame {
                name: "CharSelectTopNavTabs", width: "auto", height: NAV_HEIGHT,
                layout: "flex-row", align: "center", gap: 0.0, padding_left: 4.0, padding_right: 4.0,
                pos_type: "absolute", left: "50%", top: 0.0, translate_x: "-50%",
                {tabs}
            }
        }
    }
}

pub fn sync_top_nav_tabs(registry: &mut FrameRegistry) {
    let campsites_open = registry
        .get_by_name(CAMPSITE_PANEL)
        .and_then(|id| registry.get(id))
        .is_some_and(|panel| !panel.hidden);
    for (name, ..) in &TABS {
        sync_tab(registry, *name, *name == CAMPSITES_TAB && campsites_open);
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
