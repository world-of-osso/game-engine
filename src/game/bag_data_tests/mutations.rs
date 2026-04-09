use super::*;

#[test]
fn set_item_populates_slot() {
    let mut inv = InventoryState::default();
    inv.set_item(
        0,
        5,
        InventorySlot {
            icon_fdid: 135274,
            count: 20,
            quality: ItemQuality::Common,
            name: "Iron Ore".into(),
        },
    );
    let slot = inv.slot(0, 5).unwrap();
    assert_eq!(slot.name, "Iron Ore");
    assert_eq!(slot.count, 20);
    assert!(!slot.is_empty());
}

#[test]
fn clear_slot_empties() {
    let mut inv = InventoryState::default();
    inv.set_item(
        0,
        0,
        InventorySlot {
            icon_fdid: 100,
            name: "Sword".into(),
            ..Default::default()
        },
    );
    assert!(!inv.slot(0, 0).unwrap().is_empty());
    inv.clear_slot(0, 0);
    assert!(inv.slot(0, 0).unwrap().is_empty());
}

#[test]
fn set_item_out_of_bounds_no_panic() {
    let mut inv = InventoryState::default();
    inv.set_item(99, 0, InventorySlot::default());
    inv.set_item(0, 99, InventorySlot::default());
}

#[test]
fn add_bag_extends_inventory() {
    let mut inv = InventoryState::default();
    assert_eq!(inv.bags.len(), 1);
    inv.add_bag(BagInfo {
        index: 1,
        name: "Netherweave Bag".into(),
        size: 16,
        icon_fdid: 133625,
    });
    assert_eq!(inv.bags.len(), 2);
    assert_eq!(inv.bag_slot_count(1), 16);
    assert_eq!(inv.total_slots(), 32);
}

#[test]
fn first_empty_slot_finds_earliest() {
    let mut inv = InventoryState::default();
    inv.set_item(
        0,
        0,
        InventorySlot {
            icon_fdid: 1,
            name: "A".into(),
            ..Default::default()
        },
    );
    inv.set_item(
        0,
        1,
        InventorySlot {
            icon_fdid: 2,
            name: "B".into(),
            ..Default::default()
        },
    );
    assert_eq!(inv.first_empty_slot(), Some((0, 2)));
}

#[test]
fn first_empty_slot_none_when_full() {
    let inv = InventoryState {
        bags: vec![BagInfo {
            index: 0,
            name: "Tiny".into(),
            size: 2,
            icon_fdid: 0,
        }],
        slots: vec![vec![
            InventorySlot {
                icon_fdid: 1,
                name: "A".into(),
                ..Default::default()
            },
            InventorySlot {
                icon_fdid: 2,
                name: "B".into(),
                ..Default::default()
            },
        ]],
    };
    assert!(inv.first_empty_slot().is_none());
}

#[test]
fn count_item_across_bags() {
    let mut inv = InventoryState::default();
    inv.add_bag(BagInfo {
        index: 1,
        name: "Bag".into(),
        size: 4,
        icon_fdid: 0,
    });
    inv.set_item(
        0,
        0,
        InventorySlot {
            icon_fdid: 1,
            count: 20,
            name: "Iron Ore".into(),
            ..Default::default()
        },
    );
    inv.set_item(
        1,
        0,
        InventorySlot {
            icon_fdid: 1,
            count: 15,
            name: "Iron Ore".into(),
            ..Default::default()
        },
    );
    assert_eq!(inv.count_item("Iron Ore"), 35);
    assert_eq!(inv.count_item("Gold Ore"), 0);
}

#[test]
fn replace_all_resets_inventory() {
    let mut inv = InventoryState::default();
    inv.set_item(
        0,
        0,
        InventorySlot {
            icon_fdid: 1,
            name: "Old".into(),
            ..Default::default()
        },
    );
    inv.replace_all(
        vec![BagInfo {
            index: 0,
            name: "New Backpack".into(),
            size: 20,
            icon_fdid: 0,
        }],
        vec![vec![InventorySlot::default(); 20]],
    );
    assert_eq!(inv.bags.len(), 1);
    assert_eq!(inv.bags[0].name, "New Backpack");
    assert_eq!(inv.total_slots(), 20);
    assert_eq!(inv.total_free_slots(), 20);
}
