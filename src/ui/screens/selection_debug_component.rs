use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::{AnchorPoint, FrameName};
use crate::ui::screens::screen_title::framed_title;
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontColor, GameFont, JustifyH};

const BUTTON_ATLAS_UP: &str = "defaultbutton-nineslice-up";
const BUTTON_ATLAS_PRESSED: &str = "defaultbutton-nineslice-pressed";
const BUTTON_ATLAS_HIGHLIGHT: &str = "defaultbutton-nineslice-highlight";
const BUTTON_ATLAS_DISABLED: &str = "defaultbutton-nineslice-disabled";

const PANEL_BG: &str = "0.04,0.04,0.05,0.94";
const PANEL_BORDER: &str = "0.92,0.76,0.34,0.18";
const ROW_IDLE: &str = "0.08,0.08,0.10,0.94";
const ROW_SELECTED: &str = "0.19,0.13,0.05,0.98";
const ROW_ACCENT: &str = "0.98,0.81,0.32,0.22";
const TEXT_GOLD: FontColor = FontColor::new(1.0, 0.84, 0.54, 1.0);
const TEXT_SUBTITLE: FontColor = FontColor::new(0.87, 0.84, 0.76, 1.0);
const TEXT_MUTED: FontColor = FontColor::new(0.64, 0.66, 0.70, 1.0);

pub const SELECTION_DEBUG_ROOT: FrameName = FrameName("SelectionDebugRoot");
const SCREEN_MOUNT: FrameName = FrameName("SelectionDebugMount");
const TITLE_FRAME: FrameName = FrameName("SelectionDebugTitleFrame");
const TITLE_LABEL: FrameName = FrameName("SelectionDebugTitleLabel");
const LIST_PANEL: FrameName = FrameName("SelectionDebugListPanel");
const DETAIL_PANEL: FrameName = FrameName("SelectionDebugDetailPanel");
const STATUS_TEXT: FrameName = FrameName("SelectionDebugStatusText");
pub const PREV_BUTTON: FrameName = FrameName("SelectionDebugPrev");
pub const NEXT_BUTTON: FrameName = FrameName("SelectionDebugNext");
pub const PIN_BUTTON: FrameName = FrameName("SelectionDebugPin");
pub const BACK_BUTTON: FrameName = FrameName("SelectionDebugBack");

struct DynName(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionDebugAction {
    SelectEntry(usize),
    Prev,
    Next,
    TogglePinned,
    Back,
}

impl fmt::Display for SelectionDebugAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SelectEntry(index) => write!(f, "selection_debug_select:{index}"),
            Self::Prev => f.write_str("selection_debug_prev"),
            Self::Next => f.write_str("selection_debug_next"),
            Self::TogglePinned => f.write_str("selection_debug_pin"),
            Self::Back => f.write_str("selection_debug_back"),
        }
    }
}

impl SelectionDebugAction {
    pub fn parse(value: &str) -> Option<Self> {
        if let Some(index) = value.strip_prefix("selection_debug_select:") {
            return index.parse().ok().map(Self::SelectEntry);
        }
        match value {
            "selection_debug_prev" => Some(Self::Prev),
            "selection_debug_next" => Some(Self::Next),
            "selection_debug_pin" => Some(Self::TogglePinned),
            "selection_debug_back" => Some(Self::Back),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionDebugEntry {
    pub label: String,
    pub subtitle: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionDebugState {
    pub entries: Vec<SelectionDebugEntry>,
    pub selected_index: usize,
    pub pinned: bool,
    pub last_action: String,
}

fn row_name(index: usize) -> DynName {
    DynName(format!("SelectionDebugRow_{index}"))
}

fn row_label_name(index: usize) -> DynName {
    DynName(format!("SelectionDebugRow_{index}Label"))
}

fn row_subtitle_name(index: usize) -> DynName {
    DynName(format!("SelectionDebugRow_{index}Subtitle"))
}

fn row_selected_name(index: usize) -> DynName {
    DynName(format!("SelectionDebugRow_{index}Selected"))
}

fn helper_text() -> Element {
    rsx! {
        fontstring {
            name: "SelectionDebugHelper",
            width: 940.0,
            height: 24.0,
            text: "Arrow keys cycle candidates. Enter or Space toggles pinned mode. Click any row to force a selection state.",
            font: GameFont::FrizQuadrata,
            font_size: 15.0,
            font_color: TEXT_MUTED,
            justify_h: JustifyH::Center,
            anchor {
                point: AnchorPoint::Top,
                relative_to: SCREEN_MOUNT,
                relative_point: AnchorPoint::Top,
                y: "-44",
            }
        }
    }
}

fn list_panel(entries: &[SelectionDebugEntry], selected_index: usize) -> Element {
    let rows: Element = entries
        .iter()
        .enumerate()
        .flat_map(|(index, entry)| selection_row(index, entry, index == selected_index))
        .collect();

    rsx! {
        r#frame {
            name: LIST_PANEL,
            width: 450.0,
            height: 470.0,
            background_color: PANEL_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_to: SCREEN_MOUNT,
                relative_point: AnchorPoint::TopLeft,
                x: "32",
                y: "-86",
            }
            {list_panel_border()}
            {list_panel_title()}
            {list_panel_rows(rows)}
        }
    }
}

fn list_panel_border() -> Element {
    rsx! {
        r#frame {
            name: "SelectionDebugListBorder",
            width: 450.0,
            height: 2.0,
            background_color: PANEL_BORDER,
            anchor {
                point: AnchorPoint::Top,
                relative_to: LIST_PANEL,
                relative_point: AnchorPoint::Top,
            }
        }
    }
}

fn list_panel_title() -> Element {
    rsx! {
        fontstring {
            name: "SelectionDebugListTitle",
            width: 390.0,
            height: 28.0,
            text: "Selection Candidates",
            font: GameFont::FrizQuadrata,
            font_size: 20.0,
            font_color: TEXT_GOLD,
            justify_h: JustifyH::Left,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_to: LIST_PANEL,
                relative_point: AnchorPoint::TopLeft,
                x: "24",
                y: "-18",
            }
        }
    }
}

fn list_panel_rows(rows: Element) -> Element {
    rsx! {
        r#frame {
            name: "SelectionDebugRows",
            width: 402.0,
            height: 368.0,
            layout: "flex-col",
            gap: 12.0,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_to: LIST_PANEL,
                relative_point: AnchorPoint::TopLeft,
                x: "24",
                y: "-56",
            }
            {rows}
        }
    }
}

fn selection_row(index: usize, entry: &SelectionDebugEntry, selected: bool) -> Element {
    let background = if selected { ROW_SELECTED } else { ROW_IDLE };
    let hide_selected = !selected;
    rsx! {
        r#frame {
            name: {row_name(index)},
            width: 402.0,
            height: 80.0,
            onclick: SelectionDebugAction::SelectEntry(index),
            background_color: {background},
            {selection_row_selected_accent(index, hide_selected)}
            {selection_row_label(index, &entry.label)}
            {selection_row_subtitle(index, &entry.subtitle)}
        }
    }
}

fn selection_row_selected_accent(index: usize, hide_selected: bool) -> Element {
    rsx! {
        r#frame {
            name: {row_selected_name(index)},
            width: 402.0,
            height: 4.0,
            hidden: hide_selected,
            background_color: ROW_ACCENT,
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::Top,
            }
        }
    }
}

fn selection_row_label(index: usize, label: &str) -> Element {
    rsx! {
        fontstring {
            name: {row_label_name(index)},
            width: 340.0,
            height: 28.0,
            text: {label},
            font: GameFont::FrizQuadrata,
            font_size: 20.0,
            font_color: TEXT_GOLD,
            justify_h: JustifyH::Left,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "16",
                y: "-14",
            }
        }
    }
}

fn selection_row_subtitle(index: usize, subtitle: &str) -> Element {
    rsx! {
        fontstring {
            name: {row_subtitle_name(index)},
            width: 360.0,
            height: 18.0,
            text: {subtitle},
            font: GameFont::FrizQuadrata,
            font_size: 13.0,
            font_color: TEXT_SUBTITLE,
            justify_h: JustifyH::Left,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "16",
                y: "-42",
            }
        }
    }
}

fn detail_panel(state: &SelectionDebugState) -> Element {
    let fallback = SelectionDebugEntry {
        label: "No selection".to_string(),
        subtitle: "Nothing selected".to_string(),
        detail: "Populate SelectionDebugState.entries before rendering this screen.".to_string(),
    };
    let selected = state.entries.get(state.selected_index).unwrap_or(&fallback);

    rsx! {
        r#frame {
            name: DETAIL_PANEL,
            width: 580.0,
            height: 470.0,
            background_color: PANEL_BG,
            anchor {
                point: AnchorPoint::TopRight,
                relative_to: SCREEN_MOUNT,
                relative_point: AnchorPoint::TopRight,
                x: "-32",
                y: "-86",
            }
            r#frame {
                name: "SelectionDebugDetailBorder",
                width: 580.0,
                height: 2.0,
                background_color: PANEL_BORDER,
                anchor {
                    point: AnchorPoint::Top,
                    relative_to: DETAIL_PANEL,
                    relative_point: AnchorPoint::Top,
                }
            }
            {detail_panel_selected_copy(selected)}
            {detail_panel_mode_copy(state)}
        }
    }
}

fn detail_panel_selected_copy(selected: &SelectionDebugEntry) -> Element {
    rsx! {
        {detail_panel_selected_title()}
        {detail_panel_selected_label(&selected.label)}
        {detail_panel_selected_subtitle(&selected.subtitle)}
        {detail_panel_selected_value(&selected.detail)}
    }
}

fn detail_panel_selected_title() -> Element {
    rsx! {
        fontstring {
            name: "SelectionDebugDetailTitle",
            width: 520.0,
            height: 28.0,
            text: "Selected Candidate",
            font: GameFont::FrizQuadrata,
            font_size: 20.0,
            font_color: TEXT_GOLD,
            justify_h: JustifyH::Left,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_to: DETAIL_PANEL,
                relative_point: AnchorPoint::TopLeft,
                x: "24",
                y: "-18",
            }
        }
    }
}

fn detail_panel_selected_label(label: &str) -> Element {
    rsx! {
        fontstring {
            name: "SelectionDebugDetailLabel",
            width: 520.0,
            height: 32.0,
            text: {label},
            font: GameFont::FrizQuadrata,
            font_size: 28.0,
            font_color: TEXT_GOLD,
            justify_h: JustifyH::Left,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_to: DETAIL_PANEL,
                relative_point: AnchorPoint::TopLeft,
                x: "24",
                y: "-62",
            }
        }
    }
}

fn detail_panel_selected_subtitle(subtitle: &str) -> Element {
    rsx! {
        fontstring {
            name: "SelectionDebugDetailSubtitle",
            width: 520.0,
            height: 18.0,
            text: {subtitle},
            font: GameFont::FrizQuadrata,
            font_size: 14.0,
            font_color: TEXT_SUBTITLE,
            justify_h: JustifyH::Left,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_to: DETAIL_PANEL,
                relative_point: AnchorPoint::TopLeft,
                x: "24",
                y: "-96",
            }
        }
    }
}

fn detail_panel_selected_value(detail: &str) -> Element {
    rsx! {
        fontstring {
            name: "SelectionDebugDetailValue",
            width: 520.0,
            height: 150.0,
            text: {detail},
            font: GameFont::FrizQuadrata,
            font_size: 15.0,
            font_color: TEXT_MUTED,
            justify_h: JustifyH::Left,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_to: DETAIL_PANEL,
                relative_point: AnchorPoint::TopLeft,
                x: "24",
                y: "-136",
            }
        }
    }
}

fn detail_panel_mode_copy(state: &SelectionDebugState) -> Element {
    let mode = if state.pinned {
        "Pinned mode keeps the current row selected while you inspect detail output."
    } else {
        "Live mode mirrors quick candidate changes so hover and keyboard traversal stay easy to inspect."
    };

    rsx! {
        fontstring {
            name: "SelectionDebugModeTitle",
            width: 520.0,
            height: 20.0,
            text: "Selection Mode",
            font: GameFont::FrizQuadrata,
            font_size: 17.0,
            font_color: TEXT_GOLD,
            justify_h: JustifyH::Left,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_to: DETAIL_PANEL,
                relative_point: AnchorPoint::TopLeft,
                x: "24",
                y: "-302",
            }
        }
        fontstring {
            name: "SelectionDebugModeValue",
            width: 520.0,
            height: 70.0,
            text: mode,
            font: GameFont::FrizQuadrata,
            font_size: 14.0,
            font_color: TEXT_SUBTITLE,
            justify_h: JustifyH::Left,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_to: DETAIL_PANEL,
                relative_point: AnchorPoint::TopLeft,
                x: "24",
                y: "-330",
            }
        }
        fontstring {
            name: "SelectionDebugLastActionTitle",
            width: 520.0,
            height: 20.0,
            text: "Last Action",
            font: GameFont::FrizQuadrata,
            font_size: 17.0,
            font_color: TEXT_GOLD,
            justify_h: JustifyH::Left,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_to: DETAIL_PANEL,
                relative_point: AnchorPoint::TopLeft,
                x: "24",
                y: "-404",
            }
        }
        fontstring {
            name: "SelectionDebugLastActionValue",
            width: 520.0,
            height: 24.0,
            text: {&state.last_action},
            font: GameFont::FrizQuadrata,
            font_size: 14.0,
            font_color: TEXT_SUBTITLE,
            justify_h: JustifyH::Left,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_to: DETAIL_PANEL,
                relative_point: AnchorPoint::TopLeft,
                x: "24",
                y: "-432",
            }
        }
    }
}

fn action_button(
    name: FrameName,
    text: &str,
    action: SelectionDebugAction,
    relative_to: FrameName,
    x: f32,
) -> Element {
    rsx! {
        button {
            name,
            width: 164.0,
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
                relative_to,
                relative_point: AnchorPoint::BottomLeft,
                x: {x.to_string()},
                y: "26",
            }
        }
    }
}

fn status_text(state: &SelectionDebugState) -> Element {
    let mode = if state.pinned { "Pinned" } else { "Live" };
    let text = format!(
        "{mode} selection · {} candidates · active row {}",
        state.entries.len(),
        state.selected_index.saturating_add(1)
    );
    rsx! {
        fontstring {
            name: STATUS_TEXT,
            width: 680.0,
            height: 24.0,
            text: {text},
            font: GameFont::FrizQuadrata,
            font_size: 14.0,
            font_color: TEXT_SUBTITLE,
            justify_h: JustifyH::Center,
            anchor {
                point: AnchorPoint::Bottom,
                relative_to: SCREEN_MOUNT,
                relative_point: AnchorPoint::Bottom,
                y: "78",
            }
        }
    }
}

pub fn selection_debug_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<SelectionDebugState>()
        .expect("SelectionDebugState must be present");

    rsx! {
        r#frame {
            name: SELECTION_DEBUG_ROOT,
            stretch: true,
            background_color: "0.01,0.01,0.02,1.0",
            strata: FrameStrata::Background,
            r#frame {
                name: SCREEN_MOUNT,
                width: 1120.0,
                height: 640.0,
                anchor {
                    point: AnchorPoint::Center,
                    relative_point: AnchorPoint::Center,
                }
                {framed_title(
                    TITLE_FRAME,
                    TITLE_LABEL,
                    SCREEN_MOUNT,
                    420.0,
                    "Selection Debug",
                )}
                {helper_text()}
                {list_panel(&state.entries, state.selected_index)}
                {detail_panel(state)}
                {action_button(PREV_BUTTON, "Previous", SelectionDebugAction::Prev, SCREEN_MOUNT, 32.0)}
                {action_button(NEXT_BUTTON, "Next", SelectionDebugAction::Next, SCREEN_MOUNT, 216.0)}
                {action_button(
                    PIN_BUTTON,
                    if state.pinned { "Unpin" } else { "Pin Selection" },
                    SelectionDebugAction::TogglePinned,
                    SCREEN_MOUNT,
                    400.0,
                )}
                {action_button(BACK_BUTTON, "Back", SelectionDebugAction::Back, SCREEN_MOUNT, 924.0)}
                {status_text(state)}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_toolkit::screen::Screen;

    use crate::ui::frame::WidgetData;
    use crate::ui::registry::FrameRegistry;

    fn sample_state() -> SelectionDebugState {
        SelectionDebugState {
            entries: vec![
                SelectionDebugEntry {
                    label: "Local Player".to_string(),
                    subtitle: "Self-target and fallback selection".to_string(),
                    detail: "Use this row to validate self-target visuals and keyboard traversal."
                        .to_string(),
                },
                SelectionDebugEntry {
                    label: "Quest NPC".to_string(),
                    subtitle: "Friendly unit with interaction affordances".to_string(),
                    detail: "This variant is useful when checking hover, click and nameplate sync."
                        .to_string(),
                },
            ],
            selected_index: 1,
            pinned: true,
            last_action: "Pinned Quest NPC".to_string(),
        }
    }

    #[test]
    fn screen_renders_selected_entry_and_controls() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(sample_state());
        Screen::new(selection_debug_screen).sync(&shared, &mut reg);

        assert!(reg.get_by_name(SELECTION_DEBUG_ROOT.0).is_some());
        assert!(reg.get_by_name(PREV_BUTTON.0).is_some());
        assert!(reg.get_by_name(NEXT_BUTTON.0).is_some());
        assert!(reg.get_by_name(PIN_BUTTON.0).is_some());

        let selected = reg
            .get_by_name("SelectionDebugRow_1Selected")
            .expect("selected row accent");
        let selected = reg.get(selected).expect("selected row accent frame");
        assert!(!selected.hidden);

        let detail = reg
            .get_by_name("SelectionDebugDetailValue")
            .expect("detail value");
        let detail = reg.get(detail).expect("detail value frame");
        let Some(WidgetData::FontString(detail)) = detail.widget_data.as_ref() else {
            panic!("SelectionDebugDetailValue should be a font string");
        };
        assert_eq!(
            detail.text.as_str(),
            "This variant is useful when checking hover, click and nameplate sync."
        );
    }
}
