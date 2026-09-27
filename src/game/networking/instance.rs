//! Dungeon difficulty and instance lock messages (docs/specs/instances.md): the server's
//! `DungeonDifficultySet`, `WorldServerInfo` and `InstanceInfo` fill [`InstanceState`];
//! difficulty changes, saves, resets and expiries print their Retail system lines
//! (`ERR_DUNGEON_DIFFICULTY_CHANGED_S`, `INSTANCE_SAVED`, `INSTANCE_RESET_SUCCESS`,
//! `INSTANCE_RESET_FAILED`, `RAID_INSTANCE_EXPIRED`); [`InstanceCommand`]s go out on
//! `InstanceChannel`. Entering a map asks for the saved instances (`RequestRaidInfo`).

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use game_engine::chat_data::{
    ChatChannelType, ChatMessage as RuntimeChatMessage, ChatState, now_timestamp,
};
use game_engine::instance_state::{
    InstanceCatalog, InstanceCommand, InstanceState, difficulty_changed_text,
};
use game_engine::network_runtime::messages::{MessageReceivers, MessageSenders};
use lightyear::prelude::Message as NetworkMessage;
use shared::protocol::{
    DungeonDifficultySet, InstanceChannel, InstanceInfo, InstanceReset, InstanceResetFailed,
    InstanceResetFailedReason, InstanceSaveCreated, RaidInstanceMessage, RaidInstanceMessageType,
    RequestRaidInfo, ResetInstances, SetDungeonDifficulty, SetSavedInstanceExtend, WorldServerInfo,
};

use crate::game_state::GameState;

pub struct InstanceNetworkPlugin;

impl Plugin for InstanceNetworkPlugin {
    fn build(&self, app: &mut App) {
        use game_engine::network_events::{add_message_route, register_message_handler};

        let catalog = InstanceCatalog::load()
            .unwrap_or_else(|err| panic!("Difficulty.db2 / Map.db2 names: {err}"));
        app.insert_resource(catalog)
            .init_resource::<InstanceState>()
            .init_resource::<ChatState>()
            .add_message::<InstanceCommand>();
        let handler =
            register_message_handler::<DungeonDifficultySet, _>(app, receive_instance, |_| true);
        add_message_route::<WorldServerInfo>(app, handler);
        add_message_route::<InstanceInfo>(app, handler);
        add_message_route::<InstanceSaveCreated>(app, handler);
        add_message_route::<InstanceReset>(app, handler);
        add_message_route::<InstanceResetFailed>(app, handler);
        add_message_route::<RaidInstanceMessage>(app, handler);
        app.add_systems(
            Update,
            send_instance_commands.run_if(in_state(GameState::InWorld)),
        );
    }
}

#[derive(SystemParam)]
struct InstanceReceivers<'w, 's> {
    difficulties: MessageReceivers<'w, 's, DungeonDifficultySet>,
    world_infos: MessageReceivers<'w, 's, WorldServerInfo>,
    infos: MessageReceivers<'w, 's, InstanceInfo>,
    saves: MessageReceivers<'w, 's, InstanceSaveCreated>,
    resets: MessageReceivers<'w, 's, InstanceReset>,
    failed_resets: MessageReceivers<'w, 's, InstanceResetFailed>,
    raid_messages: MessageReceivers<'w, 's, RaidInstanceMessage>,
}

fn receive_instance(
    mut rx: InstanceReceivers,
    catalog: Res<InstanceCatalog>,
    time: Res<Time>,
    mut state: ResMut<InstanceState>,
    mut chat: ResMut<ChatState>,
    mut commands: MessageWriter<InstanceCommand>,
) {
    for inbox in rx.difficulties.iter_mut() {
        for set in inbox.receive() {
            // The login value only initializes; a change prints the Retail line.
            if state
                .dungeon_difficulty
                .is_some_and(|old| old != set.difficulty_id)
            {
                add_system_line(
                    &mut chat,
                    &difficulty_changed_text(&catalog, set.difficulty_id),
                );
            }
            state.dungeon_difficulty = Some(set.difficulty_id);
        }
    }
    for inbox in rx.world_infos.iter_mut() {
        for info in inbox.receive() {
            state.current_map = Some((info.map_id, info.difficulty_id));
            commands.write(InstanceCommand::RequestRaidInfo);
        }
    }
    for inbox in rx.infos.iter_mut() {
        for info in inbox.receive() {
            state.saved = info.locks;
            state.saved_at = time.elapsed_secs_f64();
        }
    }
    for inbox in rx.saves.iter_mut() {
        for _ in inbox.receive() {
            add_system_line(&mut chat, "You are now saved to this instance");
            commands.write(InstanceCommand::RequestRaidInfo);
        }
    }
    for inbox in rx.resets.iter_mut() {
        for reset in inbox.receive() {
            let name = catalog.map_name(reset.map_id);
            add_system_line(&mut chat, &format!("{name} has been reset."));
        }
    }
    for inbox in rx.failed_resets.iter_mut() {
        for failed in inbox.receive() {
            add_system_line(&mut chat, &reset_failed_text(&catalog, &failed));
        }
    }
    for inbox in rx.raid_messages.iter_mut() {
        for message in inbox.receive() {
            let name = catalog.map_name(message.map_id);
            match message.kind {
                RaidInstanceMessageType::Expired => add_system_line(
                    &mut chat,
                    &format!("Your instance lock for {name} has expired."),
                ),
            }
            commands.write(InstanceCommand::RequestRaidInfo);
        }
    }
}

/// `INSTANCE_RESET_FAILED`, `INSTANCE_RESET_FAILED_OFFLINE`, `INSTANCE_RESET_FAILED_ZONING`.
fn reset_failed_text(catalog: &InstanceCatalog, failed: &InstanceResetFailed) -> String {
    let name = catalog.map_name(failed.map_id);
    match failed.reason {
        InstanceResetFailedReason::PlayersInside => {
            format!("Cannot reset {name}.  There are players still inside the instance.")
        }
        InstanceResetFailedReason::PlayersOffline => {
            format!("Cannot reset {name}.  There are players offline in your party.")
        }
        InstanceResetFailedReason::PlayersZoning => format!(
            "Cannot reset {name}.  There are players in your party attempting to zone into an instance."
        ),
    }
}

/// Retail prints instance results as yellow system chat lines.
fn add_system_line(chat: &mut ChatState, text: &str) {
    chat.add_message(RuntimeChatMessage {
        channel_type: ChatChannelType::System,
        channel_name: String::new(),
        sender: String::new(),
        text: text.to_string(),
        timestamp: now_timestamp(),
    });
}

#[derive(SystemParam)]
struct InstanceSenders<'w, 's> {
    difficulty: MessageSenders<'w, 's, SetDungeonDifficulty>,
    raid_info: MessageSenders<'w, 's, RequestRaidInfo>,
    extend: MessageSenders<'w, 's, SetSavedInstanceExtend>,
    reset: MessageSenders<'w, 's, ResetInstances>,
}

fn send_instance_commands(
    mut commands: MessageReader<InstanceCommand>,
    mut senders: InstanceSenders,
) {
    for command in commands.read() {
        match *command {
            InstanceCommand::SetDungeonDifficulty(difficulty_id) => send(
                &mut senders.difficulty,
                SetDungeonDifficulty { difficulty_id },
            ),
            InstanceCommand::RequestRaidInfo => send(&mut senders.raid_info, RequestRaidInfo),
            InstanceCommand::SetExtended {
                map_id,
                difficulty_id,
                extend,
            } => send(
                &mut senders.extend,
                SetSavedInstanceExtend {
                    map_id,
                    difficulty_id,
                    extend,
                },
            ),
            InstanceCommand::ResetInstances => send(&mut senders.reset, ResetInstances),
        }
    }
}

fn send<M: NetworkMessage>(senders: &mut MessageSenders<M>, message: M) {
    let Some(mut sender) = senders.iter_mut().next() else {
        warn!(
            "[Instance] not connected; dropped {}",
            std::any::type_name::<M>()
        );
        return;
    };
    sender.send::<InstanceChannel>(message);
}

#[cfg(test)]
#[path = "instance_tests.rs"]
mod tests;
