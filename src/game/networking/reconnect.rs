use bevy::prelude::*;
use lightyear::prelude::client::*;
use lightyear::prelude::*;

use crate::camera::{CharacterFacing, MovementState, Player};
use crate::networking::{
    ChatInput, ChatLog, CurrentZone, LocalAliveState, LocalPlayer, ReconnectPhase, ReconnectState,
    ServerAddr,
};

pub(crate) fn drive_inworld_reconnect(
    reconnect: Option<ResMut<ReconnectState>>,
    server_addr: Option<Res<ServerAddr>>,
    client_q: Query<(), With<Client>>,
    mut commands: Commands,
) {
    let (Some(reconnect), Some(server_addr)) = (reconnect, server_addr) else {
        return;
    };
    if reconnect.phase != ReconnectPhase::PendingConnect || !client_q.is_empty() {
        return;
    }
    crate::networking::connect_to_server_inner(&mut commands, server_addr.0);
}

pub(crate) fn finish_reconnect_when_world_ready(
    mut reconnect: ResMut<ReconnectState>,
    local_player_q: Query<(), With<LocalPlayer>>,
) {
    if reconnect.phase != ReconnectPhase::AwaitingWorld {
        return;
    }
    if reconnect.terrain_refresh_seen && !local_player_q.is_empty() {
        info!("Reconnect complete, resynchronized local world state");
        reconnect.phase = ReconnectPhase::Inactive;
        reconnect.terrain_refresh_seen = false;
    }
}

pub(crate) fn reset_network_world(world: &mut World) {
    despawn_client_entities(world);
    despawn_replicated_entities(world);
    strip_local_player_components(world);
    reset_world_resources(world);
    reset_status_snapshots(world);
}

fn despawn_client_entities(world: &mut World) {
    let entities: Vec<_> = world
        .query_filtered::<Entity, With<Client>>()
        .iter(world)
        .collect();
    for entity in entities {
        if let Ok(entity_mut) = world.get_entity_mut(entity) {
            entity_mut.despawn();
        }
    }
}

fn despawn_replicated_entities(world: &mut World) {
    let entities: Vec<_> = world
        .query_filtered::<Entity, With<Replicated>>()
        .iter(world)
        .collect();
    for entity in entities {
        if let Ok(entity_mut) = world.get_entity_mut(entity) {
            entity_mut.despawn();
        }
    }
}

fn strip_local_player_components(world: &mut World) {
    let entities: Vec<_> = world
        .query_filtered::<Entity, (With<LocalPlayer>, Without<Replicated>)>()
        .iter(world)
        .collect();
    for entity in entities {
        if let Ok(mut entity_mut) = world.get_entity_mut(entity) {
            entity_mut.remove::<(
                LocalPlayer,
                Player,
                MovementState,
                CharacterFacing,
                crate::collision::CharacterPhysics,
            )>();
        }
    }
}

fn reset_world_resources(world: &mut World) {
    if let Some(mut t) = world.get_resource_mut::<game_engine::targeting::CurrentTarget>() {
        t.0 = None;
    }
    if let Some(mut zone) = world.get_resource_mut::<CurrentZone>() {
        zone.zone_id = 0;
    }
    if let Some(mut alive) = world.get_resource_mut::<LocalAliveState>() {
        alive.0 = true;
    }
    if let Some(mut log) = world.get_resource_mut::<ChatLog>() {
        log.messages.clear();
    }
    if let Some(mut chat_input) = world.get_resource_mut::<ChatInput>() {
        chat_input.0 = None;
    }
    if let Some(mut adt_manager) = world.get_resource_mut::<crate::terrain::AdtManager>() {
        adt_manager.server_requested.clear();
    }
}

fn reset_status_snapshots(world: &mut World) {
    reset_resource::<game_engine::status::NetworkStatusSnapshot>(world);
    reset_resource::<game_engine::status::CharacterStatsSnapshot>(world);
    reset_resource::<game_engine::status::CollectionStatusSnapshot>(world);
    reset_resource::<game_engine::status::CombatLogStatusSnapshot>(world);
    reset_resource::<game_engine::status::CurrenciesStatusSnapshot>(world);
    reset_resource::<game_engine::status::GroupStatusSnapshot>(world);
    reset_resource::<game_engine::status::GuildVaultStatusSnapshot>(world);
    reset_resource::<game_engine::status::InventorySearchSnapshot>(world);
    reset_resource::<game_engine::status::MapStatusSnapshot>(world);
    reset_resource::<game_engine::status::ProfessionStatusSnapshot>(world);
    reset_resource::<game_engine::status::QuestLogStatusSnapshot>(world);
    reset_resource::<game_engine::status::ReputationsStatusSnapshot>(world);
    reset_resource::<game_engine::status::WarbankStatusSnapshot>(world);
}

fn reset_resource<T: Resource + Default>(world: &mut World) {
    if let Some(mut resource) = world.get_resource_mut::<T>() {
        *resource = T::default();
    }
}

pub(crate) fn rand_client_id() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}
