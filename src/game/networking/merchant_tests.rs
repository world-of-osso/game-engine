use std::sync::mpsc;

use bevy::ecs::system::RunSystemOnce;
use game_engine::merchant_data::MerchantTab;
use game_engine::network_runtime::messages::{ConnectionSender, Inbox};
use game_engine::network_runtime::worker::NetworkCommand;
use shared::protocol::{BuybackItem, MerchantError, VendorItem};

use super::*;

/// Server entity bits of Godric Rothgar in these fixtures.
const GODRIC_SERVER: u64 = 0x0000_0001_0000_04BD;

fn fixture() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_message::<MerchantRequest>()
        .add_message::<NpcFrameEvent>()
        .init_resource::<MerchantState>()
        .init_resource::<UiErrors>()
        .init_resource::<ReplicationMirrorMap>()
        .init_resource::<ConnectionSender>()
        .init_resource::<Inbox<VendorInventory>>()
        .init_resource::<Inbox<BuybackList>>()
        .init_resource::<Inbox<MerchantFailed>>();
    let godric = app
        .world_mut()
        .spawn(Npc {
            template_id: 1213,
            name: "Godric Rothgar".into(),
        })
        .id();
    app.world_mut()
        .resource_mut::<ReplicationMirrorMap>()
        .insert(Entity::from_bits(GODRIC_SERVER), godric);
    app
}

fn vest() -> VendorItem {
    VendorItem {
        slot: 1,
        item_id: 2379,
        name: "Tarnished Chain Vest".into(),
        quality: 1,
        price: 89,
        stack_count: 1,
        max_stack: 1,
        num_available: None,
        usable: true,
        max_durability: None,
    }
}

fn deliver<M: lightyear::prelude::Message>(app: &mut App, messages: Vec<M>) {
    app.world_mut().insert_resource(Inbox::new(messages));
}

fn receive(app: &mut App) {
    app.world_mut().run_system_once(receive_merchant).unwrap();
}

#[test]
fn vendor_list_opens_the_frame_with_the_npc_name_and_buyback_fills_it() {
    let mut app = fixture();
    deliver(
        &mut app,
        vec![VendorInventory {
            npc: GODRIC_SERVER,
            can_repair: true,
            guild_repair_money: None,
            items: vec![vest()],
        }],
    );
    deliver(
        &mut app,
        vec![BuybackList {
            items: vec![BuybackItem {
                slot: 0,
                item_id: 2589,
                name: "Linen Cloth".into(),
                quality: 1,
                count: 5,
                price: 65,
            }],
        }],
    );
    receive(&mut app);

    let merchant = app.world().resource::<MerchantState>();
    assert_eq!(merchant.npc, Some(GODRIC_SERVER));
    assert_eq!(merchant.vendor_name, "Godric Rothgar");
    assert!(merchant.can_repair);
    assert_eq!(merchant.items, vec![vest()]);
    assert_eq!(merchant.tab, MerchantTab::Merchant);
    assert_eq!(merchant.buyback.len(), 1);
    assert_eq!(merchant.last_buyback().unwrap().price, 65);
}

#[test]
fn refusals_show_the_retail_error_and_interaction_end_closes_the_frame() {
    let mut app = fixture();
    deliver(
        &mut app,
        vec![VendorInventory {
            npc: GODRIC_SERVER,
            can_repair: true,
            guild_repair_money: None,
            items: vec![vest()],
        }],
    );
    deliver(
        &mut app,
        vec![MerchantFailed {
            npc: GODRIC_SERVER,
            error: MerchantError::NotEnoughMoney,
        }],
    );
    receive(&mut app);
    assert_eq!(
        app.world().resource::<UiErrors>().lines[0].text,
        "You don't have enough money."
    );

    // Another NPC's interaction ending leaves the vendor open.
    app.world_mut()
        .write_message(NpcFrameEvent::Closed { npc: 99 });
    app.world_mut()
        .run_system_once(close_on_interaction_end)
        .unwrap();
    assert!(app.world().resource::<MerchantState>().is_open());

    app.world_mut()
        .write_message(NpcFrameEvent::Closed { npc: GODRIC_SERVER });
    app.world_mut()
        .run_system_once(close_on_interaction_end)
        .unwrap();
    assert!(!app.world().resource::<MerchantState>().is_open());
}

#[test]
fn frame_requests_go_out_only_while_a_vendor_is_open() {
    let mut app = fixture();
    let (sender, commands) = mpsc::channel();
    app.insert_resource(ConnectionSender::new(Some(sender)));
    let buy = MerchantRequest::Buy {
        slot: 1,
        item_id: 2379,
        count: 1,
        destination: None,
    };

    app.world_mut().write_message(buy.clone());
    app.world_mut()
        .run_system_once(send_merchant_requests)
        .unwrap();
    assert!(commands.try_recv().is_err());
    app.world_mut()
        .resource_mut::<Messages<MerchantRequest>>()
        .clear();

    app.world_mut().resource_mut::<MerchantState>().npc = Some(GODRIC_SERVER);
    app.world_mut().write_message(buy);
    app.world_mut()
        .write_message(MerchantRequest::Repair { item_guid: None });
    app.world_mut().write_message(MerchantRequest::SellAllJunk);
    app.world_mut()
        .run_system_once(send_merchant_requests)
        .unwrap();
    for _ in 0..3 {
        assert!(matches!(commands.try_recv(), Ok(NetworkCommand::Apply(_))));
    }
    assert!(commands.try_recv().is_err());
}
