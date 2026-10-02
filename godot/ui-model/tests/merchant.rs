//! Native vendor session over the shared merchant data and screens, with the real world.db
//! vendors of Northshire (docs/specs/merchant-frame.md): Godric Rothgar 1213 (armor, repair)
//! and Brother Danil 152 (food and water sold in bundles of 5).

use std::path::PathBuf;

use game_engine_ui_model::merchant::{Click, MerchantEffect, MerchantSession, SplitKey};
use game_engine_ui_model::merchant_data::{MerchantRequest, MerchantTab};
use game_engine_ui_model::merchant_frame_component::{
    ACTION_BUYBACK_LAST, ACTION_CLOSE, ACTION_ITEM_PREFIX, ACTION_TAB_BUYBACK, ACTION_TAB_MERCHANT,
    CellTint,
};
use game_engine_ui_model::stack_split_frame_component::{ACTION_OKAY, ACTION_RIGHT};
use shared::protocol::{
    BagContents, BagSlotItem, BuybackItem, BuybackList, InventoryDelta, InventorySlotChange,
    InventorySnapshot, ItemLocation, ItemStack, VendorInventory, VendorItem,
};
use ui_toolkit::frame::{Dimension, WidgetData};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

const GODRIC: u64 = 1213 << 32 | 7;
const DANIL: u64 = 152 << 32 | 9;
/// `INV_Misc_QuestionMark`.
const UNKNOWN_ICON: u32 = 134_400;

fn data_root() {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    game_engine_ui_model::item_catalog::wait_for_item_catalog();
}

fn item(slot: u32, item_id: u32, name: &str, price: u32) -> VendorItem {
    VendorItem {
        slot,
        item_id,
        name: name.into(),
        quality: 1,
        price,
        stack_count: 1,
        max_stack: 1,
        num_available: None,
        usable: true,
    }
}

/// `content_npc_vendor` 1213 with `content_item.BuyPrice`.
fn godric() -> VendorInventory {
    VendorInventory {
        npc: GODRIC,
        can_repair: true,
        items: vec![
            item(0, 2379, "Tarnished Chain Vest", 89),
            item(1, 2381, "Tarnished Chain Leggings", 90),
            item(2, 2380, "Tarnished Chain Belt", 45),
            item(3, 2383, "Tarnished Chain Boots", 67),
            item(4, 2384, "Tarnished Chain Bracers", 45),
            item(5, 2385, "Tarnished Chain Gloves", 45),
            VendorItem {
                usable: false,
                ..item(6, 17184, "Small Shield", 36)
            },
            item(7, 2129, "Large Round Shield", 78),
        ],
    }
}

/// `content_npc_vendor` 152: 25 copper per 5 bread and 5 per 5 water, stacking to 20.
fn danil() -> VendorInventory {
    let bundle = |slot, item_id, name: &str, price| VendorItem {
        stack_count: 5,
        max_stack: 20,
        ..item(slot, item_id, name, price)
    };
    VendorInventory {
        npc: DANIL,
        can_repair: false,
        items: vec![
            bundle(0, 4540, "Tough Hunk of Bread", 25),
            bundle(1, 159, "Refreshing Spring Water", 5),
            item(2, 4496, "Small Brown Pouch", 500),
        ],
    }
}

fn stack(item_guid: u64, item_id: u32, count: u32) -> ItemStack {
    ItemStack {
        item_guid,
        item_id,
        count,
        durability: None,
        soulbound: false,
    }
}

/// A backpack with 5 Linen Cloth in slot 2.
fn linen_backpack() -> InventorySnapshot {
    InventorySnapshot {
        bags: vec![BagContents {
            bag: 0,
            size: 16,
            items: vec![BagSlotItem {
                slot: 2,
                item: stack(77, 2589, 5),
            }],
        }],
    }
}

fn session(inventory: VendorInventory, name: &str, money: u64) -> MerchantSession {
    data_root();
    let mut session = MerchantSession {
        money,
        ..Default::default()
    };
    session.receive_inventory_snapshot(&linen_backpack());
    session.receive_inventory(inventory, name.into());
    session
}

fn cell(index: usize) -> String {
    format!("{ACTION_ITEM_PREFIX}{index}")
}

fn request(npc: u64, request: MerchantRequest) -> Option<MerchantEffect> {
    Some(MerchantEffect::Request { npc, request })
}

fn render(session: &MerchantSession) -> FrameRegistry {
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    let mut shared = SharedContext::new();
    shared.insert(session.frame_state());
    shared.insert(session.bag_state());
    shared.insert(session.split_state(&registry));
    Screen::new(game_engine_ui_model::merchant::merchant_screen).sync(&shared, &mut registry);
    game_engine_ui_model::merchant::place_merchant_windows(&mut registry);
    registry
}

fn text(registry: &FrameRegistry, name: &str) -> String {
    let frame = registry
        .get(
            registry
                .get_by_name(name)
                .unwrap_or_else(|| panic!("no {name}")),
        )
        .unwrap();
    match &frame.widget_data {
        Some(WidgetData::FontString(text)) => text.text.clone(),
        other => panic!("{name} is {other:?}"),
    }
}

#[test]
fn godric_list_opens_the_frame_with_priced_icons_and_repair() {
    let mut session = session(godric(), "Godric Rothgar", 50);
    let state = session.frame_state();
    assert!(state.visible);
    assert_eq!(state.title, "Godric Rothgar");
    assert_eq!(state.cells.len(), 8);
    assert_eq!(state.page_text, None, "8 items fit one page");
    let vest = &state.cells[0];
    assert_eq!(
        (vest.name.as_str(), vest.price),
        ("Tarnished Chain Vest", 89)
    );
    assert!(vest.price_gray, "50 copper cannot afford 89");
    assert!(
        !state.cells[2].price_gray,
        "the 45 copper belt is affordable"
    );
    assert_ne!(vest.icon_fdid, UNKNOWN_ICON);
    assert_eq!(state.cells[6].tint, CellTint::Unusable);
    assert_eq!(state.repair, Some(false), "nothing damaged");
    session.repair_cost = 16;
    assert_eq!(session.frame_state().repair, Some(true));

    let registry = render(&session);
    assert_eq!(text(&registry, "MerchantFrameTitleText"), "Godric Rothgar");
    assert_eq!(text(&registry, "MerchantItem1Name"), "Tarnished Chain Vest");
    assert_eq!(text(&registry, "MerchantItem8Name"), "Large Round Shield");
    assert!(registry.get_by_name("MerchantRepairAllButton").is_some());
}

#[test]
fn right_click_buys_one_purchase_and_left_click_does_not() {
    let mut session = session(godric(), "Godric Rothgar", 500);
    assert_eq!(session.click_frame(&cell(0), Click::LEFT), None);
    assert_eq!(
        session.click_frame(&cell(0), Click::RIGHT),
        request(
            GODRIC,
            MerchantRequest::Buy {
                slot: 0,
                item_id: 2379,
                count: 1,
                destination: None,
            }
        )
    );
    assert_eq!(
        session.click_frame("merchant_repair_all", Click::LEFT),
        request(GODRIC, MerchantRequest::Repair { item_guid: None })
    );
}

#[test]
fn shift_click_on_bundled_bread_splits_by_the_purchase_and_enter_buys_it() {
    let mut session = session(danil(), "Brother Danil", 100);
    assert_eq!(session.click_frame(&cell(0), Click::SHIFT_LEFT), None);
    let split = session.split.clone().expect("bread opens the split frame");
    assert_eq!(
        (split.text(), split.total_text()),
        ("1 Stack".into(), Some("5 Total".into()))
    );
    assert_eq!(split.max, 20, "100 copper buys 20 bread, the max stack");
    session.split_key(SplitKey::Digit(3));
    assert_eq!(session.split.as_ref().unwrap().text(), "3 Stacks");
    assert_eq!(
        session.split_key(SplitKey::Enter),
        Some(request(
            DANIL,
            MerchantRequest::Buy {
                slot: 0,
                item_id: 4540,
                count: 3,
                destination: None,
            }
        ))
    );
    assert_eq!(session.split, None);
    assert_eq!(
        session.split_key(SplitKey::Enter),
        None,
        "closed frame owns no keys"
    );
}

#[test]
fn split_is_capped_by_money_and_okay_click_buys() {
    let mut session = session(danil(), "Brother Danil", 50);
    session.click_frame(&cell(0), Click::SHIFT_LEFT);
    assert_eq!(
        session.split.as_ref().unwrap().max,
        10,
        "50 copper: 10 bread"
    );
    assert_eq!(session.click_frame(ACTION_RIGHT, Click::LEFT), None);
    assert_eq!(session.click_frame(ACTION_RIGHT, Click::LEFT), None);
    assert_eq!(
        session.split.as_ref().unwrap().text(),
        "2 Stacks",
        "10 is the cap"
    );
    assert_eq!(
        session.click_frame(ACTION_OKAY, Click::LEFT),
        request(
            DANIL,
            MerchantRequest::Buy {
                slot: 0,
                item_id: 4540,
                count: 2,
                destination: None,
            }
        )
    );
    // The pouch does not stack: no split frame.
    session.click_frame(&cell(2), Click::SHIFT_LEFT);
    assert_eq!(session.split, None);
}

#[test]
fn right_clicking_a_bag_item_sells_the_stack_only_on_the_merchant_tab() {
    let mut session = session(danil(), "Brother Danil", 0);
    assert_eq!(session.click_bag("bag_slot:0:2", Click::LEFT), None);
    assert_eq!(
        session.click_bag("bag_slot:0:3", Click::RIGHT),
        None,
        "empty slot"
    );
    assert_eq!(
        session.click_bag("bag_slot:0:2", Click::RIGHT),
        request(
            DANIL,
            MerchantRequest::Sell {
                item_guid: 77,
                count: 0,
            }
        )
    );
    session.click_frame(ACTION_TAB_BUYBACK, Click::LEFT);
    assert_eq!(session.merchant.tab, MerchantTab::Buyback);
    assert_eq!(session.click_bag("bag_slot:0:2", Click::RIGHT), None);
}

#[test]
fn a_sale_shows_in_buyback_and_either_slot_buys_it_back() {
    let mut session = session(danil(), "Brother Danil", 0);
    session.receive_inventory_delta(&InventoryDelta {
        changes: vec![InventorySlotChange {
            location: ItemLocation::Bag { bag: 0, slot: 2 },
            item: None,
        }],
    });
    session.receive_buyback(BuybackList {
        items: vec![BuybackItem {
            slot: 0,
            item_id: 2589,
            name: "Linen Cloth".into(),
            quality: 1,
            count: 5,
            price: 65,
        }],
    });
    let state = session.frame_state();
    let last = state.last_buyback.expect("last sale slot");
    assert_eq!((last.name.as_str(), last.count), ("Linen Cloth", 5));
    assert_eq!(
        session.click_frame(ACTION_BUYBACK_LAST, Click::LEFT),
        request(DANIL, MerchantRequest::Buyback { slot: 0 })
    );
    session.click_frame(ACTION_TAB_BUYBACK, Click::LEFT);
    let state = session.frame_state();
    assert_eq!(state.title, "Merchant Buyback");
    assert_eq!(state.cells[0].name, "Linen Cloth");
    assert!(
        state.cells[0].price_gray,
        "no money for the 65 copper buyback"
    );
    assert_eq!(
        session.click_frame(&cell(0), Click::LEFT),
        request(DANIL, MerchantRequest::Buyback { slot: 0 })
    );
    session.click_frame(ACTION_TAB_MERCHANT, Click::LEFT);
    assert_eq!(session.frame_state().cells.len(), 3);
}

#[test]
fn close_button_ends_the_interaction_and_the_server_close_sends_nothing() {
    let mut session = session(danil(), "Brother Danil", 0);
    assert_eq!(
        session.click_frame(ACTION_CLOSE, Click::LEFT),
        Some(MerchantEffect::CloseInteraction { npc: DANIL })
    );
    assert!(!session.is_open());
    assert!(!session.frame_state().visible);
    assert_eq!(session.close(), None);

    session.receive_inventory(godric(), "Godric Rothgar".into());
    session.receive_interaction_closed(DANIL);
    assert!(session.is_open(), "another NPC's close keeps Godric open");
    session.receive_interaction_closed(GODRIC);
    assert!(!session.is_open());
    assert_eq!(session.click_frame(&cell(0), Click::RIGHT), None);
}

#[test]
fn the_backpack_shows_catalog_items_only_while_the_vendor_is_open() {
    let mut session = session(danil(), "Brother Danil", 0);
    let bags = session.bag_state();
    let backpack = &bags.bags[0];
    assert!(backpack.visible);
    assert_eq!(backpack.slots.len(), 16);
    // Linen Cloth 2589 uses Item.IconFileDataID 132889.
    assert_eq!(
        (backpack.slots[2].icon_fdid, backpack.slots[2].count),
        (132_889, 5)
    );
    assert_eq!(session.inventory.slots[0][2].name, "Linen Cloth");

    let registry = render(&session);
    let id = registry.get_by_name("ContainerFrame0").unwrap();
    let frame = registry.get(id).unwrap();
    let Dimension::Fixed(width) = frame.width else {
        panic!("backpack width")
    };
    assert_eq!(
        frame.position.left,
        ui_toolkit::layout_values::Val::Px(1280.0 - 16.0 - width)
    );
    assert_eq!(text(&registry, "ContainerFrame0Slot2Count"), "5");

    session.close();
    assert!(!session.bag_state().bags[0].visible);
}
