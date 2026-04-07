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

// --- Layout constants ---

pub const FRAME_W: f32 = 700.0;
pub const FRAME_H: f32 = 500.0;
const HEADER_H: f32 = 28.0;
const INSET: f32 = 8.0;
const CONTENT_TOP: f32 = HEADER_H + 4.0;

const LIST_W: f32 = 240.0;
const LIST_GAP: f32 = 6.0;
const DETAIL_INSET: f32 = LIST_W + LIST_GAP + INSET;

const ZONE_HEADER_H: f32 = 22.0;
const QUEST_ROW_H: f32 = 20.0;
const ROW_GAP: f32 = 2.0;

const DETAIL_TITLE_H: f32 = 24.0;
const DETAIL_DESC_H: f32 = 80.0;
const DETAIL_OBJ_ROW_H: f32 = 18.0;
const DETAIL_OBJ_GAP: f32 = 2.0;
const DETAIL_SECTION_GAP: f32 = 12.0;

// --- Colors ---

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const LIST_BG: &str = "0.0,0.0,0.0,0.3";
const DETAIL_BG: &str = "0.0,0.0,0.0,0.3";
const ZONE_HEADER_BG: &str = "0.12,0.10,0.06,0.9";
const ZONE_HEADER_COLOR: &str = "1.0,0.82,0.0,1.0";
const QUEST_SELECTED_BG: &str = "0.2,0.15,0.05,0.95";
const QUEST_NORMAL_COLOR: &str = "1.0,1.0,1.0,1.0";
const QUEST_COMPLETE_COLOR: &str = "0.5,0.5,0.5,1.0";
const QUEST_SELECTED_COLOR: &str = "1.0,0.82,0.0,1.0";
const DETAIL_TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const DETAIL_DESC_COLOR: &str = "0.85,0.85,0.85,1.0";
const OBJ_INCOMPLETE_COLOR: &str = "1.0,1.0,1.0,1.0";
const OBJ_COMPLETE_COLOR: &str = "0.5,0.5,0.5,1.0";
const LEVEL_COLOR: &str = "0.7,0.7,0.7,1.0";

// --- Data types ---

#[derive(Clone, Debug, PartialEq)]
pub struct QuestLogObjective {
    pub text: String,
    pub current: u32,
    pub required: u32,
}

impl QuestLogObjective {
    pub fn is_complete(&self) -> bool {
        self.current >= self.required
    }

    pub fn display_text(&self) -> String {
        if self.required <= 1 {
            self.text.clone()
        } else {
            format!("{}: {}/{}", self.text, self.current, self.required)
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuestLogEntry {
    pub quest_id: u32,
    pub title: String,
    pub level: u32,
    pub zone: String,
    pub description: String,
    pub objectives: Vec<QuestLogObjective>,
    pub selected: bool,
}

impl QuestLogEntry {
    pub fn is_complete(&self) -> bool {
        !self.objectives.is_empty() && self.objectives.iter().all(|o| o.is_complete())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuestLogFrameState {
    pub visible: bool,
    pub quests: Vec<QuestLogEntry>,
}

impl Default for QuestLogFrameState {
    fn default() -> Self {
        Self {
            visible: false,
            quests: vec![],
        }
    }
}

// --- Screen entry ---

pub fn quest_log_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<QuestLogFrameState>()
        .expect("QuestLogFrameState must be in SharedContext");
    let hide = !state.visible;
    let selected = state.quests.iter().find(|q| q.selected);
    rsx! {
        r#frame {
            name: "QuestLogFrame",
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
            {quest_list(&state.quests)}
            {detail_panel(selected)}
        }
    }
}

// --- Title ---

fn title_bar() -> Element {
    rsx! {
        fontstring {
            name: "QuestLogFrameTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: "Quest Log",
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

// --- Quest list (left panel, grouped by zone) ---

/// Compute the y-offset for each row in the zone-grouped quest list.
/// Returns (group_idx, zone_name, quests_with_indices, zone_header_y) tuples.
fn zone_row_positions(
    groups: &[(String, Vec<&QuestLogEntry>)],
) -> Vec<(usize, f32, Vec<(usize, f32)>)> {
    let mut y: f32 = 0.0;
    groups
        .iter()
        .enumerate()
        .map(|(gi, (_zone, zone_quests))| {
            let header_y = y;
            y += ZONE_HEADER_H + ROW_GAP;
            let quest_positions: Vec<(usize, f32)> = zone_quests
                .iter()
                .enumerate()
                .map(|(qi, _)| {
                    let qy = y;
                    y += QUEST_ROW_H + ROW_GAP;
                    (qi, qy)
                })
                .collect();
            (gi, header_y, quest_positions)
        })
        .collect()
}

fn quest_list(quests: &[QuestLogEntry]) -> Element {
    let list_h = FRAME_H - CONTENT_TOP - INSET;
    let list_y = -CONTENT_TOP;
    let groups = group_by_zone(quests);
    let positions = zone_row_positions(&groups);
    let rows: Element = positions
        .iter()
        .flat_map(|(gi, header_y, quest_positions)| {
            let (zone, zone_quests) = &groups[*gi];
            let mut elems = zone_header(*gi, zone, *header_y);
            for &(qi, qy) in quest_positions {
                elems.extend(quest_row(*gi, qi, zone_quests[qi], qy));
            }
            elems
        })
        .collect();
    rsx! {
        r#frame {
            name: "QuestLogList",
            width: {LIST_W},
            height: {list_h},
            background_color: LIST_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {INSET},
                y: {list_y},
            }
            {rows}
        }
    }
}

fn group_by_zone(quests: &[QuestLogEntry]) -> Vec<(String, Vec<&QuestLogEntry>)> {
    let mut groups: Vec<(String, Vec<&QuestLogEntry>)> = Vec::new();
    for q in quests {
        if let Some(g) = groups.iter_mut().find(|(z, _)| *z == q.zone) {
            g.1.push(q);
        } else {
            groups.push((q.zone.clone(), vec![q]));
        }
    }
    groups
}

fn zone_header(group_idx: usize, zone: &str, y: f32) -> Element {
    let id = DynName(format!("QuestLogZone{group_idx}"));
    let label_id = DynName(format!("QuestLogZone{group_idx}Label"));
    rsx! {
        r#frame {
            name: id,
            width: {LIST_W - 4.0},
            height: {ZONE_HEADER_H},
            background_color: ZONE_HEADER_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "2",
                y: {-y},
            }
            fontstring {
                name: label_id,
                width: {LIST_W - 12.0},
                height: {ZONE_HEADER_H},
                text: zone,
                font_size: 11.0,
                font_color: ZONE_HEADER_COLOR,
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

fn quest_row(group_idx: usize, quest_idx: usize, quest: &QuestLogEntry, y: f32) -> Element {
    let row_id = DynName(format!("QuestLogRow{group_idx}_{quest_idx}"));
    let label_id = DynName(format!("QuestLogRow{group_idx}_{quest_idx}Label"));
    let level_id = DynName(format!("QuestLogRow{group_idx}_{quest_idx}Level"));
    let level_text = format!("[{}]", quest.level);
    let color = if quest.selected {
        QUEST_SELECTED_COLOR
    } else if quest.is_complete() {
        QUEST_COMPLETE_COLOR
    } else {
        QUEST_NORMAL_COLOR
    };
    let bg = if quest.selected {
        QUEST_SELECTED_BG
    } else {
        "0.0,0.0,0.0,0.0"
    };
    rsx! {
        r#frame {
            name: row_id,
            width: {LIST_W - 4.0},
            height: {QUEST_ROW_H},
            background_color: bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "2",
                y: {-y},
            }
            fontstring {
                name: level_id,
                width: 30.0,
                height: {QUEST_ROW_H},
                text: {level_text.as_str()},
                font_size: 10.0,
                font_color: LEVEL_COLOR,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "4",
                    y: "0",
                }
            }
            fontstring {
                name: label_id,
                width: {LIST_W - 42.0},
                height: {QUEST_ROW_H},
                text: {quest.title.as_str()},
                font_size: 10.0,
                font_color: color,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "34",
                    y: "0",
                }
            }
        }
    }
}

// --- Detail panel (right side) ---

fn detail_panel(selected: Option<&QuestLogEntry>) -> Element {
    let detail_x = DETAIL_INSET;
    let detail_y = -CONTENT_TOP;
    let detail_w = FRAME_W - DETAIL_INSET - INSET;
    let detail_h = FRAME_H - CONTENT_TOP - INSET;
    let content: Element = match selected {
        Some(quest) => detail_content(quest, detail_w),
        None => empty_detail(detail_w),
    };
    rsx! {
        r#frame {
            name: "QuestLogDetail",
            width: {detail_w},
            height: {detail_h},
            background_color: DETAIL_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {detail_x},
                y: {detail_y},
            }
            {content}
        }
    }
}

fn empty_detail(w: f32) -> Element {
    rsx! {
        fontstring {
            name: "QuestLogDetailEmpty",
            width: {w},
            height: 20.0,
            text: "Select a quest to view details",
            font_size: 11.0,
            font_color: LEVEL_COLOR,
            justify_h: "CENTER",
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
                x: "0",
                y: "0",
            }
        }
    }
}

fn detail_content(quest: &QuestLogEntry, w: f32) -> Element {
    let inner_w = w - 16.0;
    let title_y: f32 = 8.0;
    let desc_y = title_y + DETAIL_TITLE_H + DETAIL_SECTION_GAP;
    let obj_y = desc_y + DETAIL_DESC_H + DETAIL_SECTION_GAP;
    rsx! {
        {detail_title(quest, inner_w, title_y)}
        {detail_description(&quest.description, inner_w, desc_y)}
        {detail_objectives(&quest.objectives, inner_w, obj_y)}
    }
}

fn detail_title(quest: &QuestLogEntry, w: f32, y: f32) -> Element {
    rsx! {
        fontstring {
            name: "QuestLogDetailTitle",
            width: {w},
            height: {DETAIL_TITLE_H},
            text: {quest.title.as_str()},
            font_size: 14.0,
            font_color: DETAIL_TITLE_COLOR,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "8",
                y: {-y},
            }
        }
    }
}

fn detail_description(description: &str, w: f32, y: f32) -> Element {
    rsx! {
        fontstring {
            name: "QuestLogDetailDesc",
            width: {w},
            height: {DETAIL_DESC_H},
            text: description,
            font_size: 11.0,
            font_color: DETAIL_DESC_COLOR,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "8",
                y: {-y},
            }
        }
    }
}

fn detail_objectives(objectives: &[QuestLogObjective], w: f32, y: f32) -> Element {
    let header_y = y;
    let rows: Element = objectives
        .iter()
        .enumerate()
        .flat_map(|(i, obj)| {
            let obj_y = header_y
                + DETAIL_OBJ_ROW_H
                + DETAIL_OBJ_GAP
                + i as f32 * (DETAIL_OBJ_ROW_H + DETAIL_OBJ_GAP);
            objective_row(i, obj, w, obj_y)
        })
        .collect();
    rsx! {
        fontstring {
            name: "QuestLogDetailObjHeader",
            width: {w},
            height: {DETAIL_OBJ_ROW_H},
            text: "Objectives",
            font_size: 12.0,
            font_color: DETAIL_TITLE_COLOR,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "8",
                y: {-header_y},
            }
        }
        {rows}
    }
}

fn objective_row(idx: usize, obj: &QuestLogObjective, w: f32, y: f32) -> Element {
    let id = DynName(format!("QuestLogObj{idx}"));
    let color = if obj.is_complete() {
        OBJ_COMPLETE_COLOR
    } else {
        OBJ_INCOMPLETE_COLOR
    };
    let text = obj.display_text();
    rsx! {
        fontstring {
            name: id,
            width: {w},
            height: {DETAIL_OBJ_ROW_H},
            text: {text.as_str()},
            font_size: 10.0,
            font_color: color,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "16",
                y: {-y},
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

    fn sample_quests() -> Vec<QuestLogEntry> {
        vec![
            QuestLogEntry {
                quest_id: 101,
                title: "The Fallen Outpost".into(),
                level: 25,
                zone: "Stonetalon Mountains".into(),
                description: "Investigate the ruins of the fallen outpost.".into(),
                objectives: vec![
                    QuestLogObjective {
                        text: "Investigate ruins".into(),
                        current: 0,
                        required: 1,
                    },
                    QuestLogObjective {
                        text: "Defeat guardians".into(),
                        current: 2,
                        required: 5,
                    },
                ],
                selected: true,
            },
            QuestLogEntry {
                quest_id: 102,
                title: "Supplies for the Front".into(),
                level: 26,
                zone: "Stonetalon Mountains".into(),
                description: "Gather supplies from the nearby camps.".into(),
                objectives: vec![QuestLogObjective {
                    text: "Gather supplies".into(),
                    current: 8,
                    required: 8,
                }],
                selected: false,
            },
            QuestLogEntry {
                quest_id: 201,
                title: "Ancient Spirits".into(),
                level: 30,
                zone: "Desolace".into(),
                description: "Commune with the ancient spirits of Desolace.".into(),
                objectives: vec![QuestLogObjective {
                    text: "Commune with spirits".into(),
                    current: 1,
                    required: 3,
                }],
                selected: false,
            },
        ]
    }

    fn build_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(QuestLogFrameState {
            visible: true,
            quests: sample_quests(),
        });
        Screen::new(quest_log_frame_screen).sync(&shared, &mut reg);
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
        assert!(reg.get_by_name("QuestLogFrame").is_some());
        assert!(reg.get_by_name("QuestLogFrameTitle").is_some());
    }

    #[test]
    fn builds_list_and_detail_panels() {
        let reg = build_registry();
        assert!(reg.get_by_name("QuestLogList").is_some());
        assert!(reg.get_by_name("QuestLogDetail").is_some());
    }

    #[test]
    fn builds_zone_headers() {
        let reg = build_registry();
        assert!(reg.get_by_name("QuestLogZone0").is_some());
        assert!(reg.get_by_name("QuestLogZone0Label").is_some());
        assert!(reg.get_by_name("QuestLogZone1").is_some());
        assert!(reg.get_by_name("QuestLogZone1Label").is_some());
    }

    #[test]
    fn builds_quest_rows() {
        let reg = build_registry();
        // Zone 0 has 2 quests, zone 1 has 1
        assert!(reg.get_by_name("QuestLogRow0_0").is_some());
        assert!(reg.get_by_name("QuestLogRow0_0Label").is_some());
        assert!(reg.get_by_name("QuestLogRow0_0Level").is_some());
        assert!(reg.get_by_name("QuestLogRow0_1").is_some());
        assert!(reg.get_by_name("QuestLogRow1_0").is_some());
    }

    #[test]
    fn builds_detail_content_for_selected() {
        let reg = build_registry();
        assert!(reg.get_by_name("QuestLogDetailTitle").is_some());
        assert!(reg.get_by_name("QuestLogDetailDesc").is_some());
        assert!(reg.get_by_name("QuestLogDetailObjHeader").is_some());
        assert!(reg.get_by_name("QuestLogObj0").is_some());
        assert!(reg.get_by_name("QuestLogObj1").is_some());
    }

    #[test]
    fn hidden_when_not_visible() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(QuestLogFrameState::default());
        Screen::new(quest_log_frame_screen).sync(&shared, &mut reg);
        let id = reg.get_by_name("QuestLogFrame").expect("frame");
        assert!(reg.get(id).expect("data").hidden);
    }

    #[test]
    fn empty_detail_when_no_selection() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        let mut quests = sample_quests();
        for q in &mut quests {
            q.selected = false;
        }
        shared.insert(QuestLogFrameState {
            visible: true,
            quests,
        });
        Screen::new(quest_log_frame_screen).sync(&shared, &mut reg);
        assert!(reg.get_by_name("QuestLogDetailEmpty").is_some());
        assert!(reg.get_by_name("QuestLogDetailTitle").is_none());
    }

    // --- Coord validation ---

    #[test]
    fn coord_main_frame_centered() {
        let reg = layout_registry();
        let r = rect(&reg, "QuestLogFrame");
        let expected_x = (1920.0 - FRAME_W) / 2.0;
        let expected_y = (1080.0 - FRAME_H) / 2.0;
        assert!((r.x - expected_x).abs() < 1.0);
        assert!((r.y - expected_y).abs() < 1.0);
        assert!((r.width - FRAME_W).abs() < 1.0);
        assert!((r.height - FRAME_H).abs() < 1.0);
    }

    #[test]
    fn coord_list_panel() {
        let reg = layout_registry();
        let frame_r = rect(&reg, "QuestLogFrame");
        let r = rect(&reg, "QuestLogList");
        assert!((r.x - (frame_r.x + INSET)).abs() < 1.0);
        assert!((r.width - LIST_W).abs() < 1.0);
    }

    #[test]
    fn coord_detail_panel() {
        let reg = layout_registry();
        let frame_r = rect(&reg, "QuestLogFrame");
        let r = rect(&reg, "QuestLogDetail");
        let expected_x = frame_r.x + DETAIL_INSET;
        let expected_w = FRAME_W - DETAIL_INSET - INSET;
        assert!((r.x - expected_x).abs() < 1.0);
        assert!((r.width - expected_w).abs() < 1.0);
    }

    // --- Data model tests ---

    #[test]
    fn objective_completion() {
        let done = QuestLogObjective {
            text: "Kill mobs".into(),
            current: 5,
            required: 5,
        };
        assert!(done.is_complete());
        let partial = QuestLogObjective {
            text: "Kill mobs".into(),
            current: 2,
            required: 5,
        };
        assert!(!partial.is_complete());
    }

    #[test]
    fn objective_display_text() {
        let counted = QuestLogObjective {
            text: "Kill mobs".into(),
            current: 2,
            required: 5,
        };
        assert_eq!(counted.display_text(), "Kill mobs: 2/5");
        let single = QuestLogObjective {
            text: "Talk to NPC".into(),
            current: 0,
            required: 1,
        };
        assert_eq!(single.display_text(), "Talk to NPC");
    }

    #[test]
    fn quest_complete_requires_all_objectives() {
        let incomplete = &sample_quests()[0];
        assert!(!incomplete.is_complete());
        let complete = &sample_quests()[1];
        assert!(complete.is_complete());
    }

    #[test]
    fn group_by_zone_preserves_order() {
        let quests = sample_quests();
        let groups = group_by_zone(&quests);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].0, "Stonetalon Mountains");
        assert_eq!(groups[0].1.len(), 2);
        assert_eq!(groups[1].0, "Desolace");
        assert_eq!(groups[1].1.len(), 1);
    }
}
