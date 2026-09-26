//! Owner-only experience messages: `PlayerXpUpdate` fills [`ExperienceState`] for the XP
//! bar, each `LogXpGain` prints its Retail chat line.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use game_engine::chat_data::{ChatChannelType, ChatMessage, ChatState};
use game_engine::experience_data::{ExperienceState, gain_chat_line};
use game_engine::network_runtime::messages::MessageReceivers;
use game_engine::network_runtime::replication::ReplicationMirrorMap;
use shared::components::Npc;
use shared::protocol::{LogXpGain, PlayerXpUpdate};

use crate::game_state::GameState;

pub struct ExperienceNetworkPlugin;

impl Plugin for ExperienceNetworkPlugin {
    fn build(&self, app: &mut App) {
        use game_engine::network_events::{add_message_route, register_message_handler};

        app.init_resource::<ExperienceState>()
            .init_resource::<ReplicationMirrorMap>();
        let handler =
            register_message_handler::<PlayerXpUpdate, _>(app, receive_experience, in_world);
        add_message_route::<LogXpGain>(app, handler);
        app.add_systems(OnExit(GameState::InWorld), reset_experience);
    }
}

fn in_world(world: &World) -> bool {
    *world.resource::<State<GameState>>().get() == GameState::InWorld
}

#[derive(SystemParam)]
struct ExperienceReceivers<'w, 's> {
    updates: MessageReceivers<'w, 's, PlayerXpUpdate>,
    gains: MessageReceivers<'w, 's, LogXpGain>,
}

/// Server entity bits → mirrored NPC name.
#[derive(SystemParam)]
struct VictimNames<'w, 's> {
    mirror: Res<'w, ReplicationMirrorMap>,
    npcs: Query<'w, 's, &'static Npc>,
}

impl VictimNames<'_, '_> {
    fn name(&self, bits: u64) -> Option<&str> {
        let main = self.mirror.server_to_main(Entity::try_from_bits(bits)?)?;
        self.npcs.get(main).ok().map(|npc| npc.name.as_str())
    }
}

fn receive_experience(
    mut receivers: ExperienceReceivers,
    victims: VictimNames,
    mut experience: ResMut<ExperienceState>,
    mut chat: ResMut<ChatState>,
) {
    for inbox in receivers.gains.iter_mut() {
        for gain in inbox.receive() {
            let victim = gain.victim.and_then(|bits| victims.name(bits));
            if let Some(text) = gain_chat_line(&gain, victim) {
                chat.add_message(ChatMessage {
                    channel_type: ChatChannelType::System,
                    channel_name: String::new(),
                    sender: String::new(),
                    text,
                    timestamp: game_engine::chat_data::now_timestamp(),
                });
            }
        }
    }
    for inbox in receivers.updates.iter_mut() {
        for update in inbox.receive() {
            experience.0 = Some(update);
        }
    }
}

fn reset_experience(mut experience: ResMut<ExperienceState>) {
    *experience = ExperienceState::default();
}

#[cfg(test)]
#[path = "experience_tests.rs"]
mod tests;
