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

// --- Boss list / detail tests ---

fn make_boss_state() -> EncounterJournalState {
    let mut state = make_test_state();
    state.bosses = vec![
        BossEntry {
            name: "Edwin VanCleef".into(),
            selected: true,
        },
        BossEntry {
            name: "Cookie".into(),
            selected: false,
        },
    ];
    state.selected_boss_name = "Edwin VanCleef".into();
    state.abilities = vec![
        BossAbility {
            name: "Deadly Poison".into(),
            description: "Coats weapons with poison.".into(),
            icon_fdid: 12345,
        },
        BossAbility {
            name: "Summon Pirates".into(),
            description: "Calls nearby pirates to aid.".into(),
            icon_fdid: 12346,
        },
    ];
    state
}

fn boss_registry() -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(make_boss_state());
    Screen::new(encounter_journal_screen).sync(&shared, &mut reg);
    reg
}

#[test]
fn boss_list_builds_entries() {
    let reg = boss_registry();
    assert!(reg.get_by_name("EJBossList").is_some());
    assert!(reg.get_by_name("EJBoss0").is_some());
    assert!(reg.get_by_name("EJBoss1").is_some());
    assert!(reg.get_by_name("EJBoss0Label").is_some());
}

#[test]
fn boss_detail_builds_name_and_abilities() {
    let reg = boss_registry();
    assert!(reg.get_by_name("EJBossDetail").is_some());
    assert!(reg.get_by_name("EJBossDetailName").is_some());
    for i in 0..2 {
        assert!(
            reg.get_by_name(&format!("EJAbility{i}")).is_some(),
            "EJAbility{i} missing"
        );
        assert!(
            reg.get_by_name(&format!("EJAbility{i}Icon")).is_some(),
            "EJAbility{i}Icon missing"
        );
        assert!(
            reg.get_by_name(&format!("EJAbility{i}Name")).is_some(),
            "EJAbility{i}Name missing"
        );
        assert!(
            reg.get_by_name(&format!("EJAbility{i}Desc")).is_some(),
            "EJAbility{i}Desc missing"
        );
    }
}

// --- Loot tab tests ---

fn make_loot_state() -> EncounterJournalState {
    let mut state = make_test_state();
    state.loot_items = vec![
        LootItem {
            name: "Cruel Barb".into(),
            slot: "One-Hand Sword".into(),
            drop_pct: "15%".into(),
            icon_fdid: 11111,
        },
        LootItem {
            name: "Cape of the Brotherhood".into(),
            slot: "Back".into(),
            drop_pct: "18%".into(),
            icon_fdid: 22222,
        },
    ];
    state
}

fn loot_registry() -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(make_loot_state());
    Screen::new(encounter_journal_screen).sync(&shared, &mut reg);
    reg
}

#[test]
fn loot_tab_builds_filters() {
    let reg = loot_registry();
    assert!(reg.get_by_name("EJLootTab").is_some());
    assert!(reg.get_by_name("EJLootSlotFilter").is_some());
    assert!(reg.get_by_name("EJLootClassFilter").is_some());
}

#[test]
fn loot_tab_builds_header() {
    let reg = loot_registry();
    assert!(reg.get_by_name("EJLootHeader").is_some());
    for i in 0..LOOT_COLUMNS.len() {
        assert!(
            reg.get_by_name(&format!("EJLootCol{i}")).is_some(),
            "EJLootCol{i} missing"
        );
    }
}

#[test]
fn loot_tab_builds_item_rows() {
    let reg = loot_registry();
    for i in 0..2 {
        assert!(
            reg.get_by_name(&format!("EJLoot{i}")).is_some(),
            "EJLoot{i} missing"
        );
        assert!(
            reg.get_by_name(&format!("EJLoot{i}Icon")).is_some(),
            "EJLoot{i}Icon missing"
        );
        assert!(
            reg.get_by_name(&format!("EJLoot{i}Name")).is_some(),
            "EJLoot{i}Name missing"
        );
        assert!(
            reg.get_by_name(&format!("EJLoot{i}Slot")).is_some(),
            "EJLoot{i}Slot missing"
        );
        assert!(
            reg.get_by_name(&format!("EJLoot{i}Drop")).is_some(),
            "EJLoot{i}Drop missing"
        );
    }
}

// --- Additional coord validation ---

fn boss_layout_registry() -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(make_boss_state());
    Screen::new(encounter_journal_screen).sync(&shared, &mut reg);
    recompute_layouts(&mut reg);
    reg
}

#[test]
fn coord_boss_list_position() {
    let reg = boss_layout_registry();
    let content = rect(&reg, "EJContentArea");
    let bl = rect(&reg, "EJBossList");
    assert!((bl.x - (content.x + BOSS_LIST_INSET)).abs() < 1.0);
    assert!((bl.y - (content.y + BOSS_LIST_INSET)).abs() < 1.0);
    assert!((bl.width - BOSS_LIST_W).abs() < 1.0);
}

#[test]
fn coord_boss_detail_right_of_list() {
    let reg = boss_layout_registry();
    let content = rect(&reg, "EJContentArea");
    let detail = rect(&reg, "EJBossDetail");
    let expected_x = content.x + BOSS_LIST_INSET + BOSS_LIST_W + DETAIL_INSET;
    assert!(
        (detail.x - expected_x).abs() < 1.0,
        "x: expected {expected_x}, got {}",
        detail.x
    );
}

#[test]
fn coord_loot_tab_filters() {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(make_loot_state());
    Screen::new(encounter_journal_screen).sync(&shared, &mut reg);
    recompute_layouts(&mut reg);

    let slot_f = rect(&reg, "EJLootSlotFilter");
    let class_f = rect(&reg, "EJLootClassFilter");
    assert!((slot_f.width - LOOT_FILTER_W).abs() < 1.0);
    assert!((class_f.width - LOOT_FILTER_W).abs() < 1.0);
    let spacing = class_f.x - slot_f.x;
    let expected = LOOT_FILTER_W + LOOT_FILTER_GAP;
    assert!(
        (spacing - expected).abs() < 1.0,
        "filter spacing: expected {expected}, got {spacing}"
    );
}
