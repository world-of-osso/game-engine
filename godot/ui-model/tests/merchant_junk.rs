//! Direct merchant services: requests do not change peer-owned bags or money.
use std::path::PathBuf;

use game_engine_ui_model::item_catalog::item_catalog_entry;
use game_engine_ui_model::merchant::{Click, MerchantEffect, MerchantSession};
use game_engine_ui_model::merchant_data::MerchantRequest;
use game_engine_ui_model::merchant_frame_component::{
    ACTION_REPAIR_ALL, ACTION_SELL_ALL_JUNK, ACTION_TAB_BUYBACK,
};
use shared::protocol::{BagContents, BagSlotItem, InventorySnapshot, ItemStack, VendorInventory};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

const NPC: u64 = 4_294_966_979;

fn session(poor: bool) -> MerchantSession {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut items = vec![stack(1, 2589, 3)];
    if poor {
        items.push(stack(0, 4865, 2));
    }
    let mut session = MerchantSession {
        money: 1000,
        repair_cost: 16,
        ..Default::default()
    };
    session.receive_inventory_snapshot(&InventorySnapshot {
        bags: vec![BagContents {
            bag: 0,
            size: 16,
            items,
        }],
    });
    session.receive_inventory(
        VendorInventory {
            npc: NPC,
            can_repair: true,
            items: vec![],
        },
        "Fixture Vendor".into(),
    );
    session
}

fn stack(slot: u32, item_id: u32, count: u32) -> BagSlotItem {
    BagSlotItem {
        slot,
        item: ItemStack {
            item_guid: 9_180_000 + u64::from(item_id),
            item_id,
            count,
            durability: None,
            soulbound: false,
        },
    }
}

fn effect(request: MerchantRequest) -> Option<MerchantEffect> {
    Some(MerchantEffect::Request { npc: NPC, request })
}

#[test]
fn poor_pelt_sends_direct_junk_request_without_optimistic_mutation() {
    let mut session = session(true);
    let pelt = item_catalog_entry(4865).expect("real Ruined Pelt catalog row");
    assert_eq!(pelt.quality, 0);
    assert!(pelt.sell_price > 0);
    assert_eq!(item_catalog_entry(2589).unwrap().quality, 1);
    assert!(session.frame_state().has_junk);
    let before = session.clone();
    assert_eq!(
        session.click_frame(ACTION_SELL_ALL_JUNK, Click::LEFT),
        effect(MerchantRequest::SellAllJunk),
        "one direct input must yield the exact open vendor request, not confirmation"
    );
    assert_eq!(session, before, "bags/money/split/vendor remain peer-owned");
}

#[test]
fn common_linen_is_not_junk_and_cannot_send_junk_request() {
    let mut session = session(false);
    assert!(!session.frame_state().has_junk);
    let before = session.clone();
    assert_eq!(session.click_frame(ACTION_SELL_ALL_JUNK, Click::LEFT), None);
    assert_eq!(session, before);
}

#[test]
fn buyback_tab_skips_junk_service_even_with_eligible_pelt() {
    let mut session = session(true);
    assert_eq!(session.click_frame(ACTION_TAB_BUYBACK, Click::LEFT), None);
    let mut shared = SharedContext::new();
    shared.insert(session.frame_state());
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    Screen::new(game_engine_ui_model::merchant_frame_component::merchant_frame_screen)
        .sync(&shared, &mut registry);
    assert!(registry.get_by_name("MerchantSellAllJunkButton").is_none());
    let before = session.clone();
    assert_eq!(session.click_frame(ACTION_SELL_ALL_JUNK, Click::LEFT), None);
    assert_eq!(session, before);
}

#[test]
fn repair_all_preserves_direct_none_guid_and_peer_owned_state() {
    let mut session = session(true);
    assert_eq!(session.frame_state().repair, Some(true));
    let before = session.clone();
    assert_eq!(
        session.click_frame(ACTION_REPAIR_ALL, Click::LEFT),
        effect(MerchantRequest::Repair { item_guid: None })
    );
    assert_eq!(session, before);
    session.repair_cost = 0;
    assert_eq!(session.frame_state().repair, Some(false));
}
