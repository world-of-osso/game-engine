use ui_toolkit::layout::LayoutRect;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

use super::*;
use crate::ui::screens::menu_character_layout_test_support::compute_layout;
use crate::ui::screens::screen_test_helpers::fontstring_text;

fn build(state: LootFrameState) -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    Screen::new(loot_frame_screen).sync(&shared, &mut reg);
    compute_layout(&mut reg);
    reg
}

fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
    reg.get(reg.get_by_name(name).expect(name))
        .and_then(|frame| frame.layout_rect.clone())
        .unwrap_or_else(|| panic!("{name} has no layout rect"))
}

fn offset(reg: &FrameRegistry, name: &str) -> (f32, f32) {
    let root = rect(reg, FRAME_NAME);
    let child = rect(reg, name);
    (child.x - root.x, child.y - root.y)
}

fn onclick(reg: &FrameRegistry, name: &str) -> Option<String> {
    reg.get(reg.get_by_name(name).expect(name))
        .unwrap()
        .onclick
        .clone()
}

/// A Kobold Vermin: 3 copper and a Melted Candle.
fn kobold_loot() -> LootFrameState {
    LootFrameState {
        visible: true,
        rows: vec![
            LootFrameRow {
                slot: 0,
                icon_fdid: 133_788,
                name: "3 Copper".into(),
                color: "1.0,1.0,1.0,1.0",
                quality_text: None,
                count: 1,
            },
            LootFrameRow {
                slot: 1,
                icon_fdid: 135_249,
                name: "Melted Candle".into(),
                color: "0.62,0.62,0.62,1.0",
                quality_text: Some("Poor"),
                count: 1,
            },
        ],
        left: 400.0,
        top: 300.0,
    }
}

#[test]
fn loot_frame_stacks_one_card_per_slot_under_the_items_title() {
    let reg = build(kobold_loot());
    let root = rect(&reg, FRAME_NAME);
    assert_eq!((root.x, root.y, root.width), (400.0, 300.0, 220.0));
    // Two 46-tall cards, 2 apart, plus 46 of chrome.
    assert_eq!(root.height, 2.0 * 46.0 + 2.0 + 46.0);
    assert_eq!(fontstring_text(&reg, "LootFrameTitleText"), "Items");
    assert_eq!(offset(&reg, "LootFrameElement1"), (10.0, 28.0));
    assert_eq!(offset(&reg, "LootFrameElement2"), (10.0, 76.0));
    assert_eq!(rect(&reg, "LootFrameElement1").width, 198.0);
    assert_eq!(
        onclick(&reg, "LootFrameElement2").as_deref(),
        Some("loot_slot:1")
    );
    assert_eq!(
        onclick(&reg, "LootFrameCloseButton").as_deref(),
        Some(ACTION_CLOSE)
    );
}

#[test]
fn item_cards_carry_the_quality_tag_and_money_cards_do_not() {
    let reg = build(kobold_loot());
    assert_eq!(
        fontstring_text(&reg, "LootFrameElement2Text"),
        "Melted Candle"
    );
    assert_eq!(
        fontstring_text(&reg, "LootFrameElement2QualityText"),
        "Poor"
    );
    assert!(reg.get_by_name("LootFrameElement2QualityStripe").is_some());
    assert_eq!(fontstring_text(&reg, "LootFrameElement1Text"), "3 Copper");
    assert!(reg.get_by_name("LootFrameElement1QualityStripe").is_none());
    // Item button TOPLEFT 5,-4, text at its TOPRIGHT +8,-8.
    assert_eq!(offset(&reg, "LootFrameElement2ItemIcon"), (15.0, 80.0));
    assert_eq!(offset(&reg, "LootFrameElement2Text"), (60.0, 88.0));
}

#[test]
fn stacks_show_their_count() {
    let mut state = kobold_loot();
    state.rows[1].count = 3;
    let reg = build(state);
    assert_eq!(fontstring_text(&reg, "LootFrameElement2ItemCount"), "3");
    assert!(reg.get_by_name("LootFrameElement1ItemCount").is_none());
}
