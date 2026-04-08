use std::path::PathBuf;

use bevy::ecs::system::SystemParam;
use bevy::image::Image;
use bevy::mesh::Mesh;
use bevy::prelude::*;
use game_engine::ipc::plugin::{EquipmentControlCommand, EquipmentControlQueue};
use game_engine::status::{
    BarberShopStatusSnapshot, CharacterRosterStatusSnapshot, CharacterStatsSnapshot,
    CollectionStatusSnapshot, CombatLogStatusSnapshot, CurrenciesStatusSnapshot,
    DuelStatusSnapshot, EquipmentAppearanceStatusSnapshot, EquippedGearEntry,
    EquippedGearStatusSnapshot, FriendsStatusSnapshot, GroupStatusSnapshot,
    GuildVaultStatusSnapshot, IgnoreListStatusSnapshot, InspectStatusSnapshot, LfgStatusSnapshot,
    MapStatusSnapshot, NetworkStatusSnapshot, PresenceStateEntry, ProfessionStatusSnapshot,
    PvpStatusSnapshot, QuestLogStatusSnapshot, ReputationsStatusSnapshot, SoundStatusSnapshot,
    TalentStatusSnapshot, TerrainStatusSnapshot, WarbankStatusSnapshot,
};
use lightyear::prelude::client::Connected;
use shared::components::{
    CombatStatus as NetCombatStatus, EquipmentAppearance as NetEquipmentAppearance,
    Health as NetHealth, Mana as NetMana, MovementSpeed as NetMovementSpeed, Player as NetPlayer,
    PresenceStatus as NetPresenceStatus,
};

use crate::camera::Player;
use crate::equipment;
use crate::m2_spawn;
use crate::networking;
use crate::process_memory_status::current_process_memory_kb;
use crate::sound;
use crate::status_asset_stats;
use crate::terrain::AdtManager;
use crate::terrain_heightmap::TerrainHeightmap;
use crate::terrain_material::TerrainMaterial;
use crate::water_material::WaterMaterial;

type LocalPlayerComponents = (
    Option<&'static NetPlayer>,
    Option<&'static NetHealth>,
    Option<&'static NetMana>,
    Option<&'static NetMovementSpeed>,
    Option<&'static NetCombatStatus>,
    Option<&'static NetPresenceStatus>,
);

#[derive(SystemParam)]
pub(crate) struct NetworkStatusParams<'w, 's> {
    server_addr: Option<Res<'w, networking::ServerAddr>>,
    game_state: Option<Res<'w, State<crate::game_state::GameState>>>,
    local_client_id: Option<Res<'w, networking::LocalClientId>>,
    current_zone: Res<'w, networking::CurrentZone>,
    chat_log: Res<'w, networking::ChatLog>,
    connected_query: Query<'w, 's, (), With<Connected>>,
    remote_query: Query<'w, 's, (), With<networking::RemoteEntity>>,
    local_player_query: Query<'w, 's, (), With<networking::LocalPlayer>>,
}

pub fn sync_network_status_snapshot(
    mut snapshot: ResMut<NetworkStatusSnapshot>,
    params: NetworkStatusParams,
) {
    let NetworkStatusParams {
        server_addr,
        game_state,
        local_client_id,
        current_zone,
        chat_log,
        connected_query,
        remote_query,
        local_player_query,
    } = params;
    snapshot.server_addr = server_addr.map(|addr| addr.0.to_string());
    snapshot.game_state = game_state
        .map(|state| format!("{:?}", state.get()))
        .unwrap_or_else(|| "Unavailable".into());
    snapshot.connected = !connected_query.is_empty();
    snapshot.connected_links = connected_query.iter().count();
    snapshot.local_client_id = local_client_id.map(|id| id.0);
    snapshot.zone_id = current_zone.zone_id;
    snapshot.remote_entities = remote_query.iter().count();
    snapshot.local_players = local_player_query.iter().count();
    snapshot.chat_messages = chat_log.messages.len();
}

pub fn sync_terrain_status_snapshot(
    mut snapshot: ResMut<TerrainStatusSnapshot>,
    adt_manager: Res<AdtManager>,
    heightmap: Res<TerrainHeightmap>,
    images: Res<Assets<Image>>,
    meshes: Res<Assets<Mesh>>,
    standard_materials: Res<Assets<StandardMaterial>>,
    terrain_materials: Res<Assets<TerrainMaterial>>,
    water_materials: Res<Assets<WaterMaterial>>,
    m2_effect_materials: Res<Assets<crate::m2_effect_material::M2EffectMaterial>>,
) {
    let process_memory = current_process_memory_kb();
    let model_cache = crate::asset::m2::model_cache_stats();
    let composited_cache = m2_spawn::composited_texture_cache_stats();
    let asset_stats = status_asset_stats::collect_asset_store_stats(
        &images,
        &meshes,
        &standard_materials,
        &terrain_materials,
        &water_materials,
        &m2_effect_materials,
    );
    snapshot.map_name = adt_manager.map_name.clone();
    snapshot.initial_tile = adt_manager.initial_tile;
    snapshot.load_radius = adt_manager.load_radius;
    snapshot.loaded_tiles = adt_manager.loaded.len();
    snapshot.pending_tiles = adt_manager.pending.len();
    snapshot.failed_tiles = adt_manager.failed.len();
    snapshot.server_requested_tiles = adt_manager.server_requested.len();
    snapshot.heightmap_tiles = heightmap.tile_keys().count();
    snapshot.process_rss_kb = process_memory.rss_kb;
    snapshot.process_anon_kb = process_memory.anon_kb;
    snapshot.process_data_kb = process_memory.data_kb;
    snapshot.m2_model_cache_entries = model_cache.entries;
    snapshot.m2_model_cache_est_cpu_bytes = model_cache.est_cpu_bytes;
    snapshot.composited_texture_cache_entries = composited_cache.entries;
    snapshot.composited_texture_cache_est_cpu_bytes = composited_cache.est_cpu_bytes;
    snapshot.image_assets = asset_stats.image_assets;
    snapshot.image_asset_cpu_bytes = asset_stats.image_asset_cpu_bytes;
    snapshot.mesh_assets = asset_stats.mesh_assets;
    snapshot.mesh_asset_est_cpu_bytes = asset_stats.mesh_asset_est_cpu_bytes;
    snapshot.standard_material_assets = asset_stats.standard_material_assets;
    snapshot.terrain_material_assets = asset_stats.terrain_material_assets;
    snapshot.water_material_assets = asset_stats.water_material_assets;
    snapshot.m2_effect_material_assets = asset_stats.m2_effect_material_assets;
}

pub fn sync_sound_status_snapshot(
    mut snapshot: ResMut<SoundStatusSnapshot>,
    sound_settings: Option<Res<sound::SoundSettings>>,
    ambient_query: Query<(), With<sound::AmbientSound>>,
    sinks: Query<&AudioSink>,
) {
    if let Some(settings) = sound_settings {
        snapshot.enabled = true;
        snapshot.muted = settings.muted;
        snapshot.master_volume = settings.master_volume;
        snapshot.ambient_volume = settings.ambient_volume;
    } else {
        *snapshot = SoundStatusSnapshot::default();
    }
    snapshot.ambient_entities = ambient_query.iter().count();
    snapshot.active_sinks = sinks.iter().count();
}

/// Fill health/mana/speed from the local player entity into the snapshot.
fn fill_local_player_stats(
    snapshot: &mut CharacterStatsSnapshot,
    local_player_query: &Query<LocalPlayerComponents, With<networking::LocalPlayer>>,
) {
    if let Some((_, health, mana, speed, in_combat, presence)) = local_player_query.iter().next() {
        snapshot.health_current = health.map(|v| v.current);
        snapshot.health_max = health.map(|v| v.max);
        snapshot.mana_current = mana.map(|v| v.current);
        snapshot.mana_max = mana.map(|v| v.max);
        snapshot.movement_speed = speed.map(|v| v.0);
        snapshot.in_combat = in_combat.is_some_and(|flag| flag.0);
        snapshot.presence = presence.copied().map(map_presence_state);
    } else {
        snapshot.health_current = None;
        snapshot.health_max = None;
        snapshot.mana_current = None;
        snapshot.mana_max = None;
        snapshot.movement_speed = None;
        snapshot.presence = None;
        snapshot.in_combat = false;
    }
}

pub fn sync_character_stats_snapshot(
    mut snapshot: ResMut<CharacterStatsSnapshot>,
    character_list: Res<networking::CharacterList>,
    selected_character_id: Res<networking::SelectedCharacterId>,
    current_zone: Res<networking::CurrentZone>,
    local_player_query: Query<LocalPlayerComponents, With<networking::LocalPlayer>>,
) {
    let selected_character = selected_character_id.character_id.and_then(|character_id| {
        character_list
            .0
            .iter()
            .find(|entry| entry.character_id == character_id)
    });
    snapshot.character_id = selected_character.map(|entry| entry.character_id);
    snapshot.name = selected_character
        .map(|entry| entry.name.clone())
        .or_else(|| {
            local_player_query
                .iter()
                .find_map(|(player, _, _, _, _, _)| player.map(|player| player.name.clone()))
        });
    snapshot.level = selected_character.map(|entry| entry.level);
    snapshot.race = selected_character.map(|entry| entry.race);
    snapshot.class = selected_character.map(|entry| entry.class);
    snapshot.appearance = selected_character.map(|entry| entry.appearance);
    snapshot.zone_id = current_zone.zone_id;
    fill_local_player_stats(&mut snapshot, &local_player_query);
}

fn map_presence_state(state: NetPresenceStatus) -> PresenceStateEntry {
    match state {
        NetPresenceStatus::Online => PresenceStateEntry::Online,
        NetPresenceStatus::Afk => PresenceStateEntry::Afk,
        NetPresenceStatus::Dnd => PresenceStateEntry::Dnd,
        NetPresenceStatus::Offline => PresenceStateEntry::Offline,
    }
}

pub fn sync_character_roster_status_snapshot(
    mut snapshot: ResMut<CharacterRosterStatusSnapshot>,
    character_list: Res<networking::CharacterList>,
) {
    snapshot.entries = character_list.0.clone();
}

pub fn sync_equipped_gear_status_snapshot(
    mut snapshot: ResMut<EquippedGearStatusSnapshot>,
    local_player_query: Query<&equipment::Equipment, With<Player>>,
) {
    snapshot.entries.clear();
    if let Some(equipment) = local_player_query.iter().next() {
        let mut entries = Vec::with_capacity(equipment.slots.len());
        for (slot, path) in &equipment.slots {
            entries.push(EquippedGearEntry {
                slot: format!("{slot:?}"),
                path: path.display().to_string(),
            });
        }
        entries.sort_by(|a, b| a.slot.cmp(&b.slot));
        snapshot.entries = entries;
    }
}

pub fn sync_equipment_appearance_status_snapshot(
    mut snapshot: ResMut<EquipmentAppearanceStatusSnapshot>,
    local_player_query: Query<&NetEquipmentAppearance, With<networking::LocalPlayer>>,
) {
    snapshot.appearance = local_player_query
        .iter()
        .next()
        .cloned()
        .unwrap_or_default();
}

pub fn apply_equipment_ipc_commands(
    mut queue: ResMut<EquipmentControlQueue>,
    mut commands: Commands,
    mut local_player_query: Query<(Entity, Option<&mut equipment::Equipment>), With<Player>>,
) {
    if queue.pending.is_empty() {
        return;
    }
    let Some((entity, maybe_equipment)) = local_player_query.iter_mut().next() else {
        queue.pending.clear();
        return;
    };
    let mut pending = std::mem::take(&mut queue.pending);
    if let Some(mut equipment) = maybe_equipment {
        for command in pending.drain(..) {
            apply_equipment_command(&mut equipment, command);
        }
        return;
    }
    let mut equipment = equipment::Equipment::default();
    for command in pending.drain(..) {
        apply_equipment_command(&mut equipment, command);
    }
    commands.entity(entity).insert(equipment);
}

fn apply_equipment_command(equipment: &mut equipment::Equipment, command: EquipmentControlCommand) {
    match command {
        EquipmentControlCommand::Set { slot, model_path } => {
            let Some(slot) = parse_equipment_slot(&slot) else {
                warn!("Ignoring equipment set with invalid slot '{slot}'");
                return;
            };
            let path = PathBuf::from(model_path);
            if !path.exists() {
                warn!(
                    "Ignoring equipment set for missing model path {}",
                    path.display()
                );
                return;
            }
            equipment.slots.insert(slot, path);
        }
        EquipmentControlCommand::Clear { slot } => {
            let Some(slot) = parse_equipment_slot(&slot) else {
                warn!("Ignoring equipment clear with invalid slot '{slot}'");
                return;
            };
            equipment.slots.remove(&slot);
        }
    }
}

fn parse_equipment_slot(value: &str) -> Option<equipment::EquipmentSlot> {
    match value.to_ascii_lowercase().as_str() {
        "mainhand" | "main-hand" | "main" | "mh" => Some(equipment::EquipmentSlot::MainHand),
        "offhand" | "off-hand" | "off" | "oh" => Some(equipment::EquipmentSlot::OffHand),
        _ => None,
    }
}

pub fn sync_map_status_snapshot(
    mut snapshot: ResMut<MapStatusSnapshot>,
    current_zone: Res<networking::CurrentZone>,
    player_query: Query<&Transform, With<Player>>,
) {
    crate::status_map_sync::fill_map_status_snapshot(
        &mut snapshot,
        current_zone.zone_id,
        player_query.iter().next(),
    );
}

pub(crate) fn init_status_resources(app: &mut App) {
    app.insert_resource(NetworkStatusSnapshot::default())
        .insert_resource(TerrainStatusSnapshot::default())
        .insert_resource(SoundStatusSnapshot::default())
        .insert_resource(CharacterRosterStatusSnapshot::default())
        .insert_resource(CharacterStatsSnapshot::default())
        .insert_resource(BarberShopStatusSnapshot::default())
        .insert_resource(FriendsStatusSnapshot::default())
        .insert_resource(IgnoreListStatusSnapshot::default())
        .insert_resource(PvpStatusSnapshot::default())
        .insert_resource(LfgStatusSnapshot::default())
        .insert_resource(EquippedGearStatusSnapshot::default())
        .insert_resource(EquipmentAppearanceStatusSnapshot::default())
        .insert_resource(MapStatusSnapshot::default())
        .insert_resource(CollectionStatusSnapshot::default())
        .insert_resource(CombatLogStatusSnapshot::default())
        .insert_resource(CurrenciesStatusSnapshot::default())
        .insert_resource(DuelStatusSnapshot::default())
        .insert_resource(GroupStatusSnapshot::default())
        .insert_resource(GuildVaultStatusSnapshot::default())
        .insert_resource(InspectStatusSnapshot::default())
        .insert_resource(ProfessionStatusSnapshot::default())
        .insert_resource(QuestLogStatusSnapshot::default())
        .insert_resource(TalentStatusSnapshot::default())
        .insert_resource(ReputationsStatusSnapshot::default())
        .insert_resource(WarbankStatusSnapshot::default());
}

pub(crate) fn register_status_sync_systems(app: &mut App) {
    app.add_systems(Update, sync_character_roster_status_snapshot);
    app.add_systems(
        Update,
        (
            sync_network_status_snapshot,
            sync_terrain_status_snapshot,
            sync_sound_status_snapshot,
            sync_character_stats_snapshot,
            apply_equipment_ipc_commands,
            sync_equipped_gear_status_snapshot,
            sync_equipment_appearance_status_snapshot,
            sync_map_status_snapshot,
        )
            .run_if(in_state(crate::game_state::GameState::InWorld)),
    );
}
