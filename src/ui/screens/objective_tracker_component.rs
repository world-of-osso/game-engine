use std::fmt;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;

struct DynName(String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub const TRACKER_W: f32 = 248.0;
const HEADER_H: f32 = 20.0;
const OBJECTIVE_H: f32 = 14.0;
const OBJECTIVE_GAP: f32 = 2.0;
const QUEST_GAP: f32 = 8.0;
const CHECKBOX_SIZE: f32 = 12.0;
const CHECKBOX_GAP: f32 = 4.0;
const INSET: f32 = 4.0;

const HEADER_COLOR: &str = "1.0,0.82,0.0,1.0";
const OBJECTIVE_COLOR: &str = "0.8,0.8,0.8,1.0";
const OBJECTIVE_DONE_COLOR: &str = "0.0,1.0,0.0,1.0";
const CHECKBOX_BG: &str = "0.1,0.1,0.1,0.8";
const CHECKBOX_CHECK: &str = "0.0,1.0,0.0,1.0";

pub const MAX_QUESTS: usize = 8;
pub const MAX_OBJECTIVES_PER_QUEST: usize = 5;

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectiveLine {
    pub text: String,
    pub completed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrackedQuest {
    pub title: String,
    pub collapsed: bool,
    pub objectives: Vec<ObjectiveLine>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ObjectiveTrackerState {
    pub quests: Vec<TrackedQuest>,
}

pub fn objective_tracker_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<ObjectiveTrackerState>()
        .expect("ObjectiveTrackerState must be in SharedContext");
    let mut y_cursor = 0.0_f32;
    let quest_elements: Element = state
        .quests
        .iter()
        .enumerate()
        .take(MAX_QUESTS)
        .flat_map(|(qi, quest)| {
            let header_y = -y_cursor;
            y_cursor += HEADER_H + QUEST_GAP;
            let header = quest_header(qi, quest, header_y);
            let objectives: Element = if quest.collapsed {
                Vec::new()
            } else {
                quest
                    .objectives
                    .iter()
                    .enumerate()
                    .take(MAX_OBJECTIVES_PER_QUEST)
                    .flat_map(|(oi, obj)| {
                        let obj_y = -y_cursor;
                        y_cursor += OBJECTIVE_H + OBJECTIVE_GAP;
                        objective_line(qi, oi, obj, obj_y)
                    })
                    .collect()
            };
            [header, objectives]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
        })
        .collect();
    rsx! {
        r#frame {
            name: "ObjectiveTrackerFrame",
            width: {TRACKER_W},
            height: {y_cursor.max(20.0)},
            anchor {
                point: AnchorPoint::TopRight,
                relative_point: AnchorPoint::TopRight,
                x: "-10",
                y: "-260",
            }
            {quest_elements}
        }
    }
}

fn quest_header(qi: usize, quest: &TrackedQuest, y: f32) -> Element {
    let header_id = DynName(format!("QuestHeader{qi}"));
    rsx! {
        fontstring {
            name: header_id,
            width: {TRACKER_W - INSET},
            height: {HEADER_H},
            text: {quest.title.as_str()},
            font_size: 12.0,
            font_color: HEADER_COLOR,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {INSET},
                y: {y},
            }
        }
    }
}

fn objective_line(qi: usize, oi: usize, obj: &ObjectiveLine, y: f32) -> Element {
    let cb_id = DynName(format!("QuestObj{qi}_{oi}Check"));
    let text_id = DynName(format!("QuestObj{qi}_{oi}Text"));
    let check_text = if obj.completed { "\u{2713}" } else { "" };
    let text_color = if obj.completed {
        OBJECTIVE_DONE_COLOR
    } else {
        OBJECTIVE_COLOR
    };
    let text_x = INSET + CHECKBOX_SIZE + CHECKBOX_GAP;
    rsx! {
        r#frame {
            name: cb_id,
            width: {CHECKBOX_SIZE},
            height: {CHECKBOX_SIZE},
            background_color: CHECKBOX_BG,
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {INSET},
                y: {y},
            }
            fontstring {
                name: DynName(format!("QuestObj{qi}_{oi}CheckText")),
                width: {CHECKBOX_SIZE},
                height: {CHECKBOX_SIZE},
                text: check_text,
                font_size: 10.0,
                font_color: CHECKBOX_CHECK,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::TopLeft,
                    relative_point: AnchorPoint::TopLeft,
                }
            }
        }
        fontstring {
            name: text_id,
            width: {TRACKER_W - text_x - INSET},
            height: {OBJECTIVE_H},
            text: {obj.text.as_str()},
            font_size: 10.0,
            font_color: text_color,
            justify_h: "LEFT",
            anchor {
                point: AnchorPoint::TopLeft,
                relative_point: AnchorPoint::TopLeft,
                x: {text_x},
                y: {y},
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

    fn make_state() -> ObjectiveTrackerState {
        ObjectiveTrackerState {
            quests: vec![
                TrackedQuest {
                    title: "The Defias Brotherhood".into(),
                    collapsed: false,
                    objectives: vec![
                        ObjectiveLine {
                            text: "Kill 10 Defias".into(),
                            completed: true,
                        },
                        ObjectiveLine {
                            text: "Collect 5 bandanas".into(),
                            completed: false,
                        },
                    ],
                },
                TrackedQuest {
                    title: "Red Ridge Supply Run".into(),
                    collapsed: true,
                    objectives: vec![ObjectiveLine {
                        text: "Gather supplies".into(),
                        completed: false,
                    }],
                },
            ],
        }
    }

    fn build_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_state());
        Screen::new(objective_tracker_screen).sync(&shared, &mut reg);
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
    fn builds_tracker_frame() {
        let reg = build_registry();
        assert!(reg.get_by_name("ObjectiveTrackerFrame").is_some());
    }

    #[test]
    fn builds_quest_headers() {
        let reg = build_registry();
        assert!(reg.get_by_name("QuestHeader0").is_some());
        assert!(reg.get_by_name("QuestHeader1").is_some());
    }

    #[test]
    fn builds_objectives_for_expanded_quest() {
        let reg = build_registry();
        assert!(reg.get_by_name("QuestObj0_0Check").is_some());
        assert!(reg.get_by_name("QuestObj0_0Text").is_some());
        assert!(reg.get_by_name("QuestObj0_1Check").is_some());
        assert!(reg.get_by_name("QuestObj0_1Text").is_some());
    }

    #[test]
    fn collapsed_quest_hides_objectives() {
        let reg = build_registry();
        // Quest 1 is collapsed — no objectives rendered
        assert!(reg.get_by_name("QuestObj1_0Check").is_none());
    }

    // --- Coord validation ---

    #[test]
    fn coord_tracker_right_anchored() {
        let reg = layout_registry();
        let r = rect(&reg, "ObjectiveTrackerFrame");
        let expected_x = 1920.0 - 10.0 - TRACKER_W;
        assert!((r.x - expected_x).abs() < 1.0);
        assert!((r.width - TRACKER_W).abs() < 1.0);
    }

    #[test]
    fn coord_first_checkbox_dimensions() {
        let reg = layout_registry();
        let r = rect(&reg, "QuestObj0_0Check");
        assert!((r.width - CHECKBOX_SIZE).abs() < 1.0);
        assert!((r.height - CHECKBOX_SIZE).abs() < 1.0);
    }
}
