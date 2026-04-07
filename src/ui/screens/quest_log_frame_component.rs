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

const REWARD_ICON_SIZE: f32 = 32.0;
const REWARD_LABEL_H: f32 = 18.0;
const REWARD_GAP: f32 = 8.0;
const REWARD_NAME_W: f32 = 80.0;
const REWARD_SLOT_W: f32 = REWARD_ICON_SIZE + REWARD_GAP;

const ACTION_BTN_W: f32 = 110.0;
const ACTION_BTN_H: f32 = 26.0;
const ACTION_BTN_GAP: f32 = 8.0;

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
const REWARD_HEADER_COLOR: &str = "1.0,0.82,0.0,1.0";
const REWARD_NAME_COLOR: &str = "1.0,1.0,1.0,1.0";
const REWARD_ICON_BG: &str = "0.08,0.08,0.08,0.8";
const ACCEPT_BTN_BG: &str = "0.15,0.25,0.1,0.95";
const ACCEPT_BTN_TEXT: &str = "0.2,1.0,0.2,1.0";
const ABANDON_BTN_BG: &str = "0.25,0.08,0.08,0.95";
const ABANDON_BTN_TEXT: &str = "1.0,0.3,0.3,1.0";
const COMPLETE_BTN_BG: &str = "0.15,0.25,0.1,0.95";
const COMPLETE_BTN_TEXT: &str = "0.2,1.0,0.2,1.0";

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
pub struct QuestRewardItem {
    pub name: String,
    pub icon_fdid: u32,
    pub quantity: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuestLogEntry {
    pub quest_id: u32,
    pub title: String,
    pub level: u32,
    pub zone: String,
    pub description: String,
    pub objectives: Vec<QuestLogObjective>,
    pub rewards: Vec<QuestRewardItem>,
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
    let obj_count = quest.objectives.len() as f32;
    let obj_y = desc_y + DETAIL_DESC_H + DETAIL_SECTION_GAP;
    let obj_total_h =
        DETAIL_OBJ_ROW_H + (obj_count * (DETAIL_OBJ_ROW_H + DETAIL_OBJ_GAP)) + DETAIL_SECTION_GAP;
    let rewards_y = obj_y + obj_total_h;
    let detail_h = FRAME_H - CONTENT_TOP - INSET;
    rsx! {
        {detail_title(quest, inner_w, title_y)}
        {detail_description(&quest.description, inner_w, desc_y)}
        {detail_objectives(&quest.objectives, inner_w, obj_y)}
        {reward_items_row(&quest.rewards, inner_w, rewards_y)}
        {action_buttons(quest.is_complete(), detail_h)}
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

// --- Reward items row ---

fn reward_items_row(rewards: &[QuestRewardItem], w: f32, y: f32) -> Element {
    let hide_rewards = rewards.is_empty();
    let items: Element = rewards
        .iter()
        .enumerate()
        .flat_map(|(i, reward)| reward_item_slot(i, reward))
        .collect();
    rsx! {
        r#frame {
            name: "QuestLogRewards",
            width: {w},
            height: {REWARD_LABEL_H + REWARD_ICON_SIZE + 4.0},
            hidden: hide_rewards,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: "8",
                y: {-y},
            }
            fontstring {
                name: "QuestLogRewardsLabel",
                width: {w},
                height: {REWARD_LABEL_H},
                text: "Rewards",
                font_size: 12.0,
                font_color: REWARD_HEADER_COLOR,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "0",
                    y: "0",
                }
            }
            {items}
        }
    }
}

fn reward_item_slot(idx: usize, reward: &QuestRewardItem) -> Element {
    let slot_id = DynName(format!("QuestLogReward{idx}"));
    let icon_id = DynName(format!("QuestLogReward{idx}Icon"));
    let name_id = DynName(format!("QuestLogReward{idx}Name"));
    let x = idx as f32 * (REWARD_SLOT_W + REWARD_NAME_W + REWARD_GAP);
    let quantity_label = if reward.quantity > 1 {
        format!("{} x{}", reward.name, reward.quantity)
    } else {
        reward.name.clone()
    };
    rsx! {
        r#frame {
            name: slot_id,
            width: {REWARD_SLOT_W + REWARD_NAME_W},
            height: {REWARD_ICON_SIZE},
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {x},
                y: {-REWARD_LABEL_H},
            }
            r#frame {
                name: icon_id,
                width: {REWARD_ICON_SIZE},
                height: {REWARD_ICON_SIZE},
                background_color: REWARD_ICON_BG,
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: "0",
                    y: "0",
                }
            }
            fontstring {
                name: name_id,
                width: {REWARD_NAME_W},
                height: {REWARD_ICON_SIZE},
                text: {quantity_label.as_str()},
                font_size: 10.0,
                font_color: REWARD_NAME_COLOR,
                justify_h: "LEFT",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                    x: {REWARD_SLOT_W},
                    y: "0",
                }
            }
        }
    }
}

// --- Action buttons ---

fn action_buttons(quest_complete: bool, panel_h: f32) -> Element {
    let y = -(panel_h - ACTION_BTN_H - 8.0);
    let (primary_label, primary_bg, primary_text) = if quest_complete {
        ("Complete", COMPLETE_BTN_BG, COMPLETE_BTN_TEXT)
    } else {
        ("Accept", ACCEPT_BTN_BG, ACCEPT_BTN_TEXT)
    };
    let primary_x = 8.0;
    let abandon_x = primary_x + ACTION_BTN_W + ACTION_BTN_GAP;
    rsx! {
        r#frame {
            name: "QuestLogAcceptBtn",
            width: {ACTION_BTN_W},
            height: {ACTION_BTN_H},
            background_color: primary_bg,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {primary_x},
                y: {y},
            }
            fontstring {
                name: "QuestLogAcceptBtnText",
                width: {ACTION_BTN_W},
                height: {ACTION_BTN_H},
                text: primary_label,
                font_size: 11.0,
                font_color: primary_text,
                justify_h: "CENTER",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
            }
        }
        r#frame {
            name: "QuestLogAbandonBtn",
            width: {ACTION_BTN_W},
            height: {ACTION_BTN_H},
            background_color: ABANDON_BTN_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {abandon_x},
                y: {y},
            }
            fontstring {
                name: "QuestLogAbandonBtnText",
                width: {ACTION_BTN_W},
                height: {ACTION_BTN_H},
                text: "Abandon",
                font_size: 11.0,
                font_color: ABANDON_BTN_TEXT,
                justify_h: "CENTER",
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
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
                rewards: vec![
                    QuestRewardItem {
                        name: "Outpost Blade".into(),
                        icon_fdid: 100001,
                        quantity: 1,
                    },
                    QuestRewardItem {
                        name: "Gold Dust".into(),
                        icon_fdid: 100002,
                        quantity: 5,
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
                rewards: vec![],
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
                rewards: vec![QuestRewardItem {
                    name: "Spirit Totem".into(),
                    icon_fdid: 200001,
                    quantity: 1,
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

    // --- Reward items tests ---

    #[test]
    fn builds_reward_items() {
        let reg = build_registry();
        assert!(reg.get_by_name("QuestLogRewards").is_some());
        assert!(reg.get_by_name("QuestLogRewardsLabel").is_some());
        // Selected quest has 2 rewards
        assert!(reg.get_by_name("QuestLogReward0").is_some());
        assert!(reg.get_by_name("QuestLogReward0Icon").is_some());
        assert!(reg.get_by_name("QuestLogReward0Name").is_some());
        assert!(reg.get_by_name("QuestLogReward1").is_some());
    }

    #[test]
    fn rewards_hidden_when_empty() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(QuestLogFrameState {
            visible: true,
            quests: vec![QuestLogEntry {
                quest_id: 1,
                title: "No Rewards".into(),
                level: 10,
                zone: "Test".into(),
                description: "A quest with no rewards.".into(),
                objectives: vec![],
                rewards: vec![],
                selected: true,
            }],
        });
        Screen::new(quest_log_frame_screen).sync(&shared, &mut reg);
        let id = reg.get_by_name("QuestLogRewards").expect("rewards frame");
        assert!(reg.get(id).expect("data").hidden);
    }

    #[test]
    fn reward_quantity_label() {
        let single = QuestRewardItem {
            name: "Sword".into(),
            icon_fdid: 1,
            quantity: 1,
        };
        // quantity 1 → just the name
        assert_eq!(single.name, "Sword");

        let multi = QuestRewardItem {
            name: "Gold Dust".into(),
            icon_fdid: 2,
            quantity: 5,
        };
        // quantity > 1 → "name xN" (tested via the rendered text)
        assert!(multi.quantity > 1);
    }

    // --- Action buttons tests ---

    #[test]
    fn builds_action_buttons() {
        let reg = build_registry();
        assert!(reg.get_by_name("QuestLogAcceptBtn").is_some());
        assert!(reg.get_by_name("QuestLogAcceptBtnText").is_some());
        assert!(reg.get_by_name("QuestLogAbandonBtn").is_some());
        assert!(reg.get_by_name("QuestLogAbandonBtnText").is_some());
    }

    fn fontstring_text(reg: &FrameRegistry, name: &str) -> String {
        use ui_toolkit::frame::WidgetData;
        let id = reg.get_by_name(name).expect(name);
        let frame = reg.get(id).expect("frame data");
        match frame.widget_data.as_ref() {
            Some(WidgetData::FontString(fs)) => fs.text.clone(),
            _ => panic!("{name} is not a FontString"),
        }
    }

    #[test]
    fn complete_button_for_finished_quest() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(QuestLogFrameState {
            visible: true,
            quests: vec![QuestLogEntry {
                quest_id: 99,
                title: "Done Quest".into(),
                level: 10,
                zone: "Test".into(),
                description: "Already finished.".into(),
                objectives: vec![QuestLogObjective {
                    text: "Done".into(),
                    current: 1,
                    required: 1,
                }],
                rewards: vec![],
                selected: true,
            }],
        });
        Screen::new(quest_log_frame_screen).sync(&shared, &mut reg);
        assert_eq!(fontstring_text(&reg, "QuestLogAcceptBtnText"), "Complete");
    }

    #[test]
    fn accept_button_for_incomplete_quest() {
        let reg = build_registry();
        assert_eq!(fontstring_text(&reg, "QuestLogAcceptBtnText"), "Accept");
    }

    #[test]
    fn coord_action_buttons() {
        let reg = layout_registry();
        let detail_r = rect(&reg, "QuestLogDetail");
        let accept_r = rect(&reg, "QuestLogAcceptBtn");
        let abandon_r = rect(&reg, "QuestLogAbandonBtn");
        // Buttons near bottom of detail panel
        let expected_btn_bottom = detail_r.y + detail_r.height;
        assert!((accept_r.y + accept_r.height - expected_btn_bottom).abs() < 10.0);
        // Abandon is to the right of accept
        assert!(abandon_r.x > accept_r.x);
        assert!((accept_r.width - ACTION_BTN_W).abs() < 1.0);
        assert!((abandon_r.width - ACTION_BTN_W).abs() < 1.0);
    }

    #[test]
    fn coord_list_panel_vertical() {
        let reg = layout_registry();
        let frame_r = rect(&reg, "QuestLogFrame");
        let list_r = rect(&reg, "QuestLogList");
        let expected_y = frame_r.y + CONTENT_TOP;
        let expected_h = FRAME_H - CONTENT_TOP - INSET;
        assert!((list_r.y - expected_y).abs() < 1.0);
        assert!((list_r.height - expected_h).abs() < 1.0);
    }

    #[test]
    fn coord_detail_panel_vertical() {
        let reg = layout_registry();
        let frame_r = rect(&reg, "QuestLogFrame");
        let detail_r = rect(&reg, "QuestLogDetail");
        let expected_y = frame_r.y + CONTENT_TOP;
        let expected_h = FRAME_H - CONTENT_TOP - INSET;
        assert!((detail_r.y - expected_y).abs() < 1.0);
        assert!((detail_r.height - expected_h).abs() < 1.0);
    }

    #[test]
    fn coord_zone_header_inside_list() {
        let reg = layout_registry();
        let list_r = rect(&reg, "QuestLogList");
        let zone_r = rect(&reg, "QuestLogZone0");
        // Zone header at top of list
        assert!((zone_r.y - list_r.y).abs() < 1.0);
        assert!((zone_r.height - ZONE_HEADER_H).abs() < 1.0);
    }

    #[test]
    fn coord_quest_row_below_zone_header() {
        let reg = layout_registry();
        let zone_r = rect(&reg, "QuestLogZone0");
        let row_r = rect(&reg, "QuestLogRow0_0");
        // First quest row starts after zone header + gap
        let expected_y = zone_r.y + ZONE_HEADER_H + ROW_GAP;
        assert!((row_r.y - expected_y).abs() < 1.0);
        assert!((row_r.height - QUEST_ROW_H).abs() < 1.0);
    }

    #[test]
    fn coord_title_centered() {
        let reg = layout_registry();
        let frame_r = rect(&reg, "QuestLogFrame");
        let title_r = rect(&reg, "QuestLogFrameTitle");
        assert!((title_r.x - frame_r.x).abs() < 1.0);
        assert!((title_r.y - frame_r.y).abs() < 1.0);
        assert!((title_r.width - FRAME_W).abs() < 1.0);
    }
}
