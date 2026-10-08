use super::*;

#[test]
fn default_inventory_has_backpack() {
    let inv = InventoryState::default();
    assert_eq!(inv.bags.len(), 1);
    assert_eq!(inv.bags[0].name, "Backpack");
    assert_eq!(inv.bags[0].size, 16);
    assert_eq!(inv.bag_slot_count(0), 16);
}

#[test]
fn empty_slot_detection() {
    let slot = InventorySlot::default();
    assert!(slot.is_empty());
    let filled = InventorySlot {
        icon_fdid: 12345,
        count: 1,
        quality: ItemQuality::Rare,
        name: "Hearthstone".into(),
        ..Default::default()
    };
    assert!(!filled.is_empty());
}

#[test]
fn bag_borders_follow_retail_bag_item_quality_colors() {
    // BAG_ITEM_QUALITY_COLORS (ColorConstants.lua:21-30): no Poor entry; Common is
    // COMMON_GRAY_COLOR (GlobalColor 0xffa8a8a8).
    assert!(!ItemQuality::Poor.has_visible_border());
    assert_eq!(ItemQuality::Common.border_color(), "0.66,0.66,0.66,1.0");
    assert_eq!(ItemQuality::Uncommon.border_color(), "0.08,0.7,0.0,1.0");
    assert_eq!(ItemQuality::Rare.border_color(), "0.0,0.57,0.95,1.0");
    assert_eq!(ItemQuality::Epic.border_color(), "0.78,0.27,0.98,1.0");
    assert_eq!(ItemQuality::Legendary.border_color(), "1.0,0.5,0.0,1.0");
    assert_eq!(ItemQuality::Heirloom.border_color(), "0.0,0.8,1.0,1.0");
    assert_eq!(ItemQuality::from_id(0), ItemQuality::Poor);
    assert_eq!(ItemQuality::from_id(7), ItemQuality::Heirloom);
}

#[test]
fn total_free_slots_counts_empty() {
    let mut inv = InventoryState::default();
    assert_eq!(inv.total_free_slots(), 16);
    inv.slots[0][0].icon_fdid = 100;
    inv.slots[0][1].icon_fdid = 200;
    assert_eq!(inv.total_free_slots(), 14);
}

#[test]
fn total_slots_across_bags() {
    let mut inv = InventoryState::default();
    inv.bags.push(BagInfo {
        index: 1,
        name: "Mooncloth Bag".into(),
        size: 12,
        icon_fdid: 0,
    });
    inv.slots.push(vec![InventorySlot::default(); 12]);
    assert_eq!(inv.total_slots(), 28);
}

#[test]
fn slot_access_out_of_bounds_returns_none() {
    let inv = InventoryState::default();
    assert!(inv.slot(0, 0).is_some());
    assert!(inv.slot(0, 15).is_some());
    assert!(inv.slot(0, 16).is_none());
    assert!(inv.slot(5, 0).is_none());
}

#[test]
fn texture_fdids_are_nonzero() {
    assert_ne!(textures::BACKPACK_BG, 0);
    assert_ne!(textures::BACKPACK_BUTTON, 0);
    assert_ne!(textures::BAG_BG_1X4, 0);
    assert_ne!(textures::BAG_BG_4X4, 0);
    assert_ne!(textures::BAG_ICON_DEFAULT, 0);
}

#[test]
fn bag_background_selects_correct_size() {
    assert_eq!(bag_background_for_rows(1), textures::BAG_BG_1X4);
    assert_eq!(bag_background_for_rows(2), textures::BAG_BG_2X4);
    assert_eq!(bag_background_for_rows(3), textures::BAG_BG_3X4);
    assert_eq!(bag_background_for_rows(4), textures::BAG_BG_4X4);
    assert_eq!(bag_background_for_rows(6), textures::BAG_BG_4X4);
}

#[test]
fn slot_contents_across_bags() {
    let mut inv = InventoryState::default();
    inv.bags.push(BagInfo {
        index: 1,
        name: "Netherweave Bag".into(),
        size: 16,
        icon_fdid: 0,
    });
    inv.slots.push(vec![InventorySlot::default(); 16]);
    inv.slots[0][3] = InventorySlot {
        icon_fdid: 111,
        name: "Hearthstone".into(),
        count: 1,
        quality: ItemQuality::Common,
        ..Default::default()
    };
    inv.slots[1][0] = InventorySlot {
        icon_fdid: 222,
        name: "Ore".into(),
        count: 20,
        quality: ItemQuality::Uncommon,
        ..Default::default()
    };
    assert_eq!(inv.slot(0, 3).unwrap().name, "Hearthstone");
    assert_eq!(inv.slot(1, 0).unwrap().name, "Ore");
    assert_eq!(inv.slot(1, 0).unwrap().count, 20);
    assert!(inv.slot(0, 0).unwrap().is_empty());
}

#[test]
fn varied_bag_sizes() {
    let mut inv = InventoryState::default();
    inv.bags.push(BagInfo {
        index: 1,
        name: "Small Bag".into(),
        size: 8,
        icon_fdid: 0,
    });
    inv.slots.push(vec![InventorySlot::default(); 8]);
    inv.bags.push(BagInfo {
        index: 2,
        name: "Large Bag".into(),
        size: 20,
        icon_fdid: 0,
    });
    inv.slots.push(vec![InventorySlot::default(); 20]);

    assert_eq!(inv.bag_slot_count(0), 16);
    assert_eq!(inv.bag_slot_count(1), 8);
    assert_eq!(inv.bag_slot_count(2), 20);
    assert_eq!(inv.total_slots(), 44);
    assert_eq!(inv.total_free_slots(), 44);
}

#[test]
fn bag_slot_count_nonexistent_bag() {
    let inv = InventoryState::default();
    assert_eq!(inv.bag_slot_count(5), 0);
}

#[test]
fn bag_background_for_zero_rows() {
    assert_eq!(bag_background_for_rows(0), textures::BAG_BG_1X4);
}

fn stack(item_guid: u64, item_id: u32, count: u32) -> shared::protocol::ItemStack {
    shared::protocol::ItemStack {
        definition_source: shared::item_data::ItemDefinitionSource::Retail,
        item_guid,
        item_id,
        count,
        durability: None,
        soulbound: false,
    }
}

/// A new character's backpack as the server sends it: Linen Cloth x20, Peacebloom x10.
#[test]
fn server_snapshot_then_delta_drive_the_backpack() {
    use shared::protocol::{
        BagContents, BagSlotItem, InventoryDelta, InventorySlotChange, InventorySnapshot,
        ItemLocation,
    };
    crate::item_catalog::wait_for_item_catalog();
    let mut inv = InventoryState::default();
    inv.apply_snapshot(&InventorySnapshot {
        bags: vec![
            BagContents {
                bag: 0,
                size: 16,
                items: vec![
                    BagSlotItem {
                        slot: 0,
                        item: stack(41, 2589, 20),
                    },
                    BagSlotItem {
                        slot: 1,
                        item: stack(42, 2447, 10),
                    },
                ],
            },
            BagContents {
                bag: 1,
                size: 0,
                items: vec![],
            },
        ],
    });

    assert_eq!(inv.bags.len(), 1);
    assert_eq!(inv.bag_slot_count(0), 16);
    let linen = inv.slot(0, 0).unwrap();
    assert_eq!(
        (linen.item_guid, linen.item_id, linen.count),
        (41, 2589, 20)
    );
    assert_eq!(
        linen.icon_fdid,
        crate::item_icons::item_icon_fdid(2589).unwrap()
    );
    assert_eq!(inv.total_free_slots(), 14);

    // Selling 5 linen leaves 15; a bought vest lands in slot 2; peacebloom sold.
    inv.apply_delta(&InventoryDelta {
        changes: vec![
            InventorySlotChange {
                location: ItemLocation::Bag { bag: 0, slot: 0 },
                item: Some(stack(41, 2589, 15)),
            },
            InventorySlotChange {
                location: ItemLocation::Bag { bag: 0, slot: 1 },
                item: None,
            },
            InventorySlotChange {
                location: ItemLocation::Bag { bag: 0, slot: 2 },
                item: Some(stack(77, 2379, 1)),
            },
        ],
    });

    assert_eq!(inv.slot(0, 0).unwrap().count, 15);
    assert!(inv.slot(0, 1).unwrap().is_empty());
    assert_eq!(inv.slot(0, 2).unwrap().item_guid, 77);
    assert_eq!(inv.total_free_slots(), 14);
}

#[test]
fn server_stacks_take_name_and_quality_from_the_item_catalog() {
    crate::item_catalog::wait_for_item_catalog();
    let linen = stack_slot(&stack(41, 2589, 20));
    assert_eq!(
        (linen.name.as_str(), linen.quality),
        ("Linen Cloth", ItemQuality::Common)
    );
    let pelt = stack_slot(&stack(42, 4865, 2));
    assert_eq!(
        (pelt.name.as_str(), pelt.quality),
        ("Ruined Pelt", ItemQuality::Poor)
    );
}

#[test]
fn equipment_snapshot_and_deltas_fill_the_equipped_slots() {
    use shared::protocol::{
        EquipmentSlot, EquipmentSnapshot, EquippedItem, InventoryDelta, InventorySlotChange,
        ItemLocation,
    };
    crate::item_catalog::wait_for_item_catalog();
    let mut inv = InventoryState::default();
    inv.apply_equipment_snapshot(&EquipmentSnapshot {
        items: vec![EquippedItem {
            slot: EquipmentSlot::MainHand,
            item: stack(90, 25, 1),
        }],
    });
    assert_eq!(inv.equipped(EquipmentSlot::MainHand).unwrap().item_id, 25);
    assert_eq!(
        inv.equipped(EquipmentSlot::MainHand).unwrap().name,
        "Worn Shortsword"
    );

    // Unequipping the sword into backpack slot 10.
    inv.apply_delta(&InventoryDelta {
        changes: vec![
            InventorySlotChange {
                location: ItemLocation::Bag { bag: 0, slot: 9 },
                item: Some(stack(90, 25, 1)),
            },
            InventorySlotChange {
                location: ItemLocation::Equipment(EquipmentSlot::MainHand),
                item: None,
            },
        ],
    });
    assert!(inv.equipped(EquipmentSlot::MainHand).is_none());
    assert_eq!(inv.slot(0, 9).unwrap().item_guid, 90);
    assert_eq!(
        inv.item_at(ItemLocation::Bag { bag: 0, slot: 9 })
            .map(|slot| slot.item_id),
        Some(25)
    );
}
