use super::*;

#[test]
fn swap_within_same_bag() {
    let mut inv = InventoryState::default();
    inv.set_item(0, 0, test_item("Sword"));
    inv.set_item(0, 1, test_item("Shield"));
    assert!(inv.swap_slots(0, 0, 0, 1));
    assert_eq!(inv.slot(0, 0).unwrap().name, "Shield");
    assert_eq!(inv.slot(0, 1).unwrap().name, "Sword");
}

#[test]
fn swap_between_bags() {
    let mut inv = InventoryState::default();
    inv.add_bag(BagInfo {
        index: 1,
        name: "Bag2".into(),
        size: 4,
        icon_fdid: 0,
    });
    inv.set_item(0, 0, test_item("Ore"));
    inv.set_item(1, 2, test_item("Gem"));
    assert!(inv.swap_slots(0, 0, 1, 2));
    assert_eq!(inv.slot(0, 0).unwrap().name, "Gem");
    assert_eq!(inv.slot(1, 2).unwrap().name, "Ore");
}

#[test]
fn swap_with_empty_slot() {
    let mut inv = InventoryState::default();
    inv.set_item(0, 0, test_item("Axe"));
    assert!(inv.swap_slots(0, 0, 0, 5));
    assert!(inv.slot(0, 0).unwrap().is_empty());
    assert_eq!(inv.slot(0, 5).unwrap().name, "Axe");
}

#[test]
fn swap_same_slot_returns_false() {
    let mut inv = InventoryState::default();
    inv.set_item(0, 0, test_item("Axe"));
    assert!(!inv.swap_slots(0, 0, 0, 0));
}

#[test]
fn swap_out_of_bounds_returns_false() {
    let mut inv = InventoryState::default();
    assert!(!inv.swap_slots(0, 0, 99, 0));
    assert!(!inv.swap_slots(99, 0, 0, 0));
}

#[test]
fn move_to_empty_succeeds() {
    let mut inv = InventoryState::default();
    inv.set_item(0, 0, test_item("Ring"));
    assert!(inv.move_to_empty(0, 0, 0, 3));
    assert!(inv.slot(0, 0).unwrap().is_empty());
    assert_eq!(inv.slot(0, 3).unwrap().name, "Ring");
}

#[test]
fn move_to_occupied_fails() {
    let mut inv = InventoryState::default();
    inv.set_item(0, 0, test_item("A"));
    inv.set_item(0, 1, test_item("B"));
    assert!(!inv.move_to_empty(0, 0, 0, 1));
}

#[test]
fn try_stack_combines_counts() {
    let mut inv = InventoryState::default();
    inv.set_item(
        0,
        0,
        InventorySlot {
            icon_fdid: 1,
            count: 10,
            name: "Iron Ore".into(),
            ..Default::default()
        },
    );
    inv.set_item(
        0,
        1,
        InventorySlot {
            icon_fdid: 1,
            count: 5,
            name: "Iron Ore".into(),
            ..Default::default()
        },
    );
    assert!(inv.try_stack(0, 0, 0, 1));
    assert!(inv.slot(0, 0).unwrap().is_empty());
    assert_eq!(inv.slot(0, 1).unwrap().count, 15);
}

#[test]
fn try_stack_different_items_fails() {
    let mut inv = InventoryState::default();
    inv.set_item(0, 0, test_item("Iron Ore"));
    inv.set_item(0, 1, test_item("Gold Ore"));
    assert!(!inv.try_stack(0, 0, 0, 1));
}
