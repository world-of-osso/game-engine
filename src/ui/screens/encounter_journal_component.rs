use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;
use crate::ui::strata::FrameStrata;

struct DynName(String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub const FRAME_W: f32 = 750.0;
pub const FRAME_H: f32 = 500.0;
const HEADER_H: f32 = 28.0;
const SIDEBAR_W: f32 = 200.0;
const SIDEBAR_INSET: f32 = 8.0;
const TAB_H: f32 = 26.0;
const TAB_GAP: f32 = 2.0;
const INSTANCE_ROW_H: f32 = 24.0;
const INSTANCE_ROW_GAP: f32 = 1.0;
const CONTENT_GAP: f32 = 4.0;
const CONTENT_TOP: f32 = HEADER_H + TAB_GAP;

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const SIDEBAR_BG: &str = "0.0,0.0,0.0,0.4";
const TAB_BG_ACTIVE: &str = "0.2,0.15,0.05,0.95";
const TAB_BG_INACTIVE: &str = "0.08,0.07,0.06,0.88";
const TAB_TEXT_ACTIVE: &str = "1.0,0.82,0.0,1.0";
const TAB_TEXT_INACTIVE: &str = "0.6,0.6,0.6,1.0";
const INSTANCE_SELECTED_BG: &str = "0.2,0.15,0.05,0.95";
const INSTANCE_NORMAL_BG: &str = "0.0,0.0,0.0,0.0";
const INSTANCE_SELECTED_COLOR: &str = "1.0,0.82,0.0,1.0";
const INSTANCE_NORMAL_COLOR: &str = "1.0,1.0,1.0,1.0";
const CONTENT_BG: &str = "0.0,0.0,0.0,0.3";

pub const MAX_INSTANCES: usize = 15;

#[derive(Clone, Debug, PartialEq)]
pub struct EJTab {
    pub name: String,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InstanceEntry {
    pub name: String,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EncounterJournalState {
    pub visible: bool,
    pub tabs: Vec<EJTab>,
    pub instances: Vec<InstanceEntry>,
}

impl Default for EncounterJournalState {
    fn default() -> Self {
        Self {
            visible: false,
            tabs: vec![
                EJTab {
                    name: "Dungeons".into(),
                    active: true,
                },
                EJTab {
                    name: "Raids".into(),
                    active: false,
                },
                EJTab {
                    name: "Tier".into(),
                    active: false,
                },
            ],
            instances: vec![],
        }
    }
}

pub fn encounter_journal_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<EncounterJournalState>()
        .expect("EncounterJournalState must be in SharedContext");
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: "EncounterJournal",
            width: {FRAME_W},
            height: {FRAME_H},
            strata: FrameStrata::Dialog,
            hidden: hide,
            background_color: FRAME_BG,
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
                x: "0",
                y: "0",
            }
            {title_bar()}
            {sidebar_tabs(&state.tabs)}
            {instance_list(&state.instances)}
            {content_area()}
        }
    }
}

fn title_bar() -> Element {
    rsx! {
        fontstring {
            name: "EncounterJournalTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: "Encounter Journal",
            font_size: 16.0,
            font_color: TITLE_COLOR,
            justify_h: "CENTER",
            anchor {
                point: AnchorPoint::Top,
                relative_point: AnchorPoint::Top,
                x: "0",
                y: "0",
            }
        }
    }
}

fn sidebar_tabs(tabs: &[EJTab]) -> Element {
    let tab_w = SIDEBAR_W;
    tabs.iter()
        .enumerate()
        .flat_map(|(i, tab)| {
            let y = -(HEADER_H + i as f32 * (TAB_H + TAB_GAP));
            sidebar_tab(i, tab, tab_w, y)
        })
        .collect()
}

fn sidebar_tab(i: usize, tab: &EJTab, tab_w: f32, y: f32) -> Element {
    let tab_id = DynName(format!("EJTab{i}"));
    let label_id = DynName(format!("EJTab{i}Label"));
    let bg = if tab.active {
        TAB_BG_ACTIVE
    } else {
        TAB_BG_INACTIVE
    };
    let color = if tab.active {
        TAB_TEXT_ACTIVE
    } else {
        TAB_TEXT_INACTIVE
    };
    rsx! {
        r#frame {
            name: tab_id,
            width: {tab_w},
            height: {TAB_H},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {SIDEBAR_INSET},
                y: {y},
            }
            fontstring {
                name: label_id,
                width: {tab_w},
                height: {TAB_H},
                text: {tab.name.as_str()},
                font_size: 11.0,
                font_color: color,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
        }
    }
}

fn instance_list(instances: &[InstanceEntry]) -> Element {
    let tabs_h = 3.0 * (TAB_H + TAB_GAP);
    let list_y = -(HEADER_H + tabs_h + CONTENT_GAP);
    let list_h = FRAME_H - HEADER_H - tabs_h - CONTENT_GAP - SIDEBAR_INSET;
    let rows: Element = instances
        .iter()
        .enumerate()
        .take(MAX_INSTANCES)
        .flat_map(|(i, inst)| instance_row(i, inst))
        .collect();
    rsx! {
        r#frame {
            name: "EJInstanceList",
            width: {SIDEBAR_W},
            height: {list_h},
            background_color: SIDEBAR_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {SIDEBAR_INSET},
                y: {list_y},
            }
            {rows}
        }
    }
}

fn instance_row(idx: usize, inst: &InstanceEntry) -> Element {
    let row_id = DynName(format!("EJInstance{idx}"));
    let label_id = DynName(format!("EJInstance{idx}Label"));
    let bg = if inst.selected {
        INSTANCE_SELECTED_BG
    } else {
        INSTANCE_NORMAL_BG
    };
    let color = if inst.selected {
        INSTANCE_SELECTED_COLOR
    } else {
        INSTANCE_NORMAL_COLOR
    };
    let y = -(idx as f32 * (INSTANCE_ROW_H + INSTANCE_ROW_GAP));
    rsx! {
        r#frame {
            name: row_id,
            width: {SIDEBAR_W},
            height: {INSTANCE_ROW_H},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "0",
                y: {y},
            }
            fontstring {
                name: label_id,
                width: {SIDEBAR_W - 8.0},
                height: {INSTANCE_ROW_H},
                text: {inst.name.as_str()},
                font_size: 10.0,
                font_color: color,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "4",
                    y: "0",
                }
            }
        }
    }
}

fn content_area() -> Element {
    let content_x = SIDEBAR_INSET + SIDEBAR_W + CONTENT_GAP;
    let content_y = -CONTENT_TOP;
    let content_w = FRAME_W - content_x - SIDEBAR_INSET;
    let content_h = FRAME_H - CONTENT_TOP - SIDEBAR_INSET;
    rsx! {
        r#frame {
            name: "EJContentArea",
            width: {content_w},
            height: {content_h},
            background_color: CONTENT_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {content_x},
                y: {content_y},
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_toolkit::layout::{LayoutRect, recompute_layouts};
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};

    fn make_test_state() -> EncounterJournalState {
        EncounterJournalState {
            visible: true,
            instances: vec![
                InstanceEntry {
                    name: "Deadmines".into(),
                    selected: true,
                },
                InstanceEntry {
                    name: "Shadowfang Keep".into(),
                    selected: false,
                },
                InstanceEntry {
                    name: "Blackfathom Deeps".into(),
                    selected: false,
                },
            ],
            ..Default::default()
        }
    }

    fn build_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        Screen::new(encounter_journal_screen).sync(&shared, &mut reg);
        reg
    }

    fn layout_registry() -> FrameRegistry {
        let mut reg = build_registry();
        recompute_layouts(&mut reg);
        reg
    }

    fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
        reg.get(reg.get_by_name(name).expect(name))
            .and_then(|f| f.layout_rect.clone())
            .unwrap_or_else(|| panic!("{name} has no layout_rect"))
    }

    #[test]
    fn builds_frame_and_title() {
        let reg = build_registry();
        assert!(reg.get_by_name("EncounterJournal").is_some());
        assert!(reg.get_by_name("EncounterJournalTitle").is_some());
    }

    #[test]
    fn builds_three_tabs() {
        let reg = build_registry();
        for i in 0..3 {
            assert!(
                reg.get_by_name(&format!("EJTab{i}")).is_some(),
                "EJTab{i} missing"
            );
        }
    }

    #[test]
    fn builds_instance_list() {
        let reg = build_registry();
        assert!(reg.get_by_name("EJInstanceList").is_some());
        for i in 0..3 {
            assert!(
                reg.get_by_name(&format!("EJInstance{i}")).is_some(),
                "EJInstance{i} missing"
            );
        }
    }

    #[test]
    fn builds_content_area() {
        let reg = build_registry();
        assert!(reg.get_by_name("EJContentArea").is_some());
    }

    #[test]
    fn hidden_when_not_visible() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(EncounterJournalState::default());
        Screen::new(encounter_journal_screen).sync(&shared, &mut reg);
        let id = reg.get_by_name("EncounterJournal").expect("frame");
        assert!(reg.get(id).expect("data").hidden);
    }

    // --- Coord validation ---

    #[test]
    fn coord_main_frame_centered() {
        let reg = layout_registry();
        let r = rect(&reg, "EncounterJournal");
        let expected_x = (1920.0 - FRAME_W) / 2.0;
        let expected_y = (1080.0 - FRAME_H) / 2.0;
        assert!((r.x - expected_x).abs() < 1.0);
        assert!((r.y - expected_y).abs() < 1.0);
        assert!((r.width - FRAME_W).abs() < 1.0);
    }

    #[test]
    fn coord_first_tab() {
        let reg = layout_registry();
        let frame_x = (1920.0 - FRAME_W) / 2.0;
        let frame_y = (1080.0 - FRAME_H) / 2.0;
        let r = rect(&reg, "EJTab0");
        assert!((r.x - (frame_x + SIDEBAR_INSET)).abs() < 1.0);
        assert!((r.y - (frame_y + HEADER_H)).abs() < 1.0);
        assert!((r.width - SIDEBAR_W).abs() < 1.0);
    }

    #[test]
    fn coord_content_area() {
        let reg = layout_registry();
        let frame_x = (1920.0 - FRAME_W) / 2.0;
        let frame_y = (1080.0 - FRAME_H) / 2.0;
        let r = rect(&reg, "EJContentArea");
        let expected_x = frame_x + SIDEBAR_INSET + SIDEBAR_W + CONTENT_GAP;
        assert!((r.x - expected_x).abs() < 1.0);
        assert!((r.y - (frame_y + CONTENT_TOP)).abs() < 1.0);
    }
}
