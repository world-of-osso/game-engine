//! The first inventory snapshot can arrive while the item catalog is still loading on its
//! background thread. Applying it must not wait for that load: the client's main thread
//! applies server messages, and a wait freezes every frame until ItemSparse is parsed.
//! The items get their data once the load completes (Retail's `GET_ITEM_INFO_RECEIVED`).
//! Its own test binary, so the catalog is not already loaded by another test.
use std::path::PathBuf;
use std::time::{Duration, Instant};

use game_engine_ui_model::bag_data::{InventoryState, ItemQuality};
use game_engine_ui_model::game_tooltip::item::item_game_tooltip;
use game_engine_ui_model::item_catalog::{wait_for_item_catalog, warm_item_catalog};
use game_engine_ui_model::item_tooltip::{RED_FONT_COLOR, RETRIEVING_ITEM_INFO};
use shared::protocol::{
    BagContents, BagSlotItem, EquipmentSlot, EquipmentSnapshot, EquippedItem, InventorySnapshot,
    ItemStack,
};

/// `INV_Misc_QuestionMark`.
const QUESTION_MARK_FDID: u32 = 134_400;

fn stack(item_guid: u64, item_id: u32, count: u32) -> ItemStack {
    ItemStack {
        definition_source: shared::item_data::ItemDefinitionSource::Retail,
        item_guid,
        item_id,
        count,
        durability: None,
        soulbound: false,
    }
}

/// Linen Cloth x5 in backpack slot 3, Ruined Pelt x2 in slot 4.
fn bag_snapshot() -> InventorySnapshot {
    InventorySnapshot {
        bags: vec![BagContents {
            bag: 0,
            size: 16,
            items: vec![
                BagSlotItem {
                    slot: 3,
                    item: stack(77, 2589, 5),
                },
                BagSlotItem {
                    slot: 4,
                    item: stack(78, 4865, 2),
                },
            ],
        }],
    }
}

/// A Worn Shortsword in the main hand.
fn equipment_snapshot() -> EquipmentSnapshot {
    EquipmentSnapshot {
        items: vec![EquippedItem {
            slot: EquipmentSlot::MainHand,
            item: stack(79, 25, 1),
        }],
    }
}

#[test]
fn inventory_snapshot_during_catalog_load_does_not_wait_for_it() {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    warm_item_catalog();

    let started = Instant::now();
    let mut inventory = InventoryState::default();
    inventory.apply_snapshot(&bag_snapshot());
    inventory.apply_equipment_snapshot(&equipment_snapshot());
    let tooltip = item_game_tooltip(inventory.slot(0, 3).unwrap(), Some(1));
    let waited = started.elapsed();

    assert!(
        waited < Duration::from_millis(50),
        "applying the snapshots waited {waited:?} for the item catalog load"
    );
    // While loading: the items occupy their slots with Retail's unknown icon, no name,
    // and the tooltip says the data is on its way.
    let linen = inventory.slot(0, 3).unwrap();
    assert_eq!(
        (
            linen.item_id,
            linen.count,
            linen.icon_fdid,
            linen.name.as_str()
        ),
        (2589, 5, QUESTION_MARK_FDID, "")
    );
    assert!(!linen.is_empty());
    assert_eq!(inventory.total_free_slots(), 14);
    assert_eq!(
        (tooltip.content.title.as_str(), tooltip.content.title_color),
        (RETRIEVING_ITEM_INFO, RED_FONT_COLOR)
    );
    assert!(tooltip.content.lines.is_empty());

    wait_for_item_catalog();
    inventory.refresh_item_data();

    let linen = inventory.slot(0, 3).unwrap();
    assert_eq!(
        (
            linen.name.as_str(),
            linen.quality,
            linen.icon_fdid,
            linen.count
        ),
        ("Linen Cloth", ItemQuality::Common, 132_889, 5)
    );
    let pelt = inventory.slot(0, 4).unwrap();
    assert_eq!(
        (pelt.name.as_str(), pelt.quality, pelt.item_guid),
        ("Ruined Pelt", ItemQuality::Poor, 78)
    );
    let sword = inventory.equipped(EquipmentSlot::MainHand).unwrap();
    assert_eq!(
        (sword.name.as_str(), sword.item_guid),
        ("Worn Shortsword", 79)
    );
    assert_ne!(sword.icon_fdid, QUESTION_MARK_FDID);
    let tooltip = item_game_tooltip(inventory.slot(0, 3).unwrap(), Some(1));
    assert_eq!(tooltip.content.title, "Linen Cloth");
}
