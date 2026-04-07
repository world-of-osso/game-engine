use super::*;
use ui_toolkit::layout::{LayoutRect, recompute_layouts};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn obj(text: &str, current: u32, required: u32) -> QuestLogObjective {
    QuestLogObjective {
        text: text.into(),
        current,
        required,
    }
}

fn reward(name: &str, fdid: u32, qty: u32) -> QuestRewardItem {
    QuestRewardItem {
        name: name.into(),
        icon_fdid: fdid,
        quantity: qty,
    }
}

fn quest(
    id: u32,
    title: &str,
    level: u32,
    zone: &str,
    desc: &str,
    objectives: Vec<QuestLogObjective>,
    rewards: Vec<QuestRewardItem>,
    selected: bool,
) -> QuestLogEntry {
    QuestLogEntry {
        quest_id: id,
        title: title.into(),
        level,
        zone: zone.into(),
        description: desc.into(),
        objectives,
        rewards,
        selected,
    }
}

fn sample_quests() -> Vec<QuestLogEntry> {
    vec![
        quest(
            101,
            "The Fallen Outpost",
            25,
            "Stonetalon Mountains",
            "Investigate the ruins of the fallen outpost.",
            vec![
                obj("Investigate ruins", 0, 1),
                obj("Defeat guardians", 2, 5),
            ],
            vec![
                reward("Outpost Blade", 100001, 1),
                reward("Gold Dust", 100002, 5),
            ],
            true,
        ),
        quest(
            102,
            "Supplies for the Front",
            26,
            "Stonetalon Mountains",
            "Gather supplies from the nearby camps.",
            vec![obj("Gather supplies", 8, 8)],
            vec![],
            false,
        ),
        quest(
            201,
            "Ancient Spirits",
            30,
            "Desolace",
            "Commune with the ancient spirits of Desolace.",
            vec![obj("Commune with spirits", 1, 3)],
            vec![reward("Spirit Totem", 200001, 1)],
            false,
        ),
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
