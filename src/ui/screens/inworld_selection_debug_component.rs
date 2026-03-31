use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::{AnchorPoint, FrameName};
use crate::ui::screens::inworld_hud_component::{action_bar_screen, minimap_screen};
use crate::ui::strata::FrameStrata;

const PANEL_BG: &str = "0.02,0.03,0.04,0.88";
const PANEL_BORDER: &str = "0.97,0.79,0.28,0.18";
const ROW_IDLE: &str = "0.05,0.07,0.08,0.9";
const ROW_SELECTED: &str = "0.18,0.14,0.07,0.96";
const ROW_ACCENT: &str = "0.96,0.78,0.25,0.28";
const TEXT_GOLD: &str = "1.0,0.84,0.44,1.0";
const TEXT_SUBTITLE: &str = "0.88,0.90,0.92,1.0";
const TEXT_MUTED: &str = "0.67,0.72,0.76,1.0";
const BUTTON_ATLAS_UP: &str = "defaultbutton-nineslice-up";
const BUTTON_ATLAS_PRESSED: &str = "defaultbutton-nineslice-pressed";
const BUTTON_ATLAS_HIGHLIGHT: &str = "defaultbutton-nineslice-highlight";
const BUTTON_ATLAS_DISABLED: &str = "defaultbutton-nineslice-disabled";

pub const INWORLD_SELECTION_DEBUG_ROOT: FrameName = FrameName("InWorldSelectionDebugRoot");
pub const INWORLD_SELECTION_DEBUG_PREV: FrameName = FrameName("InWorldSelectionDebugPrev");
pub const INWORLD_SELECTION_DEBUG_NEXT: FrameName = FrameName("InWorldSelectionDebugNext");
pub const INWORLD_SELECTION_DEBUG_PIN: FrameName = FrameName("InWorldSelectionDebugPin");
pub const INWORLD_SELECTION_DEBUG_BACK: FrameName = FrameName("InWorldSelectionDebugBack");

struct DynName(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InWorldSelectionDebugAction {
    SelectEntry(usize),
    SelectCircleStyle(usize),
    Prev,
    Next,
    TogglePinned,
    Back,
}

impl fmt::Display for InWorldSelectionDebugAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SelectEntry(i) => write!(f, "inworld_selection_debug_select:{i}"),
            Self::SelectCircleStyle(i) => write!(f, "inworld_selection_debug_circle:{i}"),
            Self::Prev => f.write_str("inworld_selection_debug_prev"),
            Self::Next => f.write_str("inworld_selection_debug_next"),
            Self::TogglePinned => f.write_str("inworld_selection_debug_pin"),
            Self::Back => f.write_str("inworld_selection_debug_back"),
        }
    }
}

impl InWorldSelectionDebugAction {
    pub fn parse(value: &str) -> Option<Self> {
        if let Some(i) = value.strip_prefix("inworld_selection_debug_select:") {
            return i.parse().ok().map(Self::SelectEntry);
        }
        if let Some(i) = value.strip_prefix("inworld_selection_debug_circle:") {
            return i.parse().ok().map(Self::SelectCircleStyle);
        }
        match value {
            "inworld_selection_debug_prev" => Some(Self::Prev),
            "inworld_selection_debug_next" => Some(Self::Next),
            "inworld_selection_debug_pin" => Some(Self::TogglePinned),
            "inworld_selection_debug_back" => Some(Self::Back),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InWorldSelectionDebugEntry {
    pub label: String,
    pub category: String,
    pub target_rule: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InWorldSelectionDebugState {
    pub entries: Vec<InWorldSelectionDebugEntry>,
    pub selected_index: usize,
    pub pinned: bool,
    pub last_action: String,
    pub circle_styles: Vec<String>,
    pub active_circle_style: usize,
}

fn row_name(index: usize) -> DynName {
    DynName(format!("InWorldSelectionDebugRow_{index}"))
}

fn row_selected_name(index: usize) -> DynName {
    DynName(format!("InWorldSelectionDebugRow_{index}Selected"))
}

fn row_label_name(index: usize) -> DynName {
    DynName(format!("InWorldSelectionDebugRow_{index}Label"))
}

fn row_category_name(index: usize) -> DynName {
    DynName(format!("InWorldSelectionDebugRow_{index}Category"))
}

fn button(name: FrameName, text: &str, action: InWorldSelectionDebugAction, x: f32) -> Element {
    rsx! {
        button {
            name,
            width: 160.0,
            height: 42.0,
            text: text,
            font_size: 14.0,
            onclick: action,
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            anchor {
                point: AnchorPoint::BottomLeft,
                relative_to: INWORLD_SELECTION_DEBUG_ROOT,
                relative_point: AnchorPoint::BottomLeft,
                x: {x.to_string()},
                y: "18",
            }
        }
    }
}

fn selection_rows(entries: &[InWorldSelectionDebugEntry], selected_index: usize) -> Element {
    entries
        .iter()
        .enumerate()
        .flat_map(|(index, entry)| row(index, entry, selected_index == index))
        .collect()
}

fn row_label(index: usize, text: &str) -> Element {
    rsx! {
        fontstring {
            name: {row_label_name(index)},
            width: 290.0,
            height: 24.0,
            text: {text},
            font_size: 18.0,
            color: TEXT_GOLD,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "14", y: "-12" }
        }
    }
}

fn row_category(index: usize, text: &str) -> Element {
    rsx! {
        fontstring {
            name: {row_category_name(index)},
            width: 290.0,
            height: 16.0,
            text: {text},
            font_size: 12.0,
            color: TEXT_SUBTITLE,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "14", y: "-38" }
        }
    }
}

fn row(index: usize, entry: &InWorldSelectionDebugEntry, selected: bool) -> Element {
    let not_selected = !selected;
    rsx! {
        r#frame {
            name: {row_name(index)},
            width: 332.0,
            height: 74.0,
            onclick: InWorldSelectionDebugAction::SelectEntry(index),
            background_color: if selected { ROW_SELECTED } else { ROW_IDLE },
            r#frame {
                name: {row_selected_name(index)},
                width: 332.0, height: 4.0,
                hidden: {not_selected},
                background_color: ROW_ACCENT,
                anchor { point: AnchorPoint::Top, relative_point: AnchorPoint::Top }
            }
            {row_label(index, &entry.label)}
            {row_category(index, &entry.category)}
        }
    }
}

fn panel_header(name: DynName, title: &str, subtitle: &str) -> Element {
    rsx! {
        r#frame {
            name: {DynName(format!("{}Border", name.0))},
            width: 360.0,
            height: 2.0,
            background_color: PANEL_BORDER,
            anchor { point: AnchorPoint::Top, relative_point: AnchorPoint::Top }
        }
        fontstring {
            name: {DynName(format!("{}Title", name.0))},
            width: 320.0,
            height: 20.0,
            text: {title},
            font_size: 18.0,
            color: TEXT_GOLD,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "14", y: "-14" }
        }
        fontstring {
            name: {DynName(format!("{}Helper", name.0))},
            width: 320.0,
            height: 34.0,
            text: {subtitle},
            font_size: 12.0,
            color: TEXT_MUTED,
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "14", y: "-38" }
        }
    }
}

fn candidate_panel(state: &InWorldSelectionDebugState) -> Element {
    rsx! {
        r#frame {
            name: "InWorldSelectionDebugListPanel",
            width: 360.0,
            height: 360.0,
            background_color: PANEL_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_to: INWORLD_SELECTION_DEBUG_ROOT,
                relative_point: AnchorPoint::TopLeft,
                x: "22", y: "-22",
            }
            {panel_header(
                DynName("InWorldSelectionDebugList".into()),
                "In-World Selection Cases",
                "Cycle likely target classes while viewing the selected wolf in the in-world debug scene.",
            )}
            r#frame {
                name: "InWorldSelectionDebugRows",
                width: 332.0,
                height: 272.0,
                layout: "flex-col",
                gap: 8.0,
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "14", y: "-82" }
                {selection_rows(&state.entries, state.selected_index)}
            }
        }
    }
}

fn circle_style_row(index: usize, label: &str, active: bool) -> Element {
    rsx! {
        r#frame {
            name: {DynName(format!("CircleStyle_{index}"))},
            width: 200.0,
            height: 30.0,
            onclick: InWorldSelectionDebugAction::SelectCircleStyle(index),
            background_color: if active { ROW_SELECTED } else { ROW_IDLE },
            fontstring {
                name: {DynName(format!("CircleStyleLabel_{index}"))},
                width: 180.0,
                height: 20.0,
                text: {label},
                font_size: 14.0,
                color: if active { TEXT_GOLD } else { TEXT_SUBTITLE },
                anchor { point: AnchorPoint::Left, relative_point: AnchorPoint::Left, x: "10" }
            }
        }
    }
}

fn circle_style_panel(state: &InWorldSelectionDebugState) -> Element {
    let rows: Element = state
        .circle_styles
        .iter()
        .enumerate()
        .flat_map(|(i, label)| circle_style_row(i, label, i == state.active_circle_style))
        .collect();
    rsx! {
        r#frame {
            name: "CircleStylePanel",
            width: 228.0,
            height: 400.0,
            background_color: PANEL_BG,
            anchor {
                point: AnchorPoint::TopRight,
                relative_to: INWORLD_SELECTION_DEBUG_ROOT,
                relative_point: AnchorPoint::TopRight,
                x: "-22", y: "-22",
            }
            {panel_header(
                DynName("CircleStyle".into()),
                "Circle Style",
                "Pick a target circle texture.",
            )}
            r#frame {
                name: "CircleStyleRows",
                width: 210.0,
                height: 340.0,
                layout: "flex-col",
                gap: 4.0,
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: "10", y: "-82" }
                {rows}
            }
        }
    }
}

pub fn inworld_selection_debug_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<InWorldSelectionDebugState>()
        .expect("InWorldSelectionDebugState must be present");

    rsx! {
        r#frame {
            name: INWORLD_SELECTION_DEBUG_ROOT,
            stretch: true,
            background_color: "0.0,0.0,0.0,0.0",
            strata: FrameStrata::Background,
            {action_bar_screen(ctx)}
            {minimap_screen(ctx)}
            {candidate_panel(state)}
            {circle_style_panel(state)}
            {button(INWORLD_SELECTION_DEBUG_PREV, "Previous", InWorldSelectionDebugAction::Prev, 22.0)}
            {button(INWORLD_SELECTION_DEBUG_NEXT, "Next", InWorldSelectionDebugAction::Next, 192.0)}
            {button(
                INWORLD_SELECTION_DEBUG_PIN,
                if state.pinned { "Unpin Target" } else { "Pin Target" },
                InWorldSelectionDebugAction::TogglePinned,
                362.0,
            )}
            {button(INWORLD_SELECTION_DEBUG_BACK, "Back", InWorldSelectionDebugAction::Back, 938.0)}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_toolkit::screen::Screen;

    use crate::ui::registry::FrameRegistry;

    fn sample_state() -> InWorldSelectionDebugState {
        InWorldSelectionDebugState {
            entries: vec![
                InWorldSelectionDebugEntry {
                    label: "Enemy Creature".to_string(),
                    category: "Hostile unit".to_string(),
                    target_rule: "required".to_string(),
                    detail: "Validates hostile target ring, target frame title updates, and spellbook cast routing.".to_string(),
                },
                InWorldSelectionDebugEntry {
                    label: "World Object".to_string(),
                    category: "Interactable prop".to_string(),
                    target_rule: "optional".to_string(),
                    detail: "Checks how mailbox or chest selection differs from unit targeting and whether the HUD still surfaces actionable context.".to_string(),
                },
            ],
            selected_index: 1,
            pinned: true,
            last_action: "Pinned World Object".to_string(),
            circle_styles: vec!["Procedural".to_string(), "Fire".to_string()],
            active_circle_style: 0,
        }
    }

    #[test]
    fn screen_renders_inworld_debug_panels() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(sample_state());
        Screen::new(inworld_selection_debug_screen).sync(&shared, &mut reg);

        assert!(reg.get_by_name(INWORLD_SELECTION_DEBUG_ROOT.0).is_some());
        assert!(reg.get_by_name("MainActionBar").is_some());
        assert!(reg.get_by_name("MinimapCluster").is_some());

        let selected = reg
            .get_by_name("InWorldSelectionDebugRow_1Selected")
            .expect("selected marker");
        let selected = reg.get(selected).expect("selected frame");
        assert!(!selected.hidden);

        assert!(
            reg.get_by_name("InWorldSelectionDebugDiagnostics")
                .is_none()
        );
    }
}
