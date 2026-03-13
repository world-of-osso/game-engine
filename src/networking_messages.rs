use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::Zone;
use shared::protocol::{
    ChatChannel, ChatMessage, CollectionSnapshot, CombatChannel, CombatEvent, CombatEventType,
    CombatLogEventKindSnapshot, CombatLogSnapshot, GroupCommandResponse, GroupRoleSnapshot,
    GroupRosterSnapshot, InputChannel, LoadTerrain, PlayerInput, ProfessionSnapshot,
    QuestLogSnapshot, QuestRepeatability as QuestRepeatabilitySnapshot, SetTarget,
};

use crate::camera::{CharacterFacing, MovementState, Player};
use crate::networking::{ChatInput, ChatLog, CurrentZone, MAX_CHAT_LOG, MAX_COMBAT_LOG};
use crate::terrain::AdtManager;
use game_engine::status::{
    CollectionMountEntry, CollectionPetEntry, CollectionStatusSnapshot, CombatLogEntry,
    CombatLogEventKind, CombatLogStatusSnapshot, GroupMemberEntry, GroupRole, GroupStatusSnapshot,
    ProfessionRecipeEntry, ProfessionStatusSnapshot, QuestEntry, QuestLogStatusSnapshot,
    QuestObjectiveEntry, QuestRepeatability,
};
use game_engine::targeting::CurrentTarget;

/// Send a queued chat message to the server.
pub(crate) fn send_chat_message(
    mut chat_input: ResMut<ChatInput>,
    mut senders: Query<&mut MessageSender<ChatMessage>>,
) {
    let Some(msg) = chat_input.0.take() else {
        return;
    };
    for mut sender in senders.iter_mut() {
        sender.send::<ChatChannel>(msg.clone());
    }
}

/// Receive chat messages from the server and append to the chat log.
pub(crate) fn receive_chat_messages(
    mut receivers: Query<&mut MessageReceiver<ChatMessage>>,
    mut chat_log: ResMut<ChatLog>,
) {
    for mut receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            chat_log
                .messages
                .push((msg.sender, msg.content, msg.channel));
            if chat_log.messages.len() > MAX_CHAT_LOG {
                chat_log.messages.remove(0);
            }
        }
    }
}

/// Receive LoadTerrain messages from the server and initialize/stream the AdtManager.
pub(crate) fn receive_load_terrain(
    mut receivers: Query<&mut MessageReceiver<LoadTerrain>>,
    mut adt_manager: ResMut<AdtManager>,
) {
    for mut receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            let key = (msg.initial_tile_y, msg.initial_tile_x);
            if adt_manager.map_name.is_empty() {
                info!(
                    "Server requested terrain: {} tile ({}, {})",
                    msg.map_name, msg.initial_tile_y, msg.initial_tile_x
                );
                adt_manager.map_name = msg.map_name;
                adt_manager.initial_tile = key;
                adt_manager.server_requested.insert(key);
            } else if adt_manager.loaded.contains_key(&key)
                || adt_manager.pending.contains(&key)
                || adt_manager.failed.contains(&key)
            {
                continue;
            } else {
                debug!(
                    "Server requested additional tile ({}, {})",
                    msg.initial_tile_y, msg.initial_tile_x
                );
                adt_manager.server_requested.insert(key);
            }
        }
    }
}

pub(crate) fn receive_quest_log_snapshot(
    mut receivers: Query<&mut MessageReceiver<QuestLogSnapshot>>,
    mut snapshot: ResMut<QuestLogStatusSnapshot>,
) {
    for mut receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            snapshot.entries = msg.entries.into_iter().map(map_quest_entry).collect();
            snapshot.watched_quest_ids = msg.watched_quest_ids;
        }
    }
}

fn map_quest_entry(entry: shared::protocol::QuestEntrySnapshot) -> QuestEntry {
    QuestEntry {
        quest_id: entry.quest_id,
        title: entry.title,
        zone: entry.zone,
        completed: entry.completed,
        repeatability: map_repeatability(entry.repeatability),
        objectives: entry
            .objectives
            .into_iter()
            .map(|obj| QuestObjectiveEntry {
                text: obj.text,
                current: obj.current,
                required: obj.required,
                completed: obj.completed,
            })
            .collect(),
    }
}

fn map_repeatability(value: QuestRepeatabilitySnapshot) -> QuestRepeatability {
    match value {
        QuestRepeatabilitySnapshot::Normal => QuestRepeatability::Normal,
        QuestRepeatabilitySnapshot::Daily => QuestRepeatability::Daily,
        QuestRepeatabilitySnapshot::Weekly => QuestRepeatability::Weekly,
    }
}

pub(crate) fn receive_group_roster_snapshot(
    mut receivers: Query<&mut MessageReceiver<GroupRosterSnapshot>>,
    mut snapshot: ResMut<GroupStatusSnapshot>,
) {
    for mut receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            snapshot.is_raid = msg.is_raid;
            snapshot.ready_count = msg.ready_count;
            snapshot.total_count = msg.total_count;
            snapshot.members = msg.members.into_iter().map(map_group_member).collect();
        }
    }
}

fn map_group_member(member: shared::protocol::GroupMemberSnapshot) -> GroupMemberEntry {
    GroupMemberEntry {
        name: member.name,
        role: match member.role {
            GroupRoleSnapshot::Tank => GroupRole::Tank,
            GroupRoleSnapshot::Healer => GroupRole::Healer,
            GroupRoleSnapshot::Damage => GroupRole::Damage,
            GroupRoleSnapshot::None => GroupRole::None,
        },
        is_leader: member.is_leader,
        online: member.online,
        subgroup: member.subgroup,
    }
}

pub(crate) fn receive_group_command_response(
    mut receivers: Query<&mut MessageReceiver<GroupCommandResponse>>,
    mut snapshot: ResMut<GroupStatusSnapshot>,
) {
    for mut receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            snapshot.last_server_message = Some(msg.message);
        }
    }
}

pub(crate) fn receive_combat_log_snapshot(
    mut receivers: Query<&mut MessageReceiver<CombatLogSnapshot>>,
    mut snapshot: ResMut<CombatLogStatusSnapshot>,
) {
    for mut receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            snapshot.entries = msg.entries.into_iter().map(map_combat_entry).collect();
            if snapshot.entries.len() > MAX_COMBAT_LOG {
                let start = snapshot.entries.len() - MAX_COMBAT_LOG;
                snapshot.entries = snapshot.entries.split_off(start);
            }
        }
    }
}

fn map_combat_entry(entry: shared::protocol::CombatLogEntrySnapshot) -> CombatLogEntry {
    CombatLogEntry {
        kind: match entry.kind {
            CombatLogEventKindSnapshot::Damage => CombatLogEventKind::Damage,
            CombatLogEventKindSnapshot::Heal => CombatLogEventKind::Heal,
            CombatLogEventKindSnapshot::Interrupt => CombatLogEventKind::Interrupt,
            CombatLogEventKindSnapshot::AuraApplied => CombatLogEventKind::AuraApplied,
            CombatLogEventKindSnapshot::Death => CombatLogEventKind::Death,
        },
        source: entry.source,
        target: entry.target,
        spell: entry.spell,
        amount: entry.amount,
        aura: entry.aura,
        text: entry.text,
    }
}

fn combat_event_to_log_entry(msg: &CombatEvent) -> CombatLogEntry {
    let (kind, amount, text) = match msg.event_type {
        CombatEventType::MeleeDamage => (
            CombatLogEventKind::Damage,
            Some(msg.damage.round() as i32),
            format!(
                "{} hit {} for {}",
                msg.attacker,
                msg.target,
                msg.damage.round() as i32
            ),
        ),
        CombatEventType::Death => (
            CombatLogEventKind::Death,
            None,
            format!("{} died", msg.target),
        ),
        CombatEventType::Respawn => (
            CombatLogEventKind::AuraApplied,
            None,
            format!("{} respawned", msg.target),
        ),
    };
    CombatLogEntry {
        kind,
        source: msg.attacker.to_string(),
        target: msg.target.to_string(),
        spell: None,
        amount,
        aura: None,
        text,
    }
}

pub(crate) fn receive_combat_events(
    mut receivers: Query<&mut MessageReceiver<CombatEvent>>,
    mut snapshot: ResMut<CombatLogStatusSnapshot>,
) {
    for mut receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            let entry = combat_event_to_log_entry(&msg);
            append_combat_entry(&mut snapshot, entry);
        }
    }
}

pub(crate) fn append_combat_entry(snapshot: &mut CombatLogStatusSnapshot, entry: CombatLogEntry) {
    snapshot.entries.push(entry);
    if snapshot.entries.len() > MAX_COMBAT_LOG {
        let overflow = snapshot.entries.len() - MAX_COMBAT_LOG;
        snapshot.entries.drain(0..overflow);
    }
}

pub(crate) fn receive_collection_snapshot(
    mut receivers: Query<&mut MessageReceiver<CollectionSnapshot>>,
    mut snapshot: ResMut<CollectionStatusSnapshot>,
) {
    for mut receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            snapshot.mounts = msg
                .mounts
                .into_iter()
                .map(|mount| CollectionMountEntry {
                    mount_id: mount.mount_id,
                    name: mount.name,
                    known: mount.known,
                })
                .collect();
            snapshot.pets = msg
                .pets
                .into_iter()
                .map(|pet| CollectionPetEntry {
                    pet_id: pet.pet_id,
                    name: pet.name,
                    known: pet.known,
                })
                .collect();
        }
    }
}

pub(crate) fn receive_profession_snapshot(
    mut receivers: Query<&mut MessageReceiver<ProfessionSnapshot>>,
    mut snapshot: ResMut<ProfessionStatusSnapshot>,
) {
    for mut receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            snapshot.recipes = msg
                .recipes
                .into_iter()
                .map(|recipe| ProfessionRecipeEntry {
                    spell_id: recipe.spell_id,
                    profession: recipe.profession,
                    name: recipe.name,
                    craftable: recipe.craftable,
                    cooldown: recipe.cooldown,
                })
                .collect();
        }
    }
}

/// When CurrentTarget changes, send a SetTarget message to the server.
pub(crate) fn send_target_to_server(
    current: Res<CurrentTarget>,
    mut senders: Query<&mut MessageSender<SetTarget>>,
) {
    if !current.is_changed() {
        return;
    }
    let target_bits = current.0.map(|e| e.to_bits());
    let msg = SetTarget {
        target_entity: target_bits,
    };
    for mut sender in senders.iter_mut() {
        sender.send::<CombatChannel>(msg.clone());
    }
}

/// Watch for Zone component changes on the local player and update the CurrentZone resource.
pub(crate) fn track_player_zone(
    player_q: Query<&Zone, (With<Player>, Changed<Zone>)>,
    mut current_zone: ResMut<CurrentZone>,
) {
    if let Ok(zone) = player_q.single()
        && current_zone.zone_id != zone.id
    {
        info!("Entered zone {}", zone.id);
        current_zone.zone_id = zone.id;
    }
}

/// Send movement input to the server every frame.
pub(crate) fn send_player_input(
    player_q: Query<(&MovementState, &CharacterFacing), With<Player>>,
    mut senders: Query<&mut MessageSender<PlayerInput>>,
) {
    let Ok((movement, facing)) = player_q.single() else {
        return;
    };
    let direction = crate::networking::movement_to_direction(movement, facing);
    if direction == [0.0, 0.0, 0.0] && !movement.jumping {
        return;
    }
    let input = PlayerInput {
        direction,
        facing_yaw: facing.yaw,
        jumping: movement.jumping,
        running: movement.running,
    };
    for mut sender in senders.iter_mut() {
        sender.send::<InputChannel>(input.clone());
    }
}
