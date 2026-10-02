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
            max_durability: None,
        })
        .collect();
    let mut state = MerchantState::default();
    state.apply_inventory(
        VendorInventory {
            npc: THARYNN_SERVER,
            can_repair: false,
            guild_repair_money: None,
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
fn right_clicks_buy_any_click_buys_back_and_left_clicks_leave_vendor_items_to_the_cursor() {
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
            count: 1,
            destination: None,
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
    for button in [MouseButton::Right, MouseButton::Left] {
        assert_eq!(
            dispatch_action("merchant_item:1", button, &mut merchant, &mut manager),
            Some(MerchantRequest::Buyback { slot: 4 })
        );
    }
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

#[test]
fn merchant_money_follows_replicated_gold_on_the_next_frame() {
    use bevy::state::app::StatesPlugin;

    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin))
        .insert_state(GameState::InWorld)
        .init_resource::<MerchantState>()
        .init_resource::<WindowManager>()
        .init_resource::<CharacterStatsSnapshot>()
        .init_resource::<InventoryState>()
        .init_resource::<crate::networking::CharacterList>()
        .init_resource::<crate::networking::SelectedCharacterId>()
        .init_resource::<crate::networking::CurrentZone>();
    crate::status_sync::register_character_stats_sync(&mut app);
    *app.world_mut().resource_mut::<MerchantState>() = tharynn();
    let player = app
        .world_mut()
        .spawn((
            crate::networking::LocalPlayer,
            shared::components::Gold(250),
        ))
        .id();
    let money = |app: &mut App| {
        app.world_mut()
            .run_system_cached(|view: MerchantView| view.state().money)
            .unwrap()
    };
    app.update();
    assert_eq!(money(&mut app), 250);

    // Buying spends 2s 40c.
    app.world_mut()
        .get_mut::<shared::components::Gold>(player)
        .unwrap()
        .0 = 10;
    app.update();
    assert_eq!(money(&mut app), 10);
}

#[test]
fn junk_is_a_poor_bag_item_a_vendor_buys() {
    use game_engine::bag_data::{InventorySlot, InventoryState};
    let mut inventory = InventoryState::default();
    // Linen Cloth is common: not junk.
    inventory.set_item(
        0,
        0,
        InventorySlot {
            icon_fdid: 132_889,
            count: 5,
            item_id: 2589,
            ..Default::default()
        },
    );
    assert!(!has_junk(&inventory));
    // Ruined Pelt (4865): poor, sells for 5 copper.
    inventory.set_item(
        0,
        1,
        InventorySlot {
            icon_fdid: 134_366,
            count: 1,
            quality: ItemQuality::Poor,
            item_id: 4865,
            ..Default::default()
        },
    );
    assert!(has_junk(&inventory));
}

#[test]
fn sell_all_junk_asks_first_and_sells_only_on_yes() {
    use bevy::ecs::system::RunSystemOnce;
    use game_engine::ui::popup::PopupId;
    let mut app = App::new();
    app.add_message::<PopupResult>()
        .add_message::<MerchantRequest>()
        .insert_resource(tharynn());
    assert_eq!(sell_all_junk_popup().key, SELL_ALL_JUNK_POPUP);
    for outcome in [PopupOutcome::Cancelled, PopupOutcome::Accepted] {
        app.world_mut().write_message(PopupResult {
            id: PopupId(1),
            key: SELL_ALL_JUNK_POPUP.into(),
            outcome,
        });
    }
    app.world_mut()
        .run_system_once(sell_junk_on_confirm)
        .unwrap();
    let sent: Vec<_> = app
        .world_mut()
        .resource_mut::<Messages<MerchantRequest>>()
        .drain()
        .collect();
    assert_eq!(sent, vec![MerchantRequest::SellAllJunk]);
}
