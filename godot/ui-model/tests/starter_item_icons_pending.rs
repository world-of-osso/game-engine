use std::path::PathBuf;

use game_engine_ui_model::bag_data::{stack_slot_in_catalog, InventoryState};
use game_engine_ui_model::character_frame::{paperdoll_slots, PAPERDOLL_BUTTONS};
use shared::item_data::ItemDefinitionSource;
use shared::protocol::{EquipmentSlot, ItemStack};

fn stack(item_guid: u64, item_id: u32) -> ItemStack {
    ItemStack {
        definition_source: ItemDefinitionSource::Retail,
        item_guid,
        item_id,
        count: 1,
        durability: None,
        soulbound: false,
        crafting_origin: None,
    }
}

#[test]
fn monk_starter_paperdoll_icons_resolve_without_item_catalog() {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();

    let starter_items = [
        (EquipmentSlot::MainHand, 79, 3661, 135_158),
        (EquipmentSlot::Chest, 80, 72019, 574_616),
        (EquipmentSlot::Legs, 81, 72020, 574_617),
        (EquipmentSlot::Wrist, 82, 77526, 574_614),
        (EquipmentSlot::Waist, 83, 77509, 574_613),
    ];
    let mut inventory = InventoryState::default();
    for &(slot, guid, item_id, expected_icon) in &starter_items {
        let item = stack_slot_in_catalog(&stack(guid, item_id), None);
        assert_eq!(item.icon_fdid, expected_icon, "item {item_id}");
        assert!(item.name.is_empty(), "item {item_id} awaits ItemSparse");
        inventory.equipment.insert(slot, item);
    }

    let views = paperdoll_slots(&inventory, None);
    for &(slot, _, item_id, expected_icon) in &starter_items {
        let view = PAPERDOLL_BUTTONS
            .iter()
            .zip(&views)
            .find_map(|(button, view)| (button.slot == slot).then_some(view))
            .unwrap_or_else(|| panic!("paperdoll has no {slot:?} button"));
        assert_eq!(view.icon_fdid, expected_icon, "paperdoll item {item_id}");
    }
}
