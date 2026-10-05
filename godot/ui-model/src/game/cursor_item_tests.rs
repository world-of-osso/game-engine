use shared::protocol::{VendorInventory, VendorItem};

use super::*;
use crate::bag_data::InventorySlot;

#[test]
fn cursor_rejects_same_id_replaced_by_a_different_definition_source() {
    use shared::item_data::ItemDefinitionSource::{Forever70205, Retail};
    let mut inventory = InventoryState::default();
    let mut throwing = item(81, 2947, 20, ItemQuality::Common, "Small Throwing Knife");
    throwing.definition_source = Forever70205;
    inventory.set_item(0, 0, throwing.clone());
    let mut cursor = CursorItem::split_from(&inventory, bag(0), 7);
    assert_eq!(cursor.definition_source(), Some(Forever70205));
    assert_eq!(cursor.icon_fdid(), Some(throwing.icon_fdid));
    throwing.definition_source = Retail;
    inventory.set_item(0, 0, throwing);
    cursor.clear_if_stale(&inventory, &vendor());
    assert!(cursor.is_empty());
}

const LINEN: u32 = 2589;
const SWORD: u32 = 25;
const PELT: u32 = 4865;

fn bag(slot: u8) -> ItemLocation {
    ItemLocation::Bag { bag: 0, slot }
}

fn item(
    item_guid: u64,
    item_id: u32,
    count: u32,
    quality: ItemQuality,
    name: &str,
) -> InventorySlot {
    InventorySlot {
        icon_fdid: 132_889,
        count,
        quality,
        name: name.into(),
        item_guid,
        item_id,
        ..Default::default()
    }
}

/// Backpack: 20 Linen Cloth in slot 0, a Worn Shortsword in slot 1, a Ruined Pelt
/// in slot 2; the shortsword also equipped in the main hand.
fn inventory() -> InventoryState {
    let mut inventory = InventoryState::default();
    inventory.set_item(
        0,
        0,
        item(41, LINEN, 20, ItemQuality::Common, "Linen Cloth"),
    );
    inventory.set_item(
        0,
        1,
        item(42, SWORD, 1, ItemQuality::Common, "Worn Shortsword"),
    );
    inventory.set_item(0, 2, item(43, PELT, 1, ItemQuality::Poor, "Ruined Pelt"));
    inventory.equipment.insert(
        EquipmentSlot::MainHand,
        item(90, SWORD, 1, ItemQuality::Common, "Worn Shortsword"),
    );
    inventory
}

fn vendor() -> MerchantState {
    let mut merchant = MerchantState::default();
    merchant.apply_inventory(
        VendorInventory {
            npc: 0x0000_0001_0000_04BD,
            can_repair: false,
            guild_repair_money: None,
            items: vec![VendorItem {
                slot: 3,
                item_id: 159,
                name: "Refreshing Spring Water".into(),
                quality: 1,
                price: 25,
                stack_count: 5,
                max_stack: 20,
                num_available: None,
                usable: true,
                max_durability: None,
            }],
        },
        "Innkeeper Farley".into(),
    );
    merchant
}

#[test]
fn picking_up_then_clicking_another_bag_slot_swaps_on_the_server() {
    let (inventory, merchant) = (inventory(), MerchantState::default());
    let mut cursor = CursorItem::Empty;

    assert_eq!(
        cursor.click(CursorTarget::Location(bag(0)), &inventory, &merchant),
        None
    );
    assert_eq!(cursor.source(), Some(bag(0)));
    assert_eq!(cursor.icon_fdid(), Some(132_889));

    let effect = cursor.click(CursorTarget::Location(bag(5)), &inventory, &merchant);
    assert_eq!(
        effect,
        Some(CursorEffect::Inventory(InventoryRequest::Swap(SwapItem {
            from: bag(0),
            to: bag(5),
        })))
    );
    assert!(cursor.is_empty());
}

#[test]
fn clicking_the_source_slot_puts_the_item_back_and_empty_slots_pick_up_nothing() {
    let (inventory, merchant) = (inventory(), MerchantState::default());
    let mut cursor = CursorItem::Empty;
    cursor.click(CursorTarget::Location(bag(7)), &inventory, &merchant);
    assert!(cursor.is_empty());

    cursor.click(CursorTarget::Location(bag(1)), &inventory, &merchant);
    assert_eq!(
        cursor.click(CursorTarget::Location(bag(1)), &inventory, &merchant),
        None
    );
    assert!(cursor.is_empty());
}

#[test]
fn a_bag_item_dropped_on_a_paperdoll_slot_equips_and_an_equipped_item_unequips() {
    let (inventory, merchant) = (inventory(), MerchantState::default());
    let main_hand = ItemLocation::Equipment(EquipmentSlot::MainHand);
    let mut cursor = CursorItem::Empty;

    cursor.click(CursorTarget::Location(bag(1)), &inventory, &merchant);
    assert_eq!(
        cursor.click(CursorTarget::Location(main_hand), &inventory, &merchant),
        Some(CursorEffect::Inventory(InventoryRequest::Swap(SwapItem {
            from: bag(1),
            to: main_hand,
        })))
    );

    cursor.click(CursorTarget::Location(main_hand), &inventory, &merchant);
    assert_eq!(cursor.source(), Some(main_hand));
    assert_eq!(
        cursor.click(CursorTarget::Location(bag(9)), &inventory, &merchant),
        Some(CursorEffect::Inventory(InventoryRequest::Swap(SwapItem {
            from: main_hand,
            to: bag(9),
        })))
    );
}

#[test]
fn a_split_stack_drops_as_split_item_and_is_sold_or_destroyed_by_its_count() {
    let (inventory, merchant) = (inventory(), vendor());
    assert!(CursorItem::split_from(&inventory, bag(0), 21).is_empty());
    assert!(CursorItem::split_from(&inventory, bag(0), 0).is_empty());
    assert!(matches!(
        CursorItem::split_from(&inventory, bag(0), 20),
        CursorItem::Inventory {
            count: 20,
            split: false,
            ..
        }
    ));

    let mut cursor = CursorItem::split_from(&inventory, bag(0), 7);
    assert_eq!(
        cursor.click(CursorTarget::Location(bag(4)), &inventory, &merchant),
        Some(CursorEffect::Inventory(InventoryRequest::Split(
            SplitItem {
                from: bag(0),
                to: bag(4),
                count: 7,
            }
        )))
    );

    let mut cursor = CursorItem::split_from(&inventory, bag(0), 7);
    assert_eq!(
        cursor.click(CursorTarget::MerchantFrame, &inventory, &merchant),
        Some(CursorEffect::Merchant(MerchantRequest::Sell {
            item_guid: 41,
            count: 7
        }))
    );

    let mut cursor = CursorItem::split_from(&inventory, bag(0), 7);
    cursor.click(CursorTarget::World, &inventory, &merchant);
    assert_eq!(
        cursor.destroy(),
        Some(InventoryRequest::Destroy(DestroyItem {
            location: bag(0),
            count: 7
        }))
    );
    assert!(cursor.is_empty());
}

#[test]
fn a_whole_stack_dropped_on_the_merchant_sells_all_of_it_only_while_a_vendor_is_open() {
    let inventory = inventory();
    let mut cursor = CursorItem::Empty;
    cursor.click(CursorTarget::Location(bag(0)), &inventory, &vendor());
    assert_eq!(
        cursor.click(CursorTarget::MerchantItem(4), &inventory, &vendor()),
        Some(CursorEffect::Merchant(MerchantRequest::Sell {
            item_guid: 41,
            count: 0
        }))
    );

    cursor.click(CursorTarget::Location(bag(0)), &inventory, &vendor());
    assert_eq!(
        cursor.click(
            CursorTarget::MerchantFrame,
            &inventory,
            &MerchantState::default()
        ),
        None
    );
}

#[test]
fn dropping_on_the_world_asks_first_and_rare_items_need_delete_typed() {
    let mut inventory = inventory();
    let merchant = MerchantState::default();
    let mut cursor = CursorItem::Empty;
    cursor.click(CursorTarget::Location(bag(2)), &inventory, &merchant);
    assert_eq!(
        cursor.click(CursorTarget::World, &inventory, &merchant),
        Some(CursorEffect::ConfirmDestroy(DestroyConfirm {
            name: "Ruined Pelt".into(),
            good: false
        }))
    );
    assert_eq!(
        cursor.source(),
        Some(bag(2)),
        "stays on the cursor while asked"
    );
    assert_eq!(
        cursor.destroy(),
        Some(InventoryRequest::Destroy(DestroyItem {
            location: bag(2),
            count: 0
        }))
    );

    inventory.set_item(0, 3, item(44, 1, 1, ItemQuality::Rare, "Martin Fury"));
    inventory.set_item(
        0,
        4,
        item(45, 2, 1, ItemQuality::Heirloom, "Bloodied Arcanite Reaper"),
    );
    for (slot, good) in [(3, true), (4, false)] {
        let mut cursor = CursorItem::Empty;
        cursor.click(CursorTarget::Location(bag(slot)), &inventory, &merchant);
        let Some(CursorEffect::ConfirmDestroy(confirm)) =
            cursor.click(CursorTarget::World, &inventory, &merchant)
        else {
            panic!("slot {slot} asks before destroying");
        };
        assert_eq!(confirm.good, good, "slot {slot}");
    }
}

#[test]
fn destroy_popup_for_an_ordinary_item_asks_yes_or_no_without_a_confirm_word() {
    let popup = destroy_popup(&DestroyConfirm {
        name: "Ruined Pelt".into(),
        good: false,
    });
    assert_eq!(
        popup,
        crate::ui::popup::PopupSpec {
            key: "DELETE_ITEM".into(),
            text: "Do you want to destroy Ruined Pelt?".into(),
            accept_label: "Yes".into(),
            cancel_label: Some("No".into()),
            timeout: None,
            confirm_text: None,
        }
    );
}

#[test]
fn destroy_popup_for_a_rare_item_requires_delete_without_a_timeout() {
    let popup = destroy_popup(&DestroyConfirm {
        name: "Martin Fury".into(),
        good: true,
    });
    assert_eq!(
        popup,
        crate::ui::popup::PopupSpec {
            key: "DELETE_GOOD_ITEM".into(),
            text:
                "Do you want to destroy Martin Fury?\n\nType \"DELETE\" into the field to confirm."
                    .into(),
            accept_label: "Yes".into(),
            cancel_label: Some("No".into()),
            timeout: None,
            confirm_text: Some("DELETE".into()),
        }
    );
}

#[test]
fn a_vendor_item_on_the_cursor_is_bought_into_the_bag_slot_it_is_dropped_on() {
    let (inventory, merchant) = (inventory(), vendor());
    let mut cursor = CursorItem::Empty;
    cursor.click(CursorTarget::MerchantItem(0), &inventory, &merchant);
    assert!(matches!(
        cursor,
        CursorItem::Merchant {
            slot: 3,
            item_id: 159,
            purchases: 1,
            ..
        }
    ));

    assert_eq!(
        cursor.click(CursorTarget::Location(bag(6)), &inventory, &merchant),
        Some(CursorEffect::Merchant(MerchantRequest::Buy {
            slot: 3,
            item_id: 159,
            count: 1,
            destination: Some(bag(6)),
        }))
    );
    assert!(cursor.is_empty());

    // Dropped back on the vendor or the world, it just leaves the cursor.
    let mut cursor = CursorItem::from_merchant(&merchant, 0, 4);
    assert_eq!(
        cursor.click(CursorTarget::World, &inventory, &merchant),
        None
    );
    assert!(cursor.is_empty());

    let mut buyback = vendor();
    buyback.set_tab(MerchantTab::Buyback);
    cursor.click(CursorTarget::MerchantItem(0), &inventory, &buyback);
    assert!(cursor.is_empty(), "buyback cells buy back instead");
}

#[test]
fn the_cursor_clears_when_its_item_leaves_the_slot_or_the_vendor_closes() {
    let mut inventory = inventory();
    let merchant = vendor();
    let mut cursor = CursorItem::Empty;
    cursor.click(CursorTarget::Location(bag(0)), &inventory, &merchant);
    cursor.clear_if_stale(&inventory, &merchant);
    assert!(!cursor.is_empty());
    inventory.clear_slot(0, 0);
    cursor.clear_if_stale(&inventory, &merchant);
    assert!(cursor.is_empty());

    let mut cursor = CursorItem::from_merchant(&merchant, 0, 1);
    cursor.clear_if_stale(&inventory, &MerchantState::default());
    assert!(cursor.is_empty());
}
