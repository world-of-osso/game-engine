//! Vendor networking: `VendorInventory` / `BuybackList` from the server fill
//! [`MerchantState`] (the list opens the frame), `MerchantFailed` shows its Retail
//! error text, the end of the vendor interaction closes it, and frame actions
//! ([`MerchantRequest`]) go to the open vendor on `MerchantChannel`.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use game_engine::merchant_data::{MerchantRequest, MerchantState};
use game_engine::network_runtime::messages::{MessageReceivers, MessageSenders};
use game_engine::network_runtime::replication::ReplicationMirrorMap;
use game_engine::quest_runtime::NpcFrameEvent;
use game_engine::ui::ui_errors::UiErrors;
use lightyear::prelude::Message as NetworkMessage;
use shared::components::Npc;
use shared::protocol::{
    BuyItem, BuybackItemRequest, BuybackList, MerchantChannel, MerchantFailed, RepairItem,
    SellAllJunkItems, SellItem, VendorInventory,
};

use crate::game_state::GameState;

pub struct MerchantNetworkPlugin;

impl Plugin for MerchantNetworkPlugin {
    fn build(&self, app: &mut App) {
        use game_engine::network_events::{add_message_route, register_message_handler};

        app.init_resource::<MerchantState>()
            .init_resource::<UiErrors>()
            .init_resource::<ReplicationMirrorMap>()
            .add_message::<MerchantRequest>()
            .add_message::<NpcFrameEvent>();
        let handler =
            register_message_handler::<VendorInventory, _>(app, receive_merchant, in_world);
        add_message_route::<BuybackList>(app, handler);
        add_message_route::<MerchantFailed>(app, handler);
        app.add_systems(
            Update,
            (close_on_interaction_end, send_merchant_requests)
                .chain()
                .run_if(in_state(GameState::InWorld)),
        );
        app.add_systems(OnExit(GameState::InWorld), reset_merchant);
    }
}

fn in_world(world: &World) -> bool {
    *world.resource::<State<GameState>>().get() == GameState::InWorld
}

#[derive(SystemParam)]
struct MerchantReceivers<'w, 's> {
    inventories: MessageReceivers<'w, 's, VendorInventory>,
    buybacks: MessageReceivers<'w, 's, BuybackList>,
    failures: MessageReceivers<'w, 's, MerchantFailed>,
}

fn npc_name(mirror: &ReplicationMirrorMap, npcs: &Query<&Npc>, bits: u64) -> String {
    Entity::try_from_bits(bits)
        .and_then(|server| mirror.server_to_main(server))
        .and_then(|main| npcs.get(main).ok())
        .map(|npc| npc.name.clone())
        .unwrap_or_default()
}

fn receive_merchant(
    mut receivers: MerchantReceivers,
    mirror: Res<ReplicationMirrorMap>,
    npcs: Query<&Npc>,
    mut merchant: ResMut<MerchantState>,
    mut errors: ResMut<UiErrors>,
) {
    for inbox in receivers.inventories.iter_mut() {
        for inventory in inbox.receive() {
            let name = npc_name(&mirror, &npcs, inventory.npc);
            merchant.apply_inventory(inventory, name);
        }
    }
    for inbox in receivers.buybacks.iter_mut() {
        for list in inbox.receive() {
            merchant.buyback = list.items;
        }
    }
    for inbox in receivers.failures.iter_mut() {
        for failed in inbox.receive() {
            errors.add(failed.error.message());
        }
    }
}

/// `InteractionClosed` for the open vendor (walked away, NPC died, another NPC).
fn close_on_interaction_end(
    mut events: MessageReader<NpcFrameEvent>,
    mut merchant: ResMut<MerchantState>,
) {
    for event in events.read() {
        if let NpcFrameEvent::Closed { npc } = *event
            && merchant.npc == Some(npc)
        {
            merchant.close();
        }
    }
}

#[derive(SystemParam)]
struct MerchantSenders<'w, 's> {
    buy: MessageSenders<'w, 's, BuyItem>,
    sell: MessageSenders<'w, 's, SellItem>,
    sell_all_junk: MessageSenders<'w, 's, SellAllJunkItems>,
    buyback: MessageSenders<'w, 's, BuybackItemRequest>,
    repair: MessageSenders<'w, 's, RepairItem>,
}

fn send_merchant_requests(
    mut requests: MessageReader<MerchantRequest>,
    merchant: Res<MerchantState>,
    mut senders: MerchantSenders,
) {
    let Some(npc) = merchant.npc else {
        requests.clear();
        return;
    };
    for request in requests.read() {
        match *request {
            MerchantRequest::Buy {
                slot,
                item_id,
                count,
                destination,
            } => send(
                &mut senders.buy,
                BuyItem {
                    npc,
                    slot,
                    item_id,
                    count,
                    destination,
                },
            ),
            MerchantRequest::SellAllJunk => {
                send(&mut senders.sell_all_junk, SellAllJunkItems { npc })
            }
            MerchantRequest::Sell { item_guid, count } => send(
                &mut senders.sell,
                SellItem {
                    npc,
                    item_guid,
                    count,
                },
            ),
            MerchantRequest::Buyback { slot } => {
                send(&mut senders.buyback, BuybackItemRequest { npc, slot })
            }
            MerchantRequest::Repair { item_guid } => send(
                &mut senders.repair,
                RepairItem {
                    npc,
                    item_guid,
                    guild_bank: false,
                },
            ),
            MerchantRequest::GuildRepairAll => send(
                &mut senders.repair,
                RepairItem {
                    npc,
                    item_guid: None,
                    guild_bank: true,
                },
            ),
        }
    }
}

fn send<M: NetworkMessage + Clone>(senders: &mut MessageSenders<M>, message: M) {
    for mut sender in senders.iter_mut() {
        sender.send::<MerchantChannel>(message.clone());
    }
}

fn reset_merchant(mut merchant: ResMut<MerchantState>) {
    merchant.close();
}

#[cfg(test)]
#[path = "merchant_tests.rs"]
mod tests;
