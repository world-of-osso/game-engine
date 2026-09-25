//! Trainer networking: `TrainerList` fills [`TrainerState`] (the list opens the
//! frame and refreshes it after a purchase), `TrainerBuyFailed` shows its Retail
//! error text, the end of the trainer interaction closes it, and
//! [`TrainerRequest`]s go to the open trainer on `TrainerChannel`.

use bevy::prelude::*;
use game_engine::network_runtime::messages::{MessageReceivers, MessageSenders};
use game_engine::network_runtime::replication::ReplicationMirrorMap;
use game_engine::quest_runtime::NpcFrameEvent;
use game_engine::trainer_data::{TrainerRequest, TrainerState};
use game_engine::ui::ui_errors::UiErrors;
use shared::components::Npc;
use shared::protocol::{TrainerBuyFailed, TrainerBuySpell, TrainerChannel, TrainerList};

use crate::game_state::GameState;

pub struct TrainerNetworkPlugin;

impl Plugin for TrainerNetworkPlugin {
    fn build(&self, app: &mut App) {
        use game_engine::network_events::{add_message_route, register_message_handler};

        app.init_resource::<TrainerState>()
            .init_resource::<UiErrors>()
            .init_resource::<ReplicationMirrorMap>()
            .add_message::<TrainerRequest>()
            .add_message::<NpcFrameEvent>();
        let handler = register_message_handler::<TrainerList, _>(app, receive_trainer, in_world);
        add_message_route::<TrainerBuyFailed>(app, handler);
        app.add_systems(
            Update,
            (close_on_interaction_end, send_trainer_requests)
                .chain()
                .run_if(in_state(GameState::InWorld)),
        );
        app.add_systems(OnExit(GameState::InWorld), reset_trainer);
    }
}

fn in_world(world: &World) -> bool {
    *world.resource::<State<GameState>>().get() == GameState::InWorld
}

fn npc_name(mirror: &ReplicationMirrorMap, npcs: &Query<&Npc>, bits: u64) -> String {
    Entity::try_from_bits(bits)
        .and_then(|server| mirror.server_to_main(server))
        .and_then(|main| npcs.get(main).ok())
        .map(|npc| npc.name.clone())
        .unwrap_or_default()
}

fn receive_trainer(
    mut lists: MessageReceivers<TrainerList>,
    mut failures: MessageReceivers<TrainerBuyFailed>,
    mirror: Res<ReplicationMirrorMap>,
    npcs: Query<&Npc>,
    mut trainer: ResMut<TrainerState>,
    mut errors: ResMut<UiErrors>,
) {
    for inbox in lists.iter_mut() {
        for list in inbox.receive() {
            let name = npc_name(&mirror, &npcs, list.npc);
            trainer.apply_list(list, name);
        }
    }
    for inbox in failures.iter_mut() {
        for failed in inbox.receive() {
            if let Some(message) = failed.reason.message() {
                errors.add(message);
            }
        }
    }
}

/// `InteractionClosed` for the open trainer (walked away, NPC died, another NPC).
fn close_on_interaction_end(
    mut events: MessageReader<NpcFrameEvent>,
    mut trainer: ResMut<TrainerState>,
) {
    for event in events.read() {
        if let NpcFrameEvent::Closed { npc } = *event
            && trainer.npc == Some(npc)
        {
            trainer.close();
        }
    }
}

fn send_trainer_requests(
    mut requests: MessageReader<TrainerRequest>,
    trainer: Res<TrainerState>,
    mut senders: MessageSenders<TrainerBuySpell>,
) {
    let Some(npc) = trainer.npc else {
        requests.clear();
        return;
    };
    for request in requests.read() {
        for mut sender in senders.iter_mut() {
            sender.send::<TrainerChannel>(TrainerBuySpell {
                npc,
                spell_id: request.spell_id,
            });
        }
    }
}

fn reset_trainer(mut trainer: ResMut<TrainerState>) {
    trainer.close();
}
