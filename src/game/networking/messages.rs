use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::Zone;
use shared::protocol::{
    AchievementStateUpdate, ChatChannel, ChatMessage, CollectionStateUpdate, CombatChannel,
    CombatEvent, CombatEventType, CombatLogEventKindSnapshot, CombatLogSnapshot, DeathStateUpdate,
    DuelStateUpdate, EmoteEvent, EmoteIntent, GroupCommandResponse, GroupRoleSnapshot,
    GroupRosterSnapshot, GuildVaultSnapshot, InputChannel, InspectStateUpdate,
    InventorySearchResultSnapshot, LoadTerrain, PlayerInput, ProfessionSnapshot,
    ProfessionStateUpdate, QuestLogSnapshot, QuestRepeatability as QuestRepeatabilitySnapshot,
    ReputationStateUpdate, RestAreaKindSnapshot, RestStateUpdate, SetTarget, StorageItemSnapshot,
    TalentStateUpdate, WarbankSnapshot, WorldMapStateUpdate,
};

use crate::camera::{CharacterFacing, MovementState, Player};
use crate::networking::{
    ChatInput, ChatLog, CurrentZone, EmoteInput, MAX_CHAT_LOG, MAX_COMBAT_LOG,
};
use crate::networking_auth::SelectedCharacterId;
use crate::terrain::AdtManager;
use game_engine::achievement::{
    AchievementToastState, apply_achievement_state_update as map_achievement_state_update,
};
use game_engine::chat_data::{
    ChatChannelType, ChatMessage as RuntimeChatMessage, ChatState, WhisperState,
};
use game_engine::collection::apply_collection_state_update as map_collection_state_update;
use game_engine::death::apply_death_state_update as map_death_state_update;
use game_engine::duel::apply_duel_state_update as map_duel_state_update;
use game_engine::ignore_list::is_ignored as is_ignored_sender;
use game_engine::inspect::apply_inspect_state_update as map_inspect_state_update;
use game_engine::status::{
    AchievementsStatusSnapshot, CollectionStatusSnapshot, CombatLogEntry, CombatLogEventKind,
    CombatLogStatusSnapshot, DeathStatusSnapshot, DuelStatusSnapshot, GroupMemberEntry, GroupRole,
    GroupStatusSnapshot, GuildVaultStatusSnapshot, IgnoreListStatusSnapshot, InspectStatusSnapshot,
    InventoryItemEntry, InventorySearchSnapshot, ProfessionRecipeEntry, ProfessionSkillEntry,
    ProfessionSkillUpEntry, ProfessionStatusSnapshot, QuestEntry, QuestLogStatusSnapshot,
    QuestObjectiveEntry, QuestRepeatability, ReputationEntry, ReputationsStatusSnapshot,
    RestAreaKindEntry, StorageItemEntry, TalentNodeEntry, TalentSpecTabEntry, TalentStatusSnapshot,
    WarbankStatusSnapshot,
};
use game_engine::targeting::CurrentTarget;
use game_engine::world_map::apply_world_map_state_update as map_world_map_state_update;

/// Send a queued chat message to the server.
pub(crate) fn send_chat_message(
    mut chat_input: ResMut<ChatInput>,
    mut whisper_state: ResMut<WhisperState>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    mut senders: Query<&mut MessageSender<ChatMessage>>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) {
        chat_input.0 = None;
        return;
    }
    let Some(msg) = chat_input.0.take() else {
        return;
    };
    apply_outgoing_chat_message(&msg, &mut whisper_state);
    for mut sender in senders.iter_mut() {
        sender.send::<ChatChannel>(msg.clone());
    }
}

/// Send a queued emote intent to the server.
pub(crate) fn send_emote_intent(
    mut emote_input: ResMut<EmoteInput>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    mut senders: Query<&mut MessageSender<EmoteIntent>>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) {
        emote_input.0 = None;
        return;
    }
    let Some(intent) = emote_input.0.take() else {
        return;
    };
    for mut sender in senders.iter_mut() {
        sender.send::<ChatChannel>(intent.clone());
    }
}

/// Receive chat messages from the server and append to the chat log.
pub(crate) fn receive_chat_messages(
    mut receivers: Query<&mut MessageReceiver<ChatMessage>>,
    mut chat_log: ResMut<ChatLog>,
    mut chat_state: ResMut<ChatState>,
    mut whisper_state: ResMut<WhisperState>,
    ignore_list: Res<IgnoreListStatusSnapshot>,
    selected_character: Option<Res<SelectedCharacterId>>,
) {
    let local_name = selected_character
        .as_deref()
        .and_then(|selected| selected.character_name.as_deref());
    for mut receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            apply_incoming_chat_message(
                &msg,
                local_name,
                &ignore_list,
                &mut chat_log,
                &mut chat_state,
                &mut whisper_state,
            );
        }
    }
}

/// Receive social emote events and attach a transient animation state to the target entity.
pub(crate) fn receive_emote_events(
    mut commands: Commands,
    mut receivers: Query<&mut MessageReceiver<EmoteEvent>>,
    children_query: Query<&Children>,
    mounted_visual_roots: Query<(), With<crate::networking_player::MountedVisualRoot>>,
) {
    for mut receiver in receivers.iter_mut() {
        for event in receiver.receive() {
            let entity = resolve_emote_visual_entity(
                event.player_entity,
                &children_query,
                &mounted_visual_roots,
            );
            commands
                .entity(entity)
                .insert(crate::animation::EmoteAnimState::new(event.emote));
        }
    }
}

pub(crate) fn receive_rest_state_update(
    mut receivers: Query<&mut MessageReceiver<RestStateUpdate>>,
    mut snapshot: ResMut<game_engine::status::CharacterStatsSnapshot>,
) {
    for mut receiver in receivers.iter_mut() {
        for update in receiver.receive() {
            apply_rest_state_update(&mut snapshot, update);
        }
    }
}

fn apply_outgoing_chat_message(msg: &ChatMessage, whisper_state: &mut WhisperState) {
    if let shared::protocol::ChatType::Whisper(target) = &msg.channel
        && !target.trim().is_empty()
    {
        whisper_state.send_whisper(target);
    }
}

fn apply_incoming_chat_message(
    msg: &ChatMessage,
    local_name: Option<&str>,
    ignore_list: &IgnoreListStatusSnapshot,
    chat_log: &mut ChatLog,
    chat_state: &mut ChatState,
    whisper_state: &mut WhisperState,
) {
    if should_hide_message(msg, local_name, ignore_list) {
        return;
    }
    chat_log
        .messages
        .push((msg.sender.clone(), msg.content.clone(), msg.channel.clone()));
    if chat_log.messages.len() > MAX_CHAT_LOG {
        chat_log.messages.remove(0);
    }

    let timestamp = current_chat_timestamp();
    let (channel_type, channel_name) =
        map_runtime_chat_channel(&msg.channel, &msg.sender, local_name);
    if channel_type == ChatChannelType::Whisper {
        update_whisper_state(whisper_state, msg, local_name);
    }
    chat_state.add_message(RuntimeChatMessage {
        channel_type,
        channel_name,
        sender: msg.sender.clone(),
        text: msg.content.clone(),
        timestamp,
    });
}

fn should_hide_message(
    msg: &ChatMessage,
    local_name: Option<&str>,
    ignore_list: &IgnoreListStatusSnapshot,
) -> bool {
    if local_name.is_some_and(|name| msg.sender.eq_ignore_ascii_case(name)) {
        return false;
    }
    is_ignored_sender(ignore_list, &msg.sender)
}

fn map_runtime_chat_channel(
    channel: &shared::protocol::ChatType,
    sender: &str,
    local_name: Option<&str>,
) -> (ChatChannelType, String) {
    match channel {
        shared::protocol::ChatType::Say => (ChatChannelType::Say, String::new()),
        shared::protocol::ChatType::Yell => (ChatChannelType::Yell, String::new()),
        shared::protocol::ChatType::Party => (ChatChannelType::Party, String::new()),
        shared::protocol::ChatType::Guild => (ChatChannelType::Guild, String::new()),
        shared::protocol::ChatType::Emote => (ChatChannelType::Emote, String::new()),
        shared::protocol::ChatType::Whisper(target) => {
            let is_outgoing = local_name.is_some_and(|name| sender.eq_ignore_ascii_case(name));
            let channel_name = if is_outgoing {
                target.clone()
            } else {
                String::new()
            };
            (ChatChannelType::Whisper, channel_name)
        }
    }
}

fn update_whisper_state(
    whisper_state: &mut WhisperState,
    msg: &ChatMessage,
    local_name: Option<&str>,
) {
    let shared::protocol::ChatType::Whisper(target) = &msg.channel else {
        return;
    };
    let is_outgoing = local_name.is_some_and(|name| msg.sender.eq_ignore_ascii_case(name));
    if is_outgoing {
        whisper_state.send_whisper(target);
    } else {
        whisper_state.receive_whisper(&msg.sender);
    }
}

fn current_chat_timestamp() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64())
        .unwrap_or_default()
}

pub(crate) fn apply_rest_state_update(
    snapshot: &mut game_engine::status::CharacterStatsSnapshot,
    update: RestStateUpdate,
) {
    let Some(rest) = update.snapshot else {
        snapshot.in_rest_area = false;
        snapshot.rest_area_kind = None;
        snapshot.rested_xp = 0;
        snapshot.rested_xp_max = 0;
        return;
    };
    snapshot.in_rest_area = rest.in_rest_area;
    snapshot.rest_area_kind = rest.rest_area_kind.map(|kind| match kind {
        RestAreaKindSnapshot::City => RestAreaKindEntry::City,
        RestAreaKindSnapshot::Inn => RestAreaKindEntry::Inn,
    });
    snapshot.rested_xp = rest.rested_xp;
    snapshot.rested_xp_max = rest.rested_xp_max;
}

fn resolve_emote_visual_entity(
    player_entity_bits: u64,
    children_query: &Query<&Children>,
    mounted_visual_roots: &Query<(), With<crate::networking_player::MountedVisualRoot>>,
) -> Entity {
    let player_entity = Entity::from_bits(player_entity_bits);
    let Ok(children) = children_query.get(player_entity) else {
        return player_entity;
    };
    children
        .iter()
        .find(|child| mounted_visual_roots.get(*child).is_ok())
        .unwrap_or(player_entity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;

    fn whisper_message(sender: &str, target: &str, content: &str) -> ChatMessage {
        ChatMessage {
            sender: sender.into(),
            content: content.into(),
            channel: shared::protocol::ChatType::Whisper(target.into()),
        }
    }

    fn default_chat_state() -> ChatState {
        ChatState {
            max_messages: MAX_CHAT_LOG,
            ..Default::default()
        }
    }

    #[test]
    fn map_runtime_chat_channel_maps_emotes() {
        let (channel, name) =
            map_runtime_chat_channel(&shared::protocol::ChatType::Emote, "Alice", Some("Theron"));
        assert_eq!(channel, ChatChannelType::Emote);
        assert!(name.is_empty());
    }

    #[test]
    fn resolve_emote_visual_entity_prefers_mounted_visual_child() {
        let mut app = App::new();
        let parent = app.world_mut().spawn_empty().id();
        let mounted_child = app
            .world_mut()
            .spawn(crate::networking_player::MountedVisualRoot)
            .id();
        let other_child = app.world_mut().spawn_empty().id();
        app.world_mut()
            .entity_mut(parent)
            .add_children(&[other_child, mounted_child]);

        let entity = app
            .world_mut()
            .run_system_once(
                move |children_query: Query<&Children>,
                      mounted_visual_roots: Query<
                    (),
                    With<crate::networking_player::MountedVisualRoot>,
                >| {
                    resolve_emote_visual_entity(
                        parent.to_bits(),
                        &children_query,
                        &mounted_visual_roots,
                    )
                },
            )
            .expect("resolve emote entity");
        assert_eq!(entity, mounted_child);
    }

    #[test]
    fn outgoing_whisper_updates_recent_targets() {
        let mut whisper_state = WhisperState {
            max_recent: 10,
            ..Default::default()
        };

        apply_outgoing_chat_message(
            &whisper_message("Theron", "Alice", "hey"),
            &mut whisper_state,
        );

        assert_eq!(whisper_state.reply_target, None);
        assert_eq!(whisper_state.recent_targets, vec!["Alice"]);
    }

    #[test]
    fn incoming_whisper_sets_reply_target_and_runtime_message() {
        let mut chat_log = ChatLog::default();
        let mut chat_state = default_chat_state();
        let mut whisper_state = WhisperState {
            max_recent: 10,
            ..Default::default()
        };

        apply_incoming_chat_message(
            &whisper_message("Alice", "Theron", "psst"),
            Some("Theron"),
            &IgnoreListStatusSnapshot::default(),
            &mut chat_log,
            &mut chat_state,
            &mut whisper_state,
        );

        assert_eq!(chat_log.messages.len(), 1);
        assert_eq!(whisper_state.reply_target.as_deref(), Some("Alice"));
        assert_eq!(whisper_state.recent_targets, vec!["Alice"]);
        assert_eq!(chat_state.messages.len(), 1);
        assert_eq!(
            chat_state.messages[0].channel_type,
            ChatChannelType::Whisper
        );
        assert_eq!(chat_state.messages[0].channel_name, "");
    }

    #[test]
    fn outgoing_whisper_message_tracks_recipient_without_reply_target() {
        let mut chat_log = ChatLog::default();
        let mut chat_state = default_chat_state();
        let mut whisper_state = WhisperState {
            max_recent: 10,
            ..Default::default()
        };

        apply_incoming_chat_message(
            &whisper_message("Theron", "Alice", "hello"),
            Some("Theron"),
            &IgnoreListStatusSnapshot::default(),
            &mut chat_log,
            &mut chat_state,
            &mut whisper_state,
        );

        assert_eq!(whisper_state.reply_target, None);
        assert_eq!(whisper_state.recent_targets, vec!["Alice"]);
        assert_eq!(chat_state.messages[0].channel_name, "Alice");
    }

    #[test]
    fn ignored_sender_message_is_not_added_to_chat_log() {
        let mut chat_log = ChatLog::default();
        let mut chat_state = default_chat_state();
        let mut whisper_state = WhisperState {
            max_recent: 10,
            ..Default::default()
        };
        let ignore_list = IgnoreListStatusSnapshot {
            names: vec!["Alice".into()],
            ..Default::default()
        };

        apply_incoming_chat_message(
            &whisper_message("Alice", "Theron", "psst"),
            Some("Theron"),
            &ignore_list,
            &mut chat_log,
            &mut chat_state,
            &mut whisper_state,
        );

        assert!(chat_log.messages.is_empty());
        assert!(chat_state.messages.is_empty());
        assert_eq!(whisper_state.reply_target, None);
    }
}

/// Receive LoadTerrain messages from the server and initialize/stream the AdtManager.
pub(crate) fn receive_load_terrain(
    mut receivers: Query<&mut MessageReceiver<LoadTerrain>>,
    mut adt_manager: ResMut<AdtManager>,
    reconnect: Option<ResMut<crate::networking::ReconnectState>>,
) {
    let mut reconnect = reconnect;
    for mut receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            if let Some(ref mut reconnect) = reconnect
                && reconnect.is_active()
            {
                reconnect.terrain_refresh_seen = true;
            }
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

pub(crate) fn receive_collection_state_update(
    mut receivers: Query<&mut MessageReceiver<CollectionStateUpdate>>,
    mut snapshot: ResMut<CollectionStatusSnapshot>,
) {
    for mut receiver in receivers.iter_mut() {
        for update in receiver.receive() {
            map_collection_state_update(&mut snapshot, update);
        }
    }
}

pub(crate) fn receive_achievement_state_update(
    mut receivers: Query<&mut MessageReceiver<AchievementStateUpdate>>,
    mut status: ResMut<AchievementsStatusSnapshot>,
    mut completion: ResMut<game_engine::achievements::AchievementCompletionState>,
    mut toast: ResMut<AchievementToastState>,
) {
    for mut receiver in receivers.iter_mut() {
        for update in receiver.receive() {
            map_achievement_state_update(&mut status, &mut completion, &mut toast, update);
        }
    }
}

pub(crate) fn receive_profession_snapshot(
    mut receivers: Query<&mut MessageReceiver<ProfessionSnapshot>>,
    mut snapshot: ResMut<ProfessionStatusSnapshot>,
) {
    for mut receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            snapshot.skills = msg.skills.into_iter().map(map_profession_skill).collect();
            snapshot.recipes = msg.recipes.into_iter().map(map_profession_recipe).collect();
        }
    }
}

pub(crate) fn receive_world_map_state_update(
    mut receivers: Query<&mut MessageReceiver<WorldMapStateUpdate>>,
    mut world_map: ResMut<game_engine::world_map_data::WorldMapState>,
) {
    for mut receiver in receivers.iter_mut() {
        for update in receiver.receive() {
            map_world_map_state_update(&mut world_map, update);
        }
    }
}

pub(crate) fn receive_death_state_update(
    mut receivers: Query<&mut MessageReceiver<DeathStateUpdate>>,
    mut status: ResMut<DeathStatusSnapshot>,
    mut map_status: ResMut<game_engine::status::MapStatusSnapshot>,
) {
    for mut receiver in receivers.iter_mut() {
        for update in receiver.receive() {
            map_death_state_update(&mut status, &mut map_status, update);
        }
    }
}

pub(crate) fn apply_profession_state_update(
    snapshot: &mut ProfessionStatusSnapshot,
    update: ProfessionStateUpdate,
) {
    if let Some(profession_snapshot) = update.snapshot {
        snapshot.skills = profession_snapshot
            .skills
            .into_iter()
            .map(map_profession_skill)
            .collect();
        snapshot.recipes = profession_snapshot
            .recipes
            .into_iter()
            .map(map_profession_recipe)
            .collect();
    }
    snapshot.last_server_message = update.message;
    snapshot.last_skill_up = update.skill_up.map(|skill| ProfessionSkillUpEntry {
        profession: skill.profession,
        current: skill.current,
        max: skill.max,
    });
    snapshot.last_error = update.error;
}

fn map_profession_skill(skill: shared::protocol::ProfessionSkillSnapshot) -> ProfessionSkillEntry {
    ProfessionSkillEntry {
        profession: skill.profession,
        current: skill.current,
        max: skill.max,
    }
}

fn map_profession_recipe(
    recipe: shared::protocol::ProfessionRecipeSnapshot,
) -> ProfessionRecipeEntry {
    ProfessionRecipeEntry {
        spell_id: recipe.spell_id,
        profession: recipe.profession,
        name: recipe.name,
        craftable: recipe.craftable,
        cooldown: recipe.cooldown,
    }
}

pub(crate) fn receive_reputation_snapshot(
    mut receivers: Query<&mut MessageReceiver<ReputationStateUpdate>>,
    mut snapshot: ResMut<ReputationsStatusSnapshot>,
) {
    for mut receiver in receivers.iter_mut() {
        for update in receiver.receive() {
            apply_reputation_state_update(&mut snapshot, update);
        }
    }
}

pub(crate) fn apply_reputation_state_update(
    snapshot: &mut ReputationsStatusSnapshot,
    update: ReputationStateUpdate,
) {
    if let Some(rep_snapshot) = update.snapshot {
        snapshot.entries = rep_snapshot
            .entries
            .into_iter()
            .map(|e| ReputationEntry {
                faction_id: e.faction_id,
                faction_name: e.faction_name,
                standing: e.standing,
                value: e.value,
            })
            .collect();
    }
    snapshot.last_server_message = update.message;
    snapshot.last_error = update.error;
}

pub(crate) fn receive_guild_vault_snapshot(
    mut receivers: Query<&mut MessageReceiver<GuildVaultSnapshot>>,
    mut snapshot: ResMut<GuildVaultStatusSnapshot>,
) {
    for mut receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            snapshot.entries = msg.entries.into_iter().map(map_storage_item).collect();
        }
    }
}

pub(crate) fn receive_warbank_snapshot(
    mut receivers: Query<&mut MessageReceiver<WarbankSnapshot>>,
    mut snapshot: ResMut<WarbankStatusSnapshot>,
) {
    for mut receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            snapshot.entries = msg.entries.into_iter().map(map_storage_item).collect();
        }
    }
}

pub(crate) fn apply_talent_state_update(
    snapshot: &mut TalentStatusSnapshot,
    update: TalentStateUpdate,
) {
    if let Some(talent_snapshot) = update.snapshot {
        snapshot.spec_tabs = talent_snapshot
            .spec_tabs
            .into_iter()
            .map(|tab| TalentSpecTabEntry {
                name: tab.name,
                active: tab.active,
            })
            .collect();
        snapshot.talents = talent_snapshot
            .talents
            .into_iter()
            .map(|talent| TalentNodeEntry {
                talent_id: talent.talent_id,
                name: talent.name,
                points_spent: talent.points_spent,
                max_points: talent.max_points,
                active: talent.active,
            })
            .collect();
        snapshot.points_remaining = talent_snapshot.points_remaining;
    }
    snapshot.last_server_message = update.message;
    snapshot.last_error = update.error;
}

pub(crate) fn apply_inspect_state_update(
    snapshot: &mut InspectStatusSnapshot,
    update: InspectStateUpdate,
) {
    map_inspect_state_update(snapshot, update);
}

pub(crate) fn apply_duel_state_update(snapshot: &mut DuelStatusSnapshot, update: DuelStateUpdate) {
    map_duel_state_update(snapshot, update);
}

fn map_storage_item(e: StorageItemSnapshot) -> StorageItemEntry {
    StorageItemEntry {
        slot: e.slot,
        item_guid: e.item_guid,
        item_id: e.item_id,
        name: e.name,
        stack_count: e.stack_count,
    }
}

pub(crate) fn receive_inventory_search_snapshot(
    mut receivers: Query<&mut MessageReceiver<InventorySearchResultSnapshot>>,
    mut snapshot: ResMut<InventorySearchSnapshot>,
) {
    for mut receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            snapshot.entries = msg
                .entries
                .into_iter()
                .map(|e| InventoryItemEntry {
                    storage: e.storage,
                    slot: e.slot,
                    item_guid: e.item_guid,
                    item_id: e.item_id,
                    name: e.name,
                    stack_count: e.stack_count,
                })
                .collect();
        }
    }
}

/// When CurrentTarget changes, send a SetTarget message to the server.
pub(crate) fn send_target_to_server(
    current: Res<CurrentTarget>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    mut senders: Query<&mut MessageSender<SetTarget>>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) {
        return;
    }
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
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    mut senders: Query<&mut MessageSender<PlayerInput>>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) {
        return;
    }
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
