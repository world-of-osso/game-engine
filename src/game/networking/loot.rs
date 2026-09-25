//! Loot networking (docs/specs/loot-frame.md): `CorpseLootable` marks corpses
//! [`Lootable`], `LootResponse` / `LootSlotRemoved` / `LootClosed` drive
//! [`LootState`] (a taken slot prints its Retail loot line), `LootFailed` shows its
//! error, and [`LootRequest`]s go out on `LootChannel`.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use game_engine::chat_data::{ChatChannelType, ChatMessage, ChatState};
use game_engine::loot_state::{LootRequest, LootState, Lootable, loot_chat_text};
use game_engine::network_runtime::messages::{MessageReceivers, MessageSenders};
use game_engine::network_runtime::replication::ReplicationMirrorMap;
use game_engine::ui::ui_errors::UiErrors;
use lightyear::prelude::Message as NetworkMessage;
use shared::protocol::{
    CorpseLootable, LootChannel, LootClosed, LootFailed, LootRelease, LootResponse,
    LootSlotRemoved, LootSlotRequest, LootUnit,
};

use crate::game_state::GameState;

pub struct LootNetworkPlugin;

impl Plugin for LootNetworkPlugin {
    fn build(&self, app: &mut App) {
        use game_engine::network_events::{add_message_route, register_message_handler};

        app.init_resource::<LootState>()
            .init_resource::<UiErrors>()
            .init_resource::<ChatState>()
            .init_resource::<ReplicationMirrorMap>()
            .add_message::<LootRequest>();
        let handler = register_message_handler::<LootResponse, _>(app, receive_loot, in_world);
        add_message_route::<LootSlotRemoved>(app, handler);
        add_message_route::<LootClosed>(app, handler);
        add_message_route::<LootFailed>(app, handler);
        add_message_route::<CorpseLootable>(app, handler);
        app.add_systems(
            Update,
            send_loot_requests.run_if(in_state(GameState::InWorld)),
        );
        app.add_systems(OnExit(GameState::InWorld), reset_loot);
    }
}

fn in_world(world: &World) -> bool {
    *world.resource::<State<GameState>>().get() == GameState::InWorld
}

#[derive(SystemParam)]
struct LootReceivers<'w, 's> {
    responses: MessageReceivers<'w, 's, LootResponse>,
    removed: MessageReceivers<'w, 's, LootSlotRemoved>,
    closed: MessageReceivers<'w, 's, LootClosed>,
    failed: MessageReceivers<'w, 's, LootFailed>,
    lootable: MessageReceivers<'w, 's, CorpseLootable>,
}

fn receive_loot(
    mut receivers: LootReceivers,
    mirror: Res<ReplicationMirrorMap>,
    mut loot: ResMut<LootState>,
    mut errors: ResMut<UiErrors>,
    mut chat: ResMut<ChatState>,
    mut commands: Commands,
) {
    for inbox in receivers.lootable.iter_mut() {
        for update in inbox.receive() {
            mark_lootable(&mirror, &mut commands, update);
        }
    }
    for inbox in receivers.responses.iter_mut() {
        for response in inbox.receive() {
            loot.open(response);
        }
    }
    for inbox in receivers.removed.iter_mut() {
        for removed in inbox.receive() {
            if let Some(content) = loot.remove(removed.corpse, removed.slot) {
                chat.add_message(ChatMessage {
                    channel_type: ChatChannelType::System,
                    channel_name: String::new(),
                    sender: String::new(),
                    text: loot_chat_text(&content),
                    timestamp: game_engine::chat_data::now_timestamp(),
                });
            }
        }
    }
    for inbox in receivers.closed.iter_mut() {
        for closed in inbox.receive() {
            loot.close(closed.corpse);
        }
    }
    for inbox in receivers.failed.iter_mut() {
        for failed in inbox.receive() {
            errors.add(failed.error.message());
        }
    }
}

fn mark_lootable(mirror: &ReplicationMirrorMap, commands: &mut Commands, update: CorpseLootable) {
    let Some(corpse) =
        Entity::try_from_bits(update.corpse).and_then(|server| mirror.server_to_main(server))
    else {
        return;
    };
    let Ok(mut entity) = commands.get_entity(corpse) else {
        return;
    };
    if update.lootable {
        entity.insert(Lootable);
    } else {
        entity.remove::<Lootable>();
    }
}

#[derive(SystemParam)]
struct LootSenders<'w, 's> {
    open: MessageSenders<'w, 's, LootUnit>,
    take: MessageSenders<'w, 's, LootSlotRequest>,
    release: MessageSenders<'w, 's, LootRelease>,
}

fn send_loot_requests(
    mut requests: MessageReader<LootRequest>,
    mirror: Res<ReplicationMirrorMap>,
    loot: Res<LootState>,
    mut senders: LootSenders,
) {
    for request in requests.read() {
        match *request {
            LootRequest::Open { corpse, auto } => {
                let Some(server) = mirror.main_to_server(corpse) else {
                    warn!("right-clicked corpse {corpse} has no server entity");
                    continue;
                };
                let corpse = server.to_bits();
                send(&mut senders.open, LootUnit { corpse, auto });
            }
            LootRequest::Take { slot } => {
                if let Some(corpse) = loot.corpse {
                    send(&mut senders.take, LootSlotRequest { corpse, slot });
                }
            }
            LootRequest::Release => {
                if let Some(corpse) = loot.corpse {
                    send(&mut senders.release, LootRelease { corpse });
                }
            }
        }
    }
}

fn send<M: NetworkMessage + Clone>(senders: &mut MessageSenders<M>, message: M) {
    for mut sender in senders.iter_mut() {
        sender.send::<LootChannel>(message.clone());
    }
}

fn reset_loot(mut loot: ResMut<LootState>) {
    *loot = LootState::default();
}
