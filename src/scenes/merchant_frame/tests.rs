use shared::protocol::{BuybackItem, VendorInventory, VendorItem};

use super::*;

const THARYNN_SERVER: u64 = 0x0000_0001_0000_0042;

/// Tharynn Bouden's list shape: 19 items, the 12th a limited pattern.
fn tharynn() -> MerchantState {
    let items = (0..19)
        .map(|slot| VendorItem {
            slot,
            item_id: 2320 + slot,
            name: format!("Goods {slot}"),
            quality: 1,
            price: 100 * (slot + 1),
            stack_count: 1,
            max_stack: 20,
            num_available: (slot == 11).then_some(1),
            usable: slot != 3,
        })
        .collect();
    let mut state = MerchantState::default();
    state.apply_inventory(
        VendorInventory {
            npc: THARYNN_SERVER,
            can_repair: false,
            items,
        },
        "Tharynn Bouden".into(),
    );
    state
}

fn linen_sale(slot: u8) -> BuybackItem {
    BuybackItem {
        slot,
        item_id: 2589,
        name: "Linen Cloth".into(),
        quality: 1,
        count: 5,
        price: 65,
    }
}

#[test]
fn nineteen_items_page_ten_at_a_time_with_affordability_and_stock() {
    let mut merchant = tharynn();
    let state = build_state(&merchant, true, 450, 0);

    assert!(state.visible);
    assert_eq!(state.title, "Tharynn Bouden");
    assert_eq!(state.cells.len(), 10);
    assert_eq!(state.page_text.as_deref(), Some("Page 1 of 2"));
    assert!(!state.prev_enabled && state.next_enabled);
    // 400 copper affordable with 450, 500 is not.
    assert!(!state.cells[3].price_gray);
    assert!(state.cells[4].price_gray);
    assert_eq!(state.cells[3].tint, CellTint::Unusable);
    assert_eq!(state.repair, None);

    merchant.next_page();
    let state = build_state(&merchant, true, 450, 0);
    assert_eq!(state.cells.len(), 9);
    assert_eq!(state.cells[1].name, "Goods 11");
    assert_eq!(state.cells[1].stock, Some(1));
    assert_eq!(state.page_text.as_deref(), Some("Page 2 of 2"));
}

#[test]
fn repair_all_enables_with_damage_and_the_last_sale_fills_the_buyback_slot() {
    let mut merchant = tharynn();
    merchant.can_repair = true;
    assert_eq!(build_state(&merchant, true, 0, 0).repair, Some(false));
    assert_eq!(build_state(&merchant, true, 0, 150).repair, Some(true));

    merchant.buyback = vec![linen_sale(0), linen_sale(1)];
    let state = build_state(&merchant, true, 0, 0);
    let last = state.last_buyback.unwrap();
    assert_eq!((last.price, last.count), (65, 5));
    assert!(last.price_gray);

    merchant.set_tab(MerchantTab::Buyback);
    let state = build_state(&merchant, true, 100, 0);
    assert_eq!(state.title, "Merchant Buyback");
    assert_eq!(state.cells.len(), 2);
    assert_eq!(state.page_text, None);
}

#[test]
fn right_clicks_buy_and_buy_back_while_left_clicks_on_items_do_nothing() {
    let mut merchant = tharynn();
    let mut manager = WindowManager::default();
    merchant.next_page();

    let left = dispatch_action(
        "merchant_item:1",
        MouseButton::Left,
        &mut merchant,
        &mut manager,
    );
    assert_eq!(left, None);
    let right = dispatch_action(
        "merchant_item:1",
        MouseButton::Right,
        &mut merchant,
        &mut manager,
    );
    assert_eq!(
        right,
        Some(MerchantRequest::Buy {
            slot: 11,
            item_id: 2331,
            count: 1
        })
    );

    merchant.buyback = vec![linen_sale(0), linen_sale(4)];
    assert_eq!(
        dispatch_action(
            ACTION_BUYBACK_LAST,
            MouseButton::Left,
            &mut merchant,
            &mut manager
        ),
        Some(MerchantRequest::Buyback { slot: 4 })
    );
    dispatch_action(
        ACTION_TAB_BUYBACK,
        MouseButton::Left,
        &mut merchant,
        &mut manager,
    );
    assert_eq!(
        dispatch_action(
            "merchant_item:0",
            MouseButton::Right,
            &mut merchant,
            &mut manager
        ),
        Some(MerchantRequest::Buyback { slot: 0 })
    );
    assert_eq!(
        dispatch_action(
            ACTION_REPAIR_ALL,
            MouseButton::Left,
            &mut merchant,
            &mut manager
        ),
        Some(MerchantRequest::Repair { item_guid: None })
    );
}

#[derive(Resource)]
struct SyncSystem(bevy::ecs::system::SystemId);

fn window_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_message::<NpcInteractionRequest>()
        .init_resource::<MerchantState>()
        .init_resource::<WindowManager>();
    let id = app.world_mut().register_system(sync_merchant_window);
    app.insert_resource(SyncSystem(id));
    app
}

fn run_sync(app: &mut App) {
    let id = app.world().resource::<SyncSystem>().0;
    app.world_mut().run_system(id).unwrap();
}

fn close_requests(app: &mut App) -> Vec<NpcInteractionRequest> {
    app.world_mut()
        .resource_mut::<Messages<NpcInteractionRequest>>()
        .drain()
        .collect()
}

#[test]
fn vendor_list_opens_the_window_and_closing_it_ends_the_interaction() {
    let mut app = window_app();
    *app.world_mut().resource_mut::<MerchantState>() = tharynn();
    run_sync(&mut app);
    assert!(
        app.world()
            .resource::<WindowManager>()
            .is_open(WindowId::Merchant)
    );
    assert!(
        app.world()
            .resource::<WindowManager>()
            .is_open(WindowId::Bag(0))
    );

    app.world_mut()
        .resource_mut::<WindowManager>()
        .close(WindowId::Merchant);
    run_sync(&mut app);
    assert!(!app.world().resource::<MerchantState>().is_open());
    assert!(
        !app.world()
            .resource::<WindowManager>()
            .is_open(WindowId::Bag(0))
    );
    assert_eq!(
        close_requests(&mut app),
        vec![NpcInteractionRequest::Close {
            npc: THARYNN_SERVER
        }]
    );
}

#[test]
fn the_server_ending_the_interaction_closes_the_window_without_a_request() {
    let mut app = window_app();
    *app.world_mut().resource_mut::<MerchantState>() = tharynn();
    run_sync(&mut app);

    app.world_mut().resource_mut::<MerchantState>().close();
    run_sync(&mut app);
    assert!(
        !app.world()
            .resource::<WindowManager>()
            .is_open(WindowId::Merchant)
    );
    assert!(close_requests(&mut app).is_empty());
}
