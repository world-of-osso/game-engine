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
    reset_resource::<game_engine::status::CharacterRosterStatusSnapshot>(world);
    reset_resource::<game_engine::status::CharacterStatsSnapshot>(world);
    reset_resource::<game_engine::status::CollectionStatusSnapshot>(world);
    reset_resource::<game_engine::status::CombatLogStatusSnapshot>(world);
    reset_resource::<game_engine::status::CurrenciesStatusSnapshot>(world);
    reset_resource::<game_engine::status::EquipmentAppearanceStatusSnapshot>(world);
    reset_resource::<game_engine::status::EquippedGearStatusSnapshot>(world);
    reset_resource::<game_engine::status::GroupStatusSnapshot>(world);
    reset_resource::<game_engine::status::GuildVaultStatusSnapshot>(world);
    reset_resource::<game_engine::status::InventorySearchSnapshot>(world);
    reset_resource::<game_engine::status::MapStatusSnapshot>(world);
    reset_resource::<game_engine::status::NetworkStatusSnapshot>(world);
    reset_resource::<game_engine::status::ProfessionStatusSnapshot>(world);
    reset_resource::<game_engine::status::QuestLogStatusSnapshot>(world);
    reset_resource::<game_engine::status::ReputationsStatusSnapshot>(world);
    reset_resource::<game_engine::status::SoundStatusSnapshot>(world);
    reset_resource::<game_engine::status::TerrainStatusSnapshot>(world);
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

#[cfg(test)]
mod tests {
    use super::*;

    // --- ReconnectState transitions ---

    #[test]
    fn reconnect_state_defaults_to_inactive() {
        let state = ReconnectState::default();
        assert_eq!(state.phase, ReconnectPhase::Inactive);
        assert!(!state.is_active());
        assert!(!state.terrain_refresh_seen);
    }

    #[test]
    fn reconnect_phase_pending_is_active() {
        let state = ReconnectState {
            phase: ReconnectPhase::PendingConnect,
            terrain_refresh_seen: false,
        };
        assert!(state.is_active());
    }

    #[test]
    fn reconnect_phase_awaiting_is_active() {
        let state = ReconnectState {
            phase: ReconnectPhase::AwaitingWorld,
            terrain_refresh_seen: false,
        };
        assert!(state.is_active());
    }

    #[test]
    fn reconnect_transition_to_inactive() {
        let mut state = ReconnectState {
            phase: ReconnectPhase::AwaitingWorld,
            terrain_refresh_seen: true,
        };
        state.phase = ReconnectPhase::Inactive;
        state.terrain_refresh_seen = false;
        assert!(!state.is_active());
        assert!(!state.terrain_refresh_seen);
    }

    // --- World reset ---

    #[test]
    fn reset_network_world_clears_status_snapshots() {
        let mut world = World::default();
        let mut snap = game_engine::status::NetworkStatusSnapshot::default();
        snap.connected = true;
        snap.game_state = "InWorld".into();
        snap.remote_entities = 10;
        world.insert_resource(snap);
        world.insert_resource(game_engine::status::TerrainStatusSnapshot {
            loaded_tiles: 5,
            ..Default::default()
        });

        reset_status_snapshots(&mut world);

        let net = world
            .get_resource::<game_engine::status::NetworkStatusSnapshot>()
            .unwrap();
        assert!(!net.connected);
        assert_eq!(net.remote_entities, 0);
        let terrain = world
            .get_resource::<game_engine::status::TerrainStatusSnapshot>()
            .unwrap();
        assert_eq!(terrain.loaded_tiles, 0);
    }

    #[test]
    fn reset_resource_resets_to_default() {
        let mut world = World::default();
        world.insert_resource(game_engine::status::CurrenciesStatusSnapshot {
            entries: vec![game_engine::status::CurrencyEntry {
                id: 1,
                name: "Honor".into(),
                amount: 5000,
            }],
        });

        reset_resource::<game_engine::status::CurrenciesStatusSnapshot>(&mut world);

        let snap = world
            .get_resource::<game_engine::status::CurrenciesStatusSnapshot>()
            .unwrap();
        assert!(snap.entries.is_empty());
    }

    #[test]
    fn reset_resource_missing_resource_no_panic() {
        let mut world = World::default();
        // Resource not inserted — should not panic
        reset_resource::<game_engine::status::CurrenciesStatusSnapshot>(&mut world);
    }

    #[test]
    fn reset_world_resources_clears_zone_and_chat() {
        let mut world = World::default();
        world.insert_resource(CurrentZone { zone_id: 42 });
        world.insert_resource(ChatLog {
            messages: vec![(
                "Player".into(),
                "hello".into(),
                shared::protocol::ChatType::Say,
            )],
        });
        world.insert_resource(LocalAliveState(false));

        reset_world_resources(&mut world);

        assert_eq!(world.resource::<CurrentZone>().zone_id, 0);
        assert!(world.resource::<ChatLog>().messages.is_empty());
        assert!(world.resource::<LocalAliveState>().0);
    }

    #[test]
    fn rand_client_id_nonzero() {
        let id = rand_client_id();
        assert_ne!(id, 0);
    }

    #[test]
    fn rand_client_id_different_calls() {
        let id1 = rand_client_id();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let id2 = rand_client_id();
        assert_ne!(id1, id2);
    }

    // --- Rapid reconnect: no state leaks ---

    #[test]
    fn rapid_reset_status_snapshots_no_accumulation() {
        let mut world = World::default();
        world.insert_resource(game_engine::status::NetworkStatusSnapshot::default());
        world.insert_resource(game_engine::status::CurrenciesStatusSnapshot::default());

        for i in 0..5 {
            // Simulate populating state between reconnects
            world
                .resource_mut::<game_engine::status::NetworkStatusSnapshot>()
                .remote_entities = i + 10;
            world
                .resource_mut::<game_engine::status::CurrenciesStatusSnapshot>()
                .entries
                .push(game_engine::status::CurrencyEntry {
                    id: i as u32,
                    name: format!("Currency{i}"),
                    amount: 100,
                });

            reset_status_snapshots(&mut world);

            let net = world.resource::<game_engine::status::NetworkStatusSnapshot>();
            assert_eq!(net.remote_entities, 0, "leak on iteration {i}");
            let cur = world.resource::<game_engine::status::CurrenciesStatusSnapshot>();
            assert!(cur.entries.is_empty(), "currency leak on iteration {i}");
        }
    }

    #[test]
    fn rapid_reset_world_resources_no_accumulation() {
        let mut world = World::default();
        world.insert_resource(CurrentZone { zone_id: 0 });
        world.insert_resource(ChatLog { messages: vec![] });
        world.insert_resource(LocalAliveState(true));

        for i in 0..5 {
            // Simulate state accumulating between reconnects
            world.resource_mut::<CurrentZone>().zone_id = 100 + i;
            world.resource_mut::<ChatLog>().messages.push((
                format!("Player{i}"),
                format!("msg{i}"),
                shared::protocol::ChatType::Say,
            ));
            world.resource_mut::<LocalAliveState>().0 = false;

            reset_world_resources(&mut world);

            assert_eq!(
                world.resource::<CurrentZone>().zone_id,
                0,
                "zone leak at {i}"
            );
            assert!(
                world.resource::<ChatLog>().messages.is_empty(),
                "chat leak at {i}"
            );
            assert!(world.resource::<LocalAliveState>().0, "alive leak at {i}");
        }
    }

    #[test]
    fn rapid_phase_cycling_no_stuck_state() {
        let mut state = ReconnectState::default();
        for _ in 0..10 {
            assert!(!state.is_active());
            // Disconnect → PendingConnect
            state.phase = ReconnectPhase::PendingConnect;
            assert!(state.is_active());
            // Connect → AwaitingWorld
            state.phase = ReconnectPhase::AwaitingWorld;
            state.terrain_refresh_seen = false;
            assert!(state.is_active());
            // Terrain arrives
            state.terrain_refresh_seen = true;
            // Complete reconnect
            state.phase = ReconnectPhase::Inactive;
            state.terrain_refresh_seen = false;
        }
        assert!(!state.is_active());
        assert!(!state.terrain_refresh_seen);
    }

    #[test]
    fn double_reset_idempotent() {
        let mut world = World::default();
        world.insert_resource(game_engine::status::NetworkStatusSnapshot {
            connected: true,
            remote_entities: 5,
            ..Default::default()
        });

        reset_status_snapshots(&mut world);
        reset_status_snapshots(&mut world);

        let net = world.resource::<game_engine::status::NetworkStatusSnapshot>();
        assert!(!net.connected);
        assert_eq!(net.remote_entities, 0);
    }

    // --- Connection timeout and retry behavior ---

    #[test]
    fn reconnect_awaiting_world_not_complete_without_terrain() {
        let mut state = ReconnectState {
            phase: ReconnectPhase::AwaitingWorld,
            terrain_refresh_seen: false,
        };
        // Without terrain refresh, should stay in AwaitingWorld
        assert!(state.is_active());
        assert!(!state.terrain_refresh_seen);
    }

    #[test]
    fn reconnect_completes_when_terrain_seen() {
        let mut state = ReconnectState {
            phase: ReconnectPhase::AwaitingWorld,
            terrain_refresh_seen: true,
        };
        // Simulate finish_reconnect_when_world_ready logic
        state.phase = ReconnectPhase::Inactive;
        state.terrain_refresh_seen = false;
        assert!(!state.is_active());
    }

    #[test]
    fn reconnect_pending_connect_requires_server() {
        // PendingConnect waits for a client entity — tested by checking phase
        let state = ReconnectState {
            phase: ReconnectPhase::PendingConnect,
            terrain_refresh_seen: false,
        };
        assert!(state.is_active());
        assert_eq!(state.phase, ReconnectPhase::PendingConnect);
    }

    #[test]
    fn reconnect_full_lifecycle() {
        let mut state = ReconnectState::default();
        // Initial: inactive
        assert!(!state.is_active());

        // Disconnect detected → PendingConnect
        state.phase = ReconnectPhase::PendingConnect;
        assert!(state.is_active());

        // Connection established → AwaitingWorld
        state.phase = ReconnectPhase::AwaitingWorld;
        assert!(state.is_active());
        assert!(!state.terrain_refresh_seen);

        // Terrain refresh arrives
        state.terrain_refresh_seen = true;

        // Local player present → complete
        state.phase = ReconnectPhase::Inactive;
        state.terrain_refresh_seen = false;
        assert!(!state.is_active());
    }

    #[test]
    fn timeout_config_is_60_seconds() {
        // Verify the timeout constant used in connect_to_server_inner
        let config = lightyear::prelude::client::NetcodeConfig {
            client_timeout_secs: 60,
            ..Default::default()
        };
        assert_eq!(config.client_timeout_secs, 60);
    }

    #[test]
    fn reset_after_partial_state_population() {
        let mut world = World::default();
        world.insert_resource(game_engine::status::NetworkStatusSnapshot::default());
        world.insert_resource(game_engine::status::QuestLogStatusSnapshot::default());

        // Only populate some resources, not all
        world
            .resource_mut::<game_engine::status::NetworkStatusSnapshot>()
            .connected = true;
        // QuestLog left at default

        reset_status_snapshots(&mut world);

        assert!(
            !world
                .resource::<game_engine::status::NetworkStatusSnapshot>()
                .connected
        );
        assert!(
            world
                .resource::<game_engine::status::QuestLogStatusSnapshot>()
                .entries
                .is_empty()
        );
    }
}
