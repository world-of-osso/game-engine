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
    };
    assert!(!filled.is_empty());
}

#[test]
fn quality_border_colors() {
    assert!(!ItemQuality::Common.has_visible_border());
    assert!(ItemQuality::Uncommon.has_visible_border());
    assert!(ItemQuality::Rare.has_visible_border());
    assert!(ItemQuality::Epic.has_visible_border());
    assert!(ItemQuality::Legendary.has_visible_border());
    assert!(ItemQuality::Poor.has_visible_border());
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
fn bank_texture_fdids_are_nonzero() {
    assert_ne!(bank_textures::FRAME_CHROME, 0);
    assert_ne!(bank_textures::CORNER_TOP_LEFT, 0);
    assert_ne!(bank_textures::BACKGROUND, 0);
    assert_ne!(bank_textures::SLOT_BACKGROUND, 0);
    assert_ne!(bank_textures::LOCK_ICON, 0);
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
    };
    inv.slots[1][0] = InventorySlot {
        icon_fdid: 222,
        name: "Ore".into(),
        count: 20,
        quality: ItemQuality::Uncommon,
    };
    assert_eq!(inv.slot(0, 3).unwrap().name, "Hearthstone");
    assert_eq!(inv.slot(1, 0).unwrap().name, "Ore");
    assert_eq!(inv.slot(1, 0).unwrap().count, 20);
    assert!(inv.slot(0, 0).unwrap().is_empty());
}

#[test]
fn quality_border_color_values() {
    assert!(ItemQuality::Poor.border_color().starts_with("0.62"));
    assert!(ItemQuality::Uncommon.border_color().starts_with("0.12"));
    assert!(ItemQuality::Rare.border_color().starts_with("0.0,0.44"));
    assert!(ItemQuality::Epic.border_color().starts_with("0.64"));
    assert!(ItemQuality::Legendary.border_color().starts_with("1.0,0.5"));
}

#[test]
fn quality_common_border_invisible() {
    assert!(ItemQuality::Common.border_color().ends_with("0.0"));
    assert!(!ItemQuality::Common.has_visible_border());
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
