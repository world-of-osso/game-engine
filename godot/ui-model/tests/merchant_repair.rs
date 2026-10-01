//! `MerchantRepairItemButton`: the repair cursor repairs the clicked item (MF.xml:280-316).
use std::path::PathBuf;

use game_engine_ui_model::merchant::{Click, MerchantEffect, MerchantSession};
use game_engine_ui_model::merchant_data::MerchantRequest;
use game_engine_ui_model::merchant_frame_component::{
    ACTION_REPAIR_ITEM, ACTION_TAB_BUYBACK, merchant_frame_screen,
};
use shared::protocol::{
    BagContents, BagSlotItem, EquipmentSlot, EquipmentSnapshot, EquippedItem, InventorySnapshot,
    ItemDurability, ItemLocation, ItemStack, VendorInventory,
};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

const NPC: u64 = 4_294_966_979;
/// Worn Shortsword 25, equipped and damaged.
const SWORD_GUID: u64 = 9_180_025;
const LINEN_GUID: u64 = 9_182_589;

fn session(can_repair: bool) -> MerchantSession {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut session = MerchantSession {
        money: 1000,
        repair_cost: 16,
        ..Default::default()
    };
    session.receive_inventory_snapshot(&linen_in_backpack());
    session.inventory.apply_equipment_snapshot(&damaged_sword());
    session.receive_inventory(
        VendorInventory {
            npc: NPC,
            can_repair,
            items: vec![],
        },
        "Fixture Vendor".into(),
    );
    session
}

fn linen_in_backpack() -> InventorySnapshot {
    InventorySnapshot {
        bags: vec![BagContents {
            bag: 0,
            size: 16,
            items: vec![BagSlotItem {
                slot: 1,
                item: stack(LINEN_GUID, 2589, 3, None),
            }],
        }],
    }
}

fn damaged_sword() -> EquipmentSnapshot {
    let durability = ItemDurability {
        current: 10,
        max: 20,
    };
    EquipmentSnapshot {
        items: vec![EquippedItem {
            slot: EquipmentSlot::MainHand,
            item: stack(SWORD_GUID, 25, 1, Some(durability)),
        }],
    }
}

fn stack(
    item_guid: u64,
    item_id: u32,
    count: u32,
    durability: Option<ItemDurability>,
) -> ItemStack {
    ItemStack {
        item_guid,
        item_id,
        count,
        durability,
        soulbound: false,
    }
}

fn repair(item_guid: u64) -> Option<Option<MerchantEffect>> {
    Some(Some(MerchantEffect::Request {
        npc: NPC,
        request: MerchantRequest::Repair {
            item_guid: Some(item_guid),
        },
    }))
}

fn registry(session: &MerchantSession) -> FrameRegistry {
    let mut shared = SharedContext::new();
    shared.insert(session.frame_state());
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    Screen::new(merchant_frame_screen).sync(&shared, &mut registry);
    registry
}

fn highlight_shown(session: &MerchantSession) -> bool {
    let registry = registry(session);
    let id = registry
        .get_by_name("MerchantRepairItemButtonHighlight")
        .expect("repair item highlight");
    registry.get(id).is_some_and(|frame| frame.visible)
}

const MAIN_HAND: ItemLocation = ItemLocation::Equipment(EquipmentSlot::MainHand);

#[test]
fn repair_item_button_toggles_the_repair_cursor_without_a_request() {
    let mut session = session(true);
    assert!(!highlight_shown(&session));
    assert_eq!(session.repair_click(MAIN_HAND), None, "no cursor yet");
    assert_eq!(session.click_frame(ACTION_REPAIR_ITEM, Click::LEFT), None);
    assert!(session.repair_mode);
    assert!(session.frame_state().repair_mode);
    assert!(
        highlight_shown(&session),
        "LockHighlight while InRepairMode"
    );
    assert_eq!(session.click_frame(ACTION_REPAIR_ITEM, Click::LEFT), None);
    assert!(
        !session.repair_mode,
        "a second click hides the repair cursor"
    );
}

#[test]
fn repair_cursor_on_the_equipped_sword_sends_its_guid_and_stays_shown() {
    let mut session = session(true);
    session.click_frame(ACTION_REPAIR_ITEM, Click::LEFT);
    let before = session.inventory.clone();
    assert_eq!(session.repair_click(MAIN_HAND), repair(SWORD_GUID));
    assert_eq!(session.inventory, before, "durability stays server-owned");
    assert!(
        session.repair_mode,
        "the repair cursor stays after a repair"
    );
    let linen = ItemLocation::Bag { bag: 0, slot: 1 };
    assert_eq!(session.repair_click(linen), repair(LINEN_GUID));
    let empty = ItemLocation::Equipment(EquipmentSlot::Head);
    assert_eq!(
        session.repair_click(empty),
        Some(None),
        "an empty slot sends nothing"
    );
}

#[test]
fn closing_the_frame_resets_the_repair_cursor() {
    let mut session = session(true);
    session.click_frame(ACTION_REPAIR_ITEM, Click::LEFT);
    assert_eq!(
        session.close(),
        Some(MerchantEffect::CloseInteraction { npc: NPC })
    );
    assert!(!session.repair_mode);
    assert_eq!(session.repair_click(MAIN_HAND), None);
}

#[test]
fn repair_item_button_exists_only_at_a_repairer_on_the_merchant_tab() {
    let mut vendor = session(false);
    assert!(
        registry(&vendor)
            .get_by_name("MerchantRepairItemButton")
            .is_none()
    );
    assert_eq!(vendor.click_frame(ACTION_REPAIR_ITEM, Click::LEFT), None);
    assert!(!vendor.repair_mode);
    let mut repairer = session(true);
    let tab1 = registry(&repairer);
    let frame = |name| {
        let id = tab1.get_by_name(name).unwrap_or_else(|| panic!("{name}"));
        tab1.get(id).unwrap()
    };
    let (item, all) = (
        frame("MerchantRepairItemButton"),
        frame("MerchantRepairAllButton"),
    );
    // RepairAll BOTTOMRIGHT at BOTTOMLEFT 118,33; the item button RIGHT at its LEFT −8
    // (MF.lua:952-953): 118 − 36 − 8 − 36 = 38.
    assert_eq!(all.position.left, Val::Px(82.0));
    assert_eq!(item.position.left, Val::Px(38.0));
    assert_eq!(item.position.top, all.position.top);
    assert_eq!(item.onclick.as_deref(), Some(ACTION_REPAIR_ITEM));
    repairer.click_frame(ACTION_TAB_BUYBACK, Click::LEFT);
    assert!(
        registry(&repairer)
            .get_by_name("MerchantRepairItemButton")
            .is_none()
    );
}
