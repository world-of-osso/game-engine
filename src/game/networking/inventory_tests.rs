use std::sync::mpsc;

use bevy::ecs::system::RunSystemOnce;
use game_engine::network_runtime::messages::{ConnectionSender, Inbox};
use game_engine::network_runtime::worker::NetworkCommand;
use shared::protocol::{
    EquipmentSlot, EquippedItem, InventoryErrorReason, ItemLocation, ItemStack,
};

use super::*;

fn fixture() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_message::<InventoryRequest>()
        .init_resource::<InventoryState>()
        .init_resource::<UiErrors>()
        .init_resource::<ConnectionSender>()
        .init_resource::<Inbox<InventorySnapshot>>()
        .init_resource::<Inbox<EquipmentSnapshot>>()
        .init_resource::<Inbox<InventoryDelta>>()
        .init_resource::<Inbox<InventoryError>>();
    app
}

#[test]
fn equipment_snapshot_fills_the_main_hand_and_errors_show_retail_text() {
    let mut app = fixture();
    app.world_mut()
        .insert_resource(Inbox::new(vec![EquipmentSnapshot {
            items: vec![EquippedItem {
                slot: EquipmentSlot::MainHand,
                item: ItemStack {
                    item_guid: 90,
                    item_id: 25,
                    count: 1,
                    durability: None,
                    soulbound: true,
                },
            }],
        }]));
    app.world_mut()
        .insert_resource(Inbox::new(vec![InventoryError {
            reason: InventoryErrorReason::WrongSlot,
            detail: None,
        }]));

    app.world_mut().run_system_once(receive_inventory).unwrap();

    let inventory = app.world().resource::<InventoryState>();
    let sword = inventory.equipped(EquipmentSlot::MainHand).unwrap();
    assert_eq!(
        (sword.item_guid, sword.name.as_str()),
        (90, "Worn Shortsword")
    );
    assert_eq!(
        app.world().resource::<UiErrors>().lines[0].text,
        "That item does not go in that slot."
    );
}

#[test]
fn a_swap_request_goes_out_on_the_inventory_channel() {
    let mut app = fixture();
    let (sender, commands) = mpsc::channel();
    app.insert_resource(ConnectionSender::new(Some(sender)));

    app.world_mut()
        .write_message(InventoryRequest::Swap(SwapItem {
            from: ItemLocation::Bag { bag: 0, slot: 0 },
            to: ItemLocation::Bag { bag: 0, slot: 4 },
        }));
    app.world_mut()
        .run_system_once(send_inventory_requests)
        .unwrap();

    assert!(matches!(commands.try_recv(), Ok(NetworkCommand::Apply(_))));
    assert!(commands.try_recv().is_err());
}
