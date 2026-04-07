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

// Bonus / timer / scenario
const PROGRESS_BAR_W: f32 = 180.0;
const PROGRESS_BAR_H: f32 = 10.0;
const TIMER_H: f32 = 16.0;
const SCENARIO_STEP_H: f32 = 16.0;
const PROGRESS_BG: &str = "0.1,0.1,0.1,0.9";
const PROGRESS_FILL: &str = "0.2,0.6,0.1,0.9";
const PROGRESS_TEXT_COLOR: &str = "1.0,1.0,1.0,0.9";
const TIMER_COLOR: &str = "1.0,0.4,0.0,1.0";
const SCENARIO_HEADER_COLOR: &str = "1.0,0.82,0.0,1.0";
const SCENARIO_STEP_COLOR: &str = "0.8,0.8,0.8,1.0";

pub const MAX_QUESTS: usize = 8;
pub const MAX_OBJECTIVES_PER_QUEST: usize = 5;
pub const MAX_BONUS_OBJECTIVES: usize = 3;
pub const MAX_SCENARIO_STEPS: usize = 5;

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

#[derive(Clone, Debug, PartialEq)]
pub struct BonusObjective {
    pub name: String,
    pub progress: f32,
    pub progress_text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimerBlock {
    pub label: String,
    pub time_text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioStep {
    pub text: String,
    pub completed: bool,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ObjectiveTrackerState {
    pub quests: Vec<TrackedQuest>,
    pub bonus_objectives: Vec<BonusObjective>,
    pub timers: Vec<TimerBlock>,
    pub scenario_name: String,
    pub scenario_steps: Vec<ScenarioStep>,
}

fn build_quest_elements(quests: &[TrackedQuest], y_cursor: &mut f32) -> Element {
    quests
        .iter()
        .enumerate()
        .take(MAX_QUESTS)
        .flat_map(|(qi, quest)| {
            let header_y = -*y_cursor;
            *y_cursor += HEADER_H + QUEST_GAP;
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
                        let obj_y = -*y_cursor;
                        *y_cursor += OBJECTIVE_H + OBJECTIVE_GAP;
                        objective_line(qi, oi, obj, obj_y)
                    })
                    .collect()
            };
            [header, objectives]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
        })
        .collect()
}

pub fn objective_tracker_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<ObjectiveTrackerState>()
        .expect("ObjectiveTrackerState must be in SharedContext");
    let mut y_cursor = 0.0_f32;
    let quest_elements = build_quest_elements(&state.quests, &mut y_cursor);
    let bonus_elements = bonus_section(&state.bonus_objectives, &mut y_cursor);
    let timer_elements = timer_section(&state.timers, &mut y_cursor);
    let scenario_elements =
        scenario_section(&state.scenario_name, &state.scenario_steps, &mut y_cursor);
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
            {bonus_elements}
            {timer_elements}
            {scenario_elements}
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

fn bonus_section(bonuses: &[BonusObjective], y: &mut f32) -> Element {
    bonuses
        .iter()
        .enumerate()
        .take(MAX_BONUS_OBJECTIVES)
        .flat_map(|(i, bonus)| {
            let label_y = -*y;
            *y += OBJECTIVE_H + OBJECTIVE_GAP;
            let bar_y = -*y;
            *y += PROGRESS_BAR_H + QUEST_GAP;
            let name_id = DynName(format!("BonusObj{i}Name"));
            let bar_id = DynName(format!("BonusObj{i}Bar"));
            let fill_id = DynName(format!("BonusObj{i}Fill"));
            let text_id = DynName(format!("BonusObj{i}Text"));
            let fill_w = PROGRESS_BAR_W * bonus.progress.clamp(0.0, 1.0);
            rsx! {
                fontstring {
                    name: name_id,
                    width: {TRACKER_W - 2.0 * INSET},
                    height: {OBJECTIVE_H},
                    text: {bonus.name.as_str()},
                    font_size: 10.0,
                    font_color: OBJECTIVE_COLOR,
                    justify_h: "LEFT",
                    anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {INSET}, y: {label_y} }
                }
                r#frame {
                    name: bar_id,
                    width: {PROGRESS_BAR_W},
                    height: {PROGRESS_BAR_H},
                    background_color: PROGRESS_BG,
                    anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {INSET}, y: {bar_y} }
                    r#frame {
                        name: fill_id,
                        width: {fill_w},
                        height: {PROGRESS_BAR_H},
                        background_color: PROGRESS_FILL,
                        anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
                    }
                    fontstring {
                        name: text_id,
                        width: {PROGRESS_BAR_W},
                        height: {PROGRESS_BAR_H},
                        text: {bonus.progress_text.as_str()},
                        font_size: 8.0,
                        font_color: PROGRESS_TEXT_COLOR,
                        justify_h: "CENTER",
                        anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
                    }
                }
            }
        })
        .collect()
}

fn timer_section(timers: &[TimerBlock], y: &mut f32) -> Element {
    timers
        .iter()
        .enumerate()
        .flat_map(|(i, timer)| {
            let ty = -*y;
            *y += TIMER_H + OBJECTIVE_GAP;
            let label_id = DynName(format!("Timer{i}Label"));
            let time_id = DynName(format!("Timer{i}Time"));
            rsx! {
                fontstring {
                    name: label_id,
                    width: {TRACKER_W * 0.6},
                    height: {TIMER_H},
                    text: {timer.label.as_str()},
                    font_size: 10.0,
                    font_color: TIMER_COLOR,
                    justify_h: "LEFT",
                    anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {INSET}, y: {ty} }
                }
                fontstring {
                    name: time_id,
                    width: {TRACKER_W * 0.35},
                    height: {TIMER_H},
                    text: {timer.time_text.as_str()},
                    font_size: 10.0,
                    font_color: TIMER_COLOR,
                    justify_h: "RIGHT",
                    anchor { point: AnchorPoint::TopRight, relative_point: AnchorPoint::TopRight, x: {-INSET}, y: {ty} }
                }
            }
        })
        .collect()
}

fn scenario_section(name: &str, steps: &[ScenarioStep], y: &mut f32) -> Element {
    if name.is_empty() {
        return Vec::new();
    }
    let header_y = -*y;
    *y += HEADER_H + OBJECTIVE_GAP;
    let step_elements: Element = steps
        .iter()
        .enumerate()
        .take(MAX_SCENARIO_STEPS)
        .flat_map(|(i, step)| {
            let sy = -*y;
            *y += SCENARIO_STEP_H + OBJECTIVE_GAP;
            let step_id = DynName(format!("ScenarioStep{i}"));
            let color = if step.completed { OBJECTIVE_DONE_COLOR } else { SCENARIO_STEP_COLOR };
            rsx! {
                fontstring {
                    name: step_id,
                    width: {TRACKER_W - 2.0 * INSET},
                    height: {SCENARIO_STEP_H},
                    text: {step.text.as_str()},
                    font_size: 10.0,
                    font_color: color,
                    justify_h: "LEFT",
                    anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {INSET}, y: {sy} }
                }
            }
        })
        .collect();
    rsx! {
        fontstring {
            name: "ScenarioHeader",
            width: {TRACKER_W - 2.0 * INSET},
            height: {HEADER_H},
            text: name,
            font_size: 12.0,
            font_color: SCENARIO_HEADER_COLOR,
            justify_h: "LEFT",
            anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft, x: {INSET}, y: {header_y} }
        }
        {step_elements}
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
            ..Default::default()
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

    // --- Bonus / timer / scenario tests ---

    fn make_full_state() -> ObjectiveTrackerState {
        let mut state = make_state();
        state.bonus_objectives = vec![BonusObjective {
            name: "Defend the Bridge".into(),
            progress: 0.6,
            progress_text: "3/5".into(),
        }];
        state.timers = vec![TimerBlock {
            label: "Arena".into(),
            time_text: "1:30".into(),
        }];
        state.scenario_name = "Proving Grounds".into();
        state.scenario_steps = vec![
            ScenarioStep {
                text: "Survive wave 1".into(),
                completed: true,
            },
            ScenarioStep {
                text: "Survive wave 2".into(),
                completed: false,
            },
        ];
        state
    }

    fn full_registry() -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_full_state());
        Screen::new(objective_tracker_screen).sync(&shared, &mut reg);
        reg
    }

    #[test]
    fn bonus_objective_builds_progress_bar() {
        let reg = full_registry();
        assert!(reg.get_by_name("BonusObj0Name").is_some());
        assert!(reg.get_by_name("BonusObj0Bar").is_some());
        assert!(reg.get_by_name("BonusObj0Fill").is_some());
    }

    #[test]
    fn timer_block_builds() {
        let reg = full_registry();
        assert!(reg.get_by_name("Timer0Label").is_some());
        assert!(reg.get_by_name("Timer0Time").is_some());
    }

    #[test]
    fn scenario_builds_header_and_steps() {
        let reg = full_registry();
        assert!(reg.get_by_name("ScenarioHeader").is_some());
        assert!(reg.get_by_name("ScenarioStep0").is_some());
        assert!(reg.get_by_name("ScenarioStep1").is_some());
    }
}
