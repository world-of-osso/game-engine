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
const LABEL_FONT_SIZE: f32 = 15.0;
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
    let divider = ui_toolkit::atlas::get_region("glues-characterselect-tophud-bg-divider")
        .expect("CharacterSelectNavBar divider atlas");
    rsx! {
        texture {
            name: tab_part(name, "Normal"), width: "auto", height: "auto", texture_atlas: format!("glues-characterselect-tophud-{end}-bg"),
            pos_type: "absolute", left: normal_left, right: normal_right, top: 0.0, bottom: 0.0,
        }
        texture {
            name: tab_part(name, "Disabled"), width: "auto", height: "auto", texture_atlas: format!("glues-characterselect-tophud-{end}-dis-bg"),
            hidden: true, pos_type: "absolute", left: normal_left, right: normal_right, top: 0.0, bottom: 0.0,
        }
        r#frame {
            name: tab_part(name, "Box"), width: "auto", height: "auto", hidden: true,
            pos_type: "absolute", left: highlight_left, right: highlight_right, top: 0.0, bottom: 0.0,
            texture {
                name: tab_part(name, "Selected"), width: "auto", height: "auto", texture_atlas: format!("glues-characterselect-tophud-selected-{end}"),
                pos_type: "absolute", left: -2.0, right: 0.0, top: 0.0, bottom: 4.0,
            }
            texture {
                name: tab_part(name, "SelectedLine"), width: "auto", texture_atlas: format!("glues-characterselect-tophud-selected-line-{end}"),
                pos_type: "absolute", left: -2.0, right: 0.0, top: "100%", margin_top: -5.0, height: 4.0,
            }
        }
        texture {
            name: tab_part(name, "Divider"), width: divider.width, height: divider.height,
            texture_atlas: "glues-characterselect-tophud-bg-divider",
            hidden: hide_divider, pos_type: "absolute", right: 0.0, top: "50%", translate_y: "-50%",
        }
    }
}

fn label_width(text: &str) -> f32 {
    ui_toolkit::text_measure::measure_text(text, GameFont::FrizQuadrata, LABEL_FONT_SIZE)
        .expect("CharacterSelectNavBar requires the loaded GlueFontNormal font")
        .0
}

fn tab(name: FrameName, text: &str, action: Option<&CharSelectAction>) -> Element {
    let onclick = action.map(ToString::to_string).unwrap_or_default();
    let text_width = label_width(text);
    let width = text_width + TEXT_PADDING * 2.0;
    rsx! {
        button {
            name, width, height: TAB_HEIGHT, onclick, button_default_skin: false,
            {tab_art(name)}
            fontstring {
                name: tab_part(name, "Label"), width: text_width, height: TAB_HEIGHT,
                text, font: GameFont::FrizQuadrata, font_size: LABEL_FONT_SIZE,
                font_color: LABEL_GOLD, justify_h: JustifyH::Center,
                pos_type: "absolute", left: "50%", top: 0.0, translate_x: "-50%",
            }
        }
    }
}

pub fn char_select_top_nav() -> Element {
    let tray_width: f32 = TABS
        .iter()
        .map(|(_, text, _)| label_width(text) + TEXT_PADDING * 2.0)
        .sum::<f32>()
        + 8.0;
    let tabs: Element = TABS
        .iter()
        .flat_map(|(name, text, action)| tab(*name, text, action.as_ref()))
        .collect();
    rsx! {
        r#frame {
            name: TOP_NAV, width: "auto", height: NAV_HEIGHT,
            pos_type: "absolute", left: 0.0, right: 0.0, top: 0.0,
            r#frame {
                name: "CharSelectTopNavTabs", width: tray_width, height: NAV_HEIGHT,
                layout: "flex-row", align: "center", justify: "center", gap: 0.0,
                pos_type: "absolute", left: "50%", top: 0.0, translate_x: "-50%",
                {tabs}
            }
        }
    }
}

pub fn sync_top_nav_tabs(registry: &mut FrameRegistry) {
    crate::campsite_component::sync_campsite_art(registry);
    super::char_select_delete_confirm_component::sync_glue_dialog_art(registry);
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
    let disabled = button.state == ButtonState::Disabled;
    let active = !disabled && (selected || button.hovered || button.state == ButtonState::Pushed);
    for (part, hidden) in [("Normal", disabled), ("Disabled", !disabled)] {
        if let Some(id) = registry.get_by_name(&tab_part(tab, part).0) {
            registry.set_hidden(id, hidden);
        }
    }
    if let Some(id) = registry.get_by_name(&tab_part(tab, "Box").0)
        && registry.get(id).is_some_and(|frame| frame.hidden == active)
    {
        registry.set_hidden(id, !active);
    }
    let color = if disabled {
        [0.5, 0.5, 0.5, 1.0]
    } else if active {
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
