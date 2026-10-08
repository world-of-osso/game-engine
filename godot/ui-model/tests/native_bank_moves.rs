//! Retail Mainline BankFrame.lua: HandleItemPickup/OnDragStart/OnReceiveDrag
//! use PickupContainerItem; OnModifiedClick opens StackSplitFrame, SplitStack
//! calls SplitContainerItem. Bank tabs are container locations, not auto-deposits.
use game_engine_ui_model::bag_data::{InventoryRequest, InventoryState};
use game_engine_ui_model::bank::BankSession;
use game_engine_ui_model::bank_frame_component::{ACTION_TAB_PREFIX, bank_frame_screen};
use game_engine_ui_model::cursor_item::{CursorEffect, CursorItem, CursorTarget};
use game_engine_ui_model::merchant::Click;
use game_engine_ui_model::merchant_data::MerchantState;
use game_engine_ui_model::stack_split::{StackSplitOwner, StackSplitState};
use shared::item_data::ItemDefinitionSource;
use shared::protocol::*;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn stack(guid: u64, count: u32) -> ItemStack {
    ItemStack {
        item_guid: guid,
        item_id: 2589,
        definition_source: ItemDefinitionSource::Retail,
        count,
        durability: None,
        soulbound: true,
    }
}
fn contents() -> BankContents {
    let mut first = vec![None; BANK_TAB_SLOTS];
    first[0] = Some(stack(71, 20));
    BankContents {
        bank: BankType::Character,
        tabs: vec![
            BankTabView {
                name: "Cloth".into(),
                icon: 134400,
                deposit_flags: 0,
                slots: first,
            },
            BankTabView {
                name: "Supplies".into(),
                icon: 134400,
                deposit_flags: 0,
                slots: vec![None; BANK_TAB_SLOTS],
            },
        ],
        next_tab_cost: Some(1_000_000),
        money: None,
    }
}
fn opened() -> (BankSession, InventoryState) {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut session = BankSession::default();
    session.open_role(42, NpcRole::Banker);
    session.apply_contents(contents());
    let mut inventory = InventoryState::default();
    inventory.apply_snapshot(&InventorySnapshot {
        bags: vec![BagContents {
            bag: 0,
            size: 16,
            items: vec![BagSlotItem {
                slot: 2,
                item: stack(91, 5),
            }],
        }],
    });
    inventory.apply_bank_contents(&contents());
    (session, inventory)
}
fn swap(from: ItemLocation, to: ItemLocation) -> Option<CursorEffect> {
    Some(CursorEffect::Inventory(InventoryRequest::Swap(SwapItem {
        from,
        to,
    })))
}

#[test]
fn bankmoves_cursor_carries_bank_to_bag_and_bag_to_exact_bank_slot() {
    let (_, inventory) = opened();
    let merchant = MerchantState::default();
    let bank = ItemLocation::Bank { tab: 0, slot: 0 };
    let bag = ItemLocation::Bag { bag: 0, slot: 2 };
    let mut cursor = CursorItem::Empty;
    assert_eq!(
        cursor.click(CursorTarget::Location(bank), &inventory, &merchant),
        None
    );
    assert_eq!(cursor.source(), Some(bank));
    assert_eq!(
        cursor.click(CursorTarget::Location(bag), &inventory, &merchant),
        swap(bank, bag)
    );
    assert!(cursor.is_empty());
    cursor.click(CursorTarget::Location(bag), &inventory, &merchant);
    let target = ItemLocation::Bank { tab: 1, slot: 97 };
    assert_eq!(
        cursor.click(CursorTarget::Location(target), &inventory, &merchant),
        swap(bag, target)
    );
    // Cursor requests never mutate authoritative stacks.
    assert_eq!(inventory.item_at(bank).unwrap().count, 20);
}

#[test]
fn bankmoves_mounted_grid_routes_cursor_between_tabs_and_same_tab_slots() {
    let (mut session, inventory) = opened();
    let mut shared = SharedContext::new();
    shared.insert(session.frame_state());
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(bank_frame_screen).sync(&shared, &mut registry);
    let frame = registry
        .get(registry.get_by_name("BankFrameItem1").unwrap())
        .unwrap();
    let action = frame.onclick.as_deref().unwrap();
    let from = session.slot_location(action).unwrap();
    let mut cursor = CursorItem::Empty;
    let merchant = MerchantState::default();
    cursor.click(CursorTarget::Location(from), &inventory, &merchant);
    let same_tab = session.slot_location("bank_slot:3").unwrap();
    assert_eq!(
        cursor.click(CursorTarget::Location(same_tab), &inventory, &merchant),
        swap(from, same_tab)
    );
    cursor.click(CursorTarget::Location(from), &inventory, &merchant);
    session.click(
        &format!("{ACTION_TAB_PREFIX}1"),
        Click::LEFT,
        &Default::default(),
    );
    let to = session.slot_location(action).unwrap();
    assert_eq!(to, ItemLocation::Bank { tab: 1, slot: 0 });
    assert_eq!(cursor.source(), Some(from));
    assert_eq!(
        cursor.click(CursorTarget::Location(to), &inventory, &merchant),
        swap(from, to)
    );
}

#[test]
fn bankmoves_existing_split_picker_sends_split_for_bank_and_bag_destinations() {
    let (session, inventory) = opened();
    let from = session.slot_location("bank_slot:0").unwrap();
    let count = inventory.item_at(from).unwrap().count;
    let mut picker = StackSplitState::open(StackSplitOwner::Bag(from), count, 1).unwrap();
    picker.type_digit(7);
    for to in [
        ItemLocation::Bag { bag: 0, slot: 10 },
        ItemLocation::Bank { tab: 1, slot: 5 },
    ] {
        let mut cursor = CursorItem::split_from(&inventory, from, picker.split);
        assert_eq!(cursor.source(), Some(from));
        assert_eq!(
            cursor.click(
                CursorTarget::Location(to),
                &inventory,
                &MerchantState::default()
            ),
            Some(CursorEffect::Inventory(InventoryRequest::Split(
                SplitItem { from, to, count: 7 }
            )))
        );
    }
    // A bag split deposited onto a bank slot also stays a split, not a full swap.
    let bag = ItemLocation::Bag { bag: 0, slot: 2 };
    let mut cursor = CursorItem::split_from(&inventory, bag, 2);
    assert_eq!(
        cursor.click(
            CursorTarget::Location(from),
            &inventory,
            &MerchantState::default()
        ),
        Some(CursorEffect::Inventory(InventoryRequest::Split(
            SplitItem {
                from: bag,
                to: from,
                count: 2
            }
        )))
    );
}

#[test]
fn bankmoves_authoritative_refresh_and_close_clear_stale_cursor() {
    let (_, mut inventory) = opened();
    let from = ItemLocation::Bank { tab: 0, slot: 0 };
    let mut cursor = CursorItem::split_from(&inventory, from, 5);
    let mut refresh = contents();
    refresh.tabs[0].slots[0] = None;
    inventory.apply_bank_contents(&refresh);
    cursor.clear_if_stale(&inventory, &MerchantState::default());
    assert!(cursor.is_empty());
    // Character bank data must not be overwritten by the separate Warband bank.
    inventory.apply_bank_contents(&contents());
    let mut account = contents();
    account.bank = BankType::Account;
    account.tabs.clear();
    inventory.apply_bank_contents(&account);
    assert_eq!(inventory.item_at(from).unwrap().item_guid, 71);
    let mut closed = contents();
    closed.tabs.clear();
    inventory.apply_bank_contents(&closed);
    assert!(inventory.item_at(from).is_none());
}

#[test]
fn bankmoves_slot_routing_excludes_closed_purchase_and_warband_views() {
    let (mut session, _) = opened();
    assert_eq!(session.slot_location("bank_slot:98"), None);
    session.click(
        &format!("{ACTION_TAB_PREFIX}2"),
        Click::LEFT,
        &Default::default(),
    );
    assert_eq!(session.slot_location("bank_slot:0"), None);
    session.state.show(BankType::Account);
    assert_eq!(session.slot_location("bank_slot:0"), None);
    session.close_frame();
    assert_eq!(session.slot_location("bank_slot:0"), None);
}
