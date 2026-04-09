use super::*;
use bevy::ecs::system::RunSystemOnce;
use std::f32::consts::{FRAC_PI_2, PI};

use crate::camera::MoveDirection;
use crate::networking_npc::{NpcVisibilityPolicy, npc_visibility_policy};
use crate::networking_player::{
    choose_local_player_entity, is_local_player_entity, net_player_customization_selection,
    resolve_player_model_path, sync_local_alive_state,
};
use crate::networking_reconnect::{finish_reconnect_when_world_ready, reset_network_world};
use game_engine::chat_data::WhisperState;
use shared::components::{CharacterAppearance, Health as NetHealth, Player as NetPlayer};

fn make_state(direction: MoveDirection) -> MovementState {
    MovementState {
        direction,
        ..Default::default()
    }
}

fn make_facing(yaw: f32) -> CharacterFacing {
    CharacterFacing { yaw }
}

#[test]
fn idle_produces_zero_direction() {
    let dir = movement_to_direction(&make_state(MoveDirection::None), &make_facing(0.0));
    assert_eq!(dir, [0.0, 0.0, 0.0]);
}

#[test]
fn forward_at_zero_yaw() {
    let dir = movement_to_direction(&make_state(MoveDirection::Forward), &make_facing(0.0));
    // yaw=0: forward = [sin(0), 0, cos(0)] = [0, 0, 1]
    assert!(dir[0].abs() < 1e-6);
    assert_eq!(dir[1], 0.0);
    assert!((dir[2] - 1.0).abs() < 1e-6);
}

#[test]
fn forward_at_90_degrees() {
    let dir = movement_to_direction(&make_state(MoveDirection::Forward), &make_facing(FRAC_PI_2));
    // yaw=π/2: forward = [sin(π/2), 0, cos(π/2)] = [1, 0, 0]
    assert!((dir[0] - 1.0).abs() < 1e-6);
    assert!((dir[2]).abs() < 1e-6);
}

#[test]
fn backward_is_opposite_of_forward() {
    let facing = make_facing(0.5);
    let fwd = movement_to_direction(&make_state(MoveDirection::Forward), &facing);
    let bwd = movement_to_direction(&make_state(MoveDirection::Backward), &facing);
    assert!((fwd[0] + bwd[0]).abs() < 1e-6);
    assert!((fwd[2] + bwd[2]).abs() < 1e-6);
}

#[test]
fn left_is_perpendicular_to_forward() {
    let facing = make_facing(0.0);
    let fwd = movement_to_direction(&make_state(MoveDirection::Forward), &facing);
    let left = movement_to_direction(&make_state(MoveDirection::Left), &facing);
    // dot product should be zero (perpendicular)
    let dot = fwd[0] * left[0] + fwd[2] * left[2];
    assert!(dot.abs() < 1e-6);
}

#[test]
fn right_is_opposite_of_left() {
    let facing = make_facing(PI / 3.0);
    let left = movement_to_direction(&make_state(MoveDirection::Left), &facing);
    let right = movement_to_direction(&make_state(MoveDirection::Right), &facing);
    assert!((left[0] + right[0]).abs() < 1e-6);
    assert!((left[2] + right[2]).abs() < 1e-6);
}

#[test]
fn direction_is_unit_length() {
    for dir in [
        MoveDirection::Forward,
        MoveDirection::Backward,
        MoveDirection::Left,
        MoveDirection::Right,
    ] {
        let d = movement_to_direction(&make_state(dir), &make_facing(1.23));
        let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        assert!(
            (len - 1.0).abs() < 1e-6,
            "direction {dir:?} has length {len}"
        );
    }
}

#[test]
fn y_component_always_zero() {
    for yaw in [0.0, FRAC_PI_2, PI, -PI] {
        for dir in [
            MoveDirection::Forward,
            MoveDirection::Backward,
            MoveDirection::Left,
            MoveDirection::Right,
        ] {
            let d = movement_to_direction(&make_state(dir), &make_facing(yaw));
            assert_eq!(d[1], 0.0);
        }
    }
}

fn charselect_disconnect_app(token: Option<&str>) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.insert_state(crate::game_state::GameState::CharSelect);
    app.init_resource::<AuthUiFeedback>();
    app.init_resource::<ReconnectState>();
    app.insert_resource(AuthToken(token.map(|t| t.to_string())));
    app.insert_resource(LoginMode::Login);
    app.insert_resource(LoginUsername("stale-user".to_string()));
    app.insert_resource(LoginPassword("stale-pass".to_string()));
    app.add_observer(handle_client_disconnected);
    app
}

fn trigger_disconnect(app: &mut App) -> Entity {
    let client = app.world_mut().spawn(Client::default()).id();
    trigger_disconnect_entity(app, client);
    client
}

fn trigger_disconnect_entity(app: &mut App, client: Entity) {
    app.world_mut().entity_mut(client).insert(Disconnected {
        reason: Some("Link failed: test".to_string()),
    });
}

#[test]
fn disconnect_during_charselect_arms_reconnect_when_token_exists() {
    let mut app = charselect_disconnect_app(Some("saved-token"));
    let client = trigger_disconnect(&mut app);
    app.update();

    let state = app
        .world()
        .resource::<State<crate::game_state::GameState>>();
    assert_eq!(*state.get(), crate::game_state::GameState::CharSelect);
    let feedback = app.world().resource::<AuthUiFeedback>();
    assert_eq!(feedback.0.as_deref(), None);
    assert_eq!(
        app.world().resource::<ReconnectState>().phase,
        ReconnectPhase::PendingConnect
    );
    assert_eq!(app.world().resource::<LoginUsername>().0, "");
    assert_eq!(app.world().resource::<LoginPassword>().0, "");
    assert!(app.world().get_entity(client).is_err());
}

#[test]
fn disconnect_during_charselect_without_token_stays_offline() {
    let mut app = charselect_disconnect_app(None);
    let client = trigger_disconnect(&mut app);
    app.update();

    let state = app
        .world()
        .resource::<State<crate::game_state::GameState>>();
    assert_eq!(*state.get(), crate::game_state::GameState::CharSelect);
    let feedback = app.world().resource::<AuthUiFeedback>();
    assert_eq!(
        feedback.0.as_deref(),
        Some("Connection lost. Char select is now offline.")
    );
    assert_eq!(
        app.world().resource::<ReconnectState>().phase,
        ReconnectPhase::Inactive
    );
    assert!(app.world().get_entity(client).is_ok());
}

#[test]
fn disconnect_during_connecting_is_ignored() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.insert_state(crate::game_state::GameState::Connecting);
    app.init_resource::<AuthUiFeedback>();
    app.add_observer(handle_client_disconnected);
    trigger_disconnect(&mut app);
    app.update();
    app.update();

    let state = app
        .world()
        .resource::<State<crate::game_state::GameState>>();
    assert_eq!(*state.get(), crate::game_state::GameState::Connecting);
    let feedback = app.world().resource::<AuthUiFeedback>();
    assert_eq!(feedback.0.as_deref(), None);
}

fn disconnect_app_with_state(state: crate::game_state::GameState) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.insert_state(state);
    app.init_resource::<AuthUiFeedback>();
    app.init_resource::<ReconnectState>();
    app.insert_resource(AuthToken(Some("saved-token".to_string())));
    app.insert_resource(selected_with_name("Theron"));
    app.insert_resource(game_engine::targeting::CurrentTarget(Some(
        Entity::from_bits(77),
    )));
    app.init_resource::<CurrentZone>();
    app.init_resource::<LocalAliveState>();
    app.init_resource::<ChatLog>();
    app.init_resource::<ChatInput>();
    app.insert_resource(WhisperState {
        reply_target: Some("StaleWhisper".into()),
        recent_targets: vec!["StaleWhisper".into()],
        max_recent: 10,
    });
    let mut trade_state = game_engine::trade::TradeClientState::default();
    trade_state.phase = Some(shared::protocol::TradePhase::Open);
    trade_state.trade = game_engine::trade_data::TradeState {
        active: true,
        ..Default::default()
    };
    trade_state.last_message = Some("stale trade".into());
    app.insert_resource(trade_state);
    app.add_observer(handle_client_disconnected);
    app
}

fn inworld_disconnect_base_app() -> App {
    disconnect_app_with_state(crate::game_state::GameState::InWorld)
}

fn populate_inworld_disconnect_entities(app: &mut App) -> (Entity, Entity) {
    let client = app.world_mut().spawn(Client::default()).id();
    let receiver = app.world_mut().spawn_empty().id();
    let replicated = app
        .world_mut()
        .spawn((Replicated { receiver }, RemoteEntity, net_player("Theron")))
        .id();
    app.world_mut().resource_mut::<ChatLog>().messages.push((
        "system".to_string(),
        "stale".to_string(),
        ChatType::Say,
    ));
    (client, replicated)
}

fn assert_inworld_reconnect_state(app: &App, client: Entity, replicated: Entity) {
    let state = app
        .world()
        .resource::<State<crate::game_state::GameState>>();
    assert_eq!(*state.get(), crate::game_state::GameState::InWorld);
    assert_eq!(
        app.world().resource::<ReconnectState>().phase,
        ReconnectPhase::PendingConnect
    );
    assert!(
        app.world()
            .contains_resource::<crate::scenes::char_select::AutoEnterWorld>()
    );
    assert_eq!(
        app.world()
            .resource::<crate::scenes::char_select::PreselectedCharName>()
            .0,
        "Theron"
    );
    assert!(app.world().get_entity(client).is_err());
    assert!(app.world().get_entity(replicated).is_err());
    assert!(
        app.world()
            .resource::<game_engine::targeting::CurrentTarget>()
            .0
            .is_none()
    );
    assert!(app.world().resource::<ChatLog>().messages.is_empty());
    let whisper_state = app.world().resource::<WhisperState>();
    assert_eq!(whisper_state.reply_target, None);
    assert!(whisper_state.recent_targets.is_empty());
    let trade_state = app
        .world()
        .resource::<game_engine::trade::TradeClientState>();
    assert_eq!(trade_state.phase, None);
    assert!(!trade_state.trade.active);
    assert_eq!(trade_state.last_message, None);
}

#[test]
fn disconnect_during_inworld_arms_reconnect_and_preserves_scene_state() {
    let mut app = inworld_disconnect_base_app();
    let (client, replicated) = populate_inworld_disconnect_entities(&mut app);
    trigger_disconnect_entity(&mut app, client);

    app.update();
    app.update();

    assert_inworld_reconnect_state(&app, client, replicated);
}

#[test]
fn reset_network_world_preserves_selected_character_for_reconnect() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(selected_with_name("Theron"));

    let _ = app.world_mut().run_system_once(|mut commands: Commands| {
        commands.queue(reset_network_world);
    });
    app.update();

    let selected = app.world().resource::<SelectedCharacterId>();
    assert_eq!(selected.character_name.as_deref(), Some("Theron"));
}

#[test]
fn gameplay_input_is_disabled_during_reconnect() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(ReconnectState {
        phase: ReconnectPhase::PendingConnect,
        terrain_refresh_seen: false,
    });

    let allowed = app
        .world_mut()
        .run_system_once(|reconnect: Option<Res<ReconnectState>>| gameplay_input_allowed(reconnect))
        .expect("run gameplay_input_allowed");
    assert!(!allowed);
}

#[test]
fn reconnect_does_not_finish_until_fresh_terrain_signal_arrives() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(ReconnectState {
        phase: ReconnectPhase::AwaitingWorld,
        terrain_refresh_seen: false,
    });
    let mut adt = crate::terrain::AdtManager::default();
    adt.map_name = "azeroth".to_string();
    app.insert_resource(adt);
    app.add_systems(Update, finish_reconnect_when_world_ready);
    app.world_mut().spawn(LocalPlayer);

    app.update();

    assert_eq!(
        app.world().resource::<ReconnectState>().phase,
        ReconnectPhase::AwaitingWorld
    );
}

#[test]
fn reconnect_finishes_after_local_player_and_terrain_signal() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(ReconnectState {
        phase: ReconnectPhase::AwaitingWorld,
        terrain_refresh_seen: true,
    });
    app.add_systems(Update, finish_reconnect_when_world_ready);
    app.world_mut().spawn(LocalPlayer);

    app.update();

    let reconnect = app.world().resource::<ReconnectState>();
    assert_eq!(reconnect.phase, ReconnectPhase::Inactive);
    assert!(!reconnect.terrain_refresh_seen);
}

fn sync_test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, sync_replicated_transforms);
    app
}

#[test]
fn sync_updates_rotation_target() {
    let mut app = sync_test_app();
    let entity = app
        .world_mut()
        .spawn((
            NetPosition {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            NetRotation {
                x: 0.0,
                y: 1.5,
                z: 0.0,
            },
            InterpolationTarget { target: Vec3::ZERO },
            RotationTarget { yaw: 0.0 },
            RemoteEntity,
        ))
        .id();
    app.update();
    let rot = app.world().get::<RotationTarget>(entity).unwrap();
    assert!(
        (rot.yaw - 1.5).abs() < 1e-6,
        "rotation target should be 1.5, got {}",
        rot.yaw
    );
}

#[test]
fn sync_updates_interpolation_target() {
    let mut app = sync_test_app();
    let entity = app
        .world_mut()
        .spawn((
            NetPosition {
                x: 10.0,
                y: 20.0,
                z: 30.0,
            },
            InterpolationTarget { target: Vec3::ZERO },
            RemoteEntity,
        ))
        .id();
    app.update();
    let interp = app.world().get::<InterpolationTarget>(entity).unwrap();
    assert_eq!(interp.target, Vec3::new(10.0, 20.0, 30.0));
}

#[test]
fn sync_skips_entities_without_remote_marker() {
    let mut app = sync_test_app();
    let entity = app
        .world_mut()
        .spawn((
            NetPosition {
                x: 5.0,
                y: 6.0,
                z: 7.0,
            },
            InterpolationTarget { target: Vec3::ZERO },
        ))
        .id();
    app.update();
    assert_eq!(
        app.world()
            .get::<InterpolationTarget>(entity)
            .unwrap()
            .target,
        Vec3::ZERO
    );
}

#[test]
fn sync_skips_local_player_even_with_remote_marker() {
    let mut app = sync_test_app();
    let entity = app
        .world_mut()
        .spawn((
            NetPosition {
                x: 5.0,
                y: 6.0,
                z: 7.0,
            },
            InterpolationTarget { target: Vec3::ZERO },
            RemoteEntity,
            LocalPlayer,
        ))
        .id();
    app.update();
    assert_eq!(
        app.world()
            .get::<InterpolationTarget>(entity)
            .unwrap()
            .target,
        Vec3::ZERO
    );
}

fn interp_test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, interpolate_remote_entities);
    app
}

#[test]
fn interpolation_moves_toward_target() {
    let mut app = interp_test_app();
    let start = Vec3::ZERO;
    let target = Vec3::new(10.0, 0.0, 0.0);
    let entity = app
        .world_mut()
        .spawn((
            InterpolationTarget { target },
            Transform::from_translation(start),
            RemoteEntity,
        ))
        .id();

    // First update has zero delta_time; run twice so time advances.
    app.update();
    app.update();

    let pos = app.world().get::<Transform>(entity).unwrap().translation;
    // Should have moved toward target but not reached it in one frame
    assert!(pos.x > 0.0, "should move toward target");
    assert!(pos.x < 10.0, "should not snap to target");
    assert!((pos.y).abs() < 1e-6, "y should stay zero");
}

#[test]
fn interpolation_skips_local_player_even_with_remote_marker() {
    let mut app = interp_test_app();
    let start = Vec3::ZERO;
    let target = Vec3::new(10.0, 20.0, 30.0);
    let entity = app
        .world_mut()
        .spawn((
            InterpolationTarget { target },
            Transform::from_translation(start),
            RemoteEntity,
            LocalPlayer,
        ))
        .id();

    app.update();
    app.update();

    let pos = app.world().get::<Transform>(entity).unwrap().translation;
    assert_eq!(pos, start);
}

#[test]
fn talent_state_update_populates_status_snapshot() {
    let mut snapshot = game_engine::status::TalentStatusSnapshot::default();

    crate::networking_messages::apply_talent_state_update(
        &mut snapshot,
        shared::protocol::TalentStateUpdate {
            snapshot: Some(shared::protocol::TalentSnapshot {
                spec_tabs: vec![shared::protocol::TalentSpecTabSnapshot {
                    name: "Protection".into(),
                    active: true,
                }],
                talents: vec![shared::protocol::TalentNodeSnapshot {
                    talent_id: 101,
                    name: "Divine Strength".into(),
                    points_spent: 1,
                    max_points: 1,
                    active: true,
                }],
                points_remaining: 50,
            }),
            message: Some("talent applied".into()),
            error: None,
        },
    );

    assert_eq!(snapshot.spec_tabs.len(), 1);
    assert_eq!(snapshot.spec_tabs[0].name, "Protection");
    assert_eq!(snapshot.talents.len(), 1);
    assert_eq!(snapshot.talents[0].talent_id, 101);
    assert_eq!(snapshot.points_remaining, 50);
    assert_eq!(
        snapshot.last_server_message.as_deref(),
        Some("talent applied")
    );
    assert_eq!(snapshot.last_error, None);
}

#[test]
fn profession_state_update_populates_status_snapshot() {
    let mut snapshot = game_engine::status::ProfessionStatusSnapshot::default();

    crate::networking_messages::apply_profession_state_update(
        &mut snapshot,
        shared::protocol::ProfessionStateUpdate {
            snapshot: Some(shared::protocol::ProfessionSnapshot {
                skills: vec![shared::protocol::ProfessionSkillSnapshot {
                    profession: "Mining".into(),
                    current: 12,
                    max: 75,
                }],
                recipes: vec![shared::protocol::ProfessionRecipeSnapshot {
                    spell_id: 5001,
                    profession: "Blacksmithing".into(),
                    name: "Copper Bracers".into(),
                    craftable: true,
                    cooldown: None,
                }],
            }),
            message: Some("gathered Copper Vein".into()),
            skill_up: Some(shared::protocol::ProfessionSkillSnapshot {
                profession: "Mining".into(),
                current: 13,
                max: 75,
            }),
            error: None,
        },
    );

    assert_eq!(snapshot.skills.len(), 1);
    assert_eq!(snapshot.skills[0].profession, "Mining");
    assert_eq!(snapshot.recipes.len(), 1);
    assert_eq!(snapshot.recipes[0].spell_id, 5001);
    assert_eq!(
        snapshot.last_server_message.as_deref(),
        Some("gathered Copper Vein")
    );
    assert_eq!(
        snapshot.last_skill_up.as_ref().map(|skill| skill.current),
        Some(13)
    );
    assert_eq!(snapshot.last_error, None);
}

#[test]
fn achievement_state_update_populates_status_snapshot() {
    let mut status = game_engine::status::AchievementsStatusSnapshot::default();
    let mut completion = game_engine::achievements::AchievementCompletionState::default();
    let mut toast = game_engine::achievement::AchievementToastState::default();

    game_engine::achievement::apply_achievement_state_update(
        &mut status,
        &mut completion,
        &mut toast,
        shared::protocol::AchievementStateUpdate {
            snapshot: Some(shared::protocol::AchievementSnapshot {
                earned_ids: vec![1],
                progress: vec![shared::protocol::AchievementProgressSnapshot {
                    achievement_id: 2,
                    current: 12,
                    required: 20,
                    completed: false,
                }],
            }),
            completed: Some(shared::protocol::AchievementToastSnapshot {
                achievement_id: 1,
                name: "Level 10".into(),
                points: 10,
            }),
            message: Some("achievement progress updated".into()),
            error: None,
        },
    );

    assert!(completion.earned.contains(&1));
    assert_eq!(completion.progress.get(&2), Some(&(12, 20)));
    assert_eq!(status.earned_ids, vec![1]);
    assert_eq!(
        status
            .last_completed
            .as_ref()
            .map(|entry| entry.name.as_str()),
        Some("Level 10")
    );
    assert_eq!(toast.queue.len(), 1);
    assert_eq!(
        status.last_server_message.as_deref(),
        Some("achievement progress updated")
    );
}

#[test]
fn world_map_state_update_populates_explored_zones() {
    let mut world_map = game_engine::world_map_data::WorldMapState::default();

    game_engine::world_map::apply_world_map_state_update(
        &mut world_map,
        shared::protocol::WorldMapStateUpdate {
            snapshot: Some(shared::protocol::WorldMapSnapshot {
                discovered_zone_ids: vec![1519, 12, 12],
            }),
            message: Some("world map discovery updated".into()),
            error: None,
        },
    );

    assert_eq!(world_map.fog.explored_zones, vec![12, 1519]);
}

#[test]
fn rest_state_update_populates_character_stats_snapshot() {
    let mut snapshot = game_engine::status::CharacterStatsSnapshot::default();

    crate::networking_messages::apply_rest_state_update(
        &mut snapshot,
        shared::protocol::RestStateUpdate {
            snapshot: Some(shared::protocol::RestSnapshot {
                in_rest_area: true,
                rest_area_kind: Some(shared::protocol::RestAreaKindSnapshot::Inn),
                rested_xp: 75,
                rested_xp_max: 400,
            }),
            message: Some("rest state updated".into()),
            error: None,
        },
    );

    assert!(snapshot.in_rest_area);
    assert_eq!(
        snapshot.rest_area_kind,
        Some(game_engine::status::RestAreaKindEntry::Inn)
    );
    assert_eq!(snapshot.rested_xp, 75);
    assert_eq!(snapshot.rested_xp_max, 400);
}

#[test]
fn reputation_state_update_populates_status_snapshot() {
    let mut snapshot = game_engine::status::ReputationsStatusSnapshot::default();

    crate::networking_messages::apply_reputation_state_update(
        &mut snapshot,
        shared::protocol::ReputationStateUpdate {
            snapshot: Some(shared::protocol::ReputationSnapshot {
                entries: vec![shared::protocol::ReputationEntrySnapshot {
                    faction_id: 72,
                    faction_name: "Stormwind".into(),
                    standing: "Friendly".into(),
                    value: 21_010,
                }],
            }),
            message: Some("gained 10 reputation with Stormwind".into()),
            error: None,
        },
    );

    assert_eq!(snapshot.entries.len(), 1);
    assert_eq!(snapshot.entries[0].faction_id, 72);
    assert_eq!(snapshot.entries[0].standing, "Friendly");
    assert_eq!(
        snapshot.last_server_message.as_deref(),
        Some("gained 10 reputation with Stormwind")
    );
    assert_eq!(snapshot.last_error, None);
}

#[test]
fn currency_state_update_populates_status_snapshot() {
    let mut snapshot = game_engine::status::CurrenciesStatusSnapshot::default();

    game_engine::currency::apply_currency_state_update(
        &mut snapshot,
        shared::protocol::CurrencyStateUpdate {
            snapshot: Some(shared::protocol::CurrencySnapshot {
                entries: vec![shared::protocol::CurrencyEntrySnapshot {
                    id: 1,
                    name: "Honor".into(),
                    amount: 75,
                }],
            }),
            message: Some("earned 75 Honor".into()),
            error: None,
        },
    );

    assert_eq!(snapshot.entries.len(), 1);
    assert_eq!(snapshot.entries[0].id, 1);
    assert_eq!(snapshot.entries[0].name, "Honor");
    assert_eq!(snapshot.entries[0].amount, 75);
    assert_eq!(
        snapshot.last_server_message.as_deref(),
        Some("earned 75 Honor")
    );
    assert_eq!(snapshot.last_error, None);
}

#[test]
fn durability_state_update_populates_status_snapshot() {
    let mut snapshot = game_engine::status::DurabilityStatusSnapshot::default();

    game_engine::durability::apply_durability_state_update(
        &mut snapshot,
        shared::protocol::DurabilityStateUpdate {
            snapshot: Some(shared::protocol::DurabilitySnapshot {
                total_repair_cost: 330,
                slots: vec![
                    shared::protocol::DurabilitySlotSnapshot {
                        slot: shared::components::EquipmentVisualSlot::Head,
                        current: 60,
                        max: 70,
                        repair_cost: 150,
                    },
                    shared::protocol::DurabilitySlotSnapshot {
                        slot: shared::components::EquipmentVisualSlot::Chest,
                        current: 70,
                        max: 80,
                        repair_cost: 180,
                    },
                ],
            }),
            message: Some("durability updated".into()),
            error: None,
        },
    );

    assert_eq!(snapshot.entries.len(), 2);
    assert_eq!(
        snapshot.entries[0].slot,
        shared::components::EquipmentVisualSlot::Head
    );
    assert_eq!(snapshot.entries[0].current, 60);
    assert_eq!(snapshot.total_repair_cost, 330);
    assert_eq!(
        snapshot.last_server_message.as_deref(),
        Some("durability updated")
    );
    assert_eq!(snapshot.last_error, None);
}

#[test]
fn collection_state_update_populates_status_snapshot() {
    let mut snapshot = game_engine::status::CollectionStatusSnapshot::default();

    game_engine::collection::apply_collection_state_update(
        &mut snapshot,
        shared::protocol::CollectionStateUpdate {
            snapshot: Some(shared::protocol::CollectionSnapshot {
                mounts: vec![shared::protocol::CollectionMountSnapshot {
                    mount_id: 101,
                    name: "Swift Brown Steed".into(),
                    known: true,
                    active: true,
                }],
                pets: vec![shared::protocol::CollectionPetSnapshot {
                    pet_id: 202,
                    name: "Brown Rabbit".into(),
                    known: true,
                    active: false,
                }],
            }),
            message: Some("summoned Swift Brown Steed".into()),
            error: None,
        },
    );

    assert_eq!(snapshot.mounts.len(), 1);
    assert_eq!(snapshot.mounts[0].mount_id, 101);
    assert!(snapshot.mounts[0].active);
    assert_eq!(snapshot.pets.len(), 1);
    assert_eq!(snapshot.pets[0].pet_id, 202);
    assert_eq!(
        snapshot.last_server_message.as_deref(),
        Some("summoned Swift Brown Steed")
    );
    assert_eq!(snapshot.last_error, None);
}

#[test]
fn inspect_state_update_populates_status_snapshot() {
    let mut snapshot = game_engine::status::InspectStatusSnapshot::default();

    crate::networking_messages::apply_inspect_state_update(
        &mut snapshot,
        shared::protocol::InspectStateUpdate {
            snapshot: Some(shared::protocol::InspectSnapshot {
                target_name: "Alice".into(),
                equipment_appearance: shared::components::EquipmentAppearance {
                    entries: vec![shared::components::EquippedAppearanceEntry {
                        slot: shared::components::EquipmentVisualSlot::Head,
                        item_id: Some(100),
                        display_info_id: Some(200),
                        inventory_type: 1,
                        hidden: false,
                    }],
                },
                talents: shared::protocol::TalentSnapshot {
                    spec_tabs: vec![shared::protocol::TalentSpecTabSnapshot {
                        name: "Protection".into(),
                        active: true,
                    }],
                    talents: vec![shared::protocol::TalentNodeSnapshot {
                        talent_id: 101,
                        name: "Divine Strength".into(),
                        points_spent: 1,
                        max_points: 1,
                        active: true,
                    }],
                    points_remaining: 50,
                },
            }),
            message: Some("inspect ready".into()),
            error: None,
        },
    );

    assert_eq!(snapshot.target_name.as_deref(), Some("Alice"));
    assert_eq!(snapshot.equipment_appearance.entries.len(), 1);
    assert_eq!(snapshot.spec_tabs.len(), 1);
    assert_eq!(snapshot.talents.len(), 1);
    assert_eq!(snapshot.points_remaining, 50);
    assert_eq!(
        snapshot.last_server_message.as_deref(),
        Some("inspect ready")
    );
    assert_eq!(snapshot.last_error, None);
}

#[test]
fn duel_state_update_populates_status_snapshot() {
    let mut snapshot = game_engine::status::DuelStatusSnapshot::default();

    crate::networking_messages::apply_duel_state_update(
        &mut snapshot,
        shared::protocol::DuelStateUpdate {
            snapshot: Some(shared::protocol::DuelSnapshot {
                phase: shared::protocol::DuelPhaseSnapshot::Active,
                opponent_name: "Alice".into(),
                boundary: Some(shared::protocol::DuelBoundarySnapshot {
                    center_x: 10.0,
                    center_z: 15.0,
                    radius: 30.0,
                }),
                result: Some(shared::protocol::DuelResultSnapshot::Won),
            }),
            message: Some("duel started".into()),
            error: None,
        },
    );

    assert_eq!(
        snapshot.phase,
        Some(game_engine::status::DuelPhaseEntry::Active)
    );
    assert_eq!(snapshot.opponent_name.as_deref(), Some("Alice"));
    assert_eq!(
        snapshot.boundary.as_ref().map(|boundary| boundary.radius),
        Some(30.0)
    );
    assert_eq!(
        snapshot.last_result,
        Some(game_engine::status::DuelResultEntry::Won)
    );
    assert_eq!(
        snapshot.last_server_message.as_deref(),
        Some("duel started")
    );
    assert_eq!(snapshot.last_error, None);
}

#[test]
fn chat_log_caps_at_max() {
    let mut log = ChatLog::default();
    for i in 0..101 {
        log.messages
            .push((format!("player{i}"), format!("msg{i}"), ChatType::Say));
        if log.messages.len() > MAX_CHAT_LOG {
            log.messages.remove(0);
        }
    }
    assert_eq!(log.messages.len(), MAX_CHAT_LOG);
    assert_eq!(log.messages[0].0, "player1");
    assert_eq!(log.messages[99].0, "player100");
}

fn selected_with_name(name: &str) -> SelectedCharacterId {
    SelectedCharacterId {
        character_id: Some(1),
        character_name: Some(name.to_string()),
    }
}

#[test]
fn is_local_player_entity_matches_selected() {
    let selected = selected_with_name("Theron");
    assert!(is_local_player_entity("Theron", Some(&selected)));
}

#[test]
fn is_local_player_entity_rejects_different() {
    let selected = selected_with_name("Theron");
    assert!(!is_local_player_entity("OtherPlayer", Some(&selected)));
}

#[test]
fn is_local_player_entity_none_without_resource() {
    assert!(!is_local_player_entity("Theron", None));
}

#[test]
fn is_local_player_entity_none_when_not_selected() {
    let selected = SelectedCharacterId::default();
    assert!(!is_local_player_entity("Theron", Some(&selected)));
}

fn net_player(name: &str) -> NetPlayer {
    NetPlayer {
        name: name.into(),
        race: 1,
        class: 1,
        appearance: CharacterAppearance::default(),
    }
}

#[test]
fn choose_local_player_entity_prefers_newest_matching_entity() {
    let older = Entity::from_bits(10);
    let newer = Entity::from_bits(20);
    let other = Entity::from_bits(30);
    let theron = net_player("Theron");
    let other_player = net_player("Other");

    let (chosen, matches) = choose_local_player_entity(
        "Theron",
        [(older, &theron), (other, &other_player), (newer, &theron)].into_iter(),
    );

    assert_eq!(matches, 2);
    assert_eq!(chosen, Some(newer));
}

#[test]
fn choose_local_player_entity_returns_none_when_name_missing() {
    let player = net_player("Other");
    let (chosen, matches) =
        choose_local_player_entity("Theron", [(Entity::from_bits(1), &player)].into_iter());

    assert_eq!(matches, 0);
    assert_eq!(chosen, None);
}

#[test]
fn net_position_to_bevy_passes_through_unchanged() {
    // Server already sends Bevy-space coordinates.
    let pos = NetPosition {
        x: -8949.0,
        y: 83.0,
        z: 132.0,
    };

    assert_eq!(net_position_to_bevy(&pos), Vec3::new(-8949.0, 83.0, 132.0));
}

#[test]
fn net_player_customization_selection_uses_player_race_class_and_appearance() {
    let player = NetPlayer {
        name: "Theron".into(),
        race: 10,
        class: 8,
        appearance: CharacterAppearance {
            sex: 1,
            skin_color: 2,
            face: 3,
            eye_color: 0,
            hair_style: 4,
            hair_color: 5,
            facial_style: 6,
        },
    };

    let selection = net_player_customization_selection(&player);

    assert_eq!(selection.race, 10);
    assert_eq!(selection.class, 8);
    assert_eq!(selection.sex, 1);
    assert_eq!(selection.appearance, player.appearance);
}

#[test]
fn resolve_player_model_path_uses_player_race_and_sex() {
    let player = NetPlayer {
        name: "Theron".into(),
        race: 10,
        class: 8,
        appearance: CharacterAppearance {
            sex: 1,
            ..Default::default()
        },
    };

    let path = resolve_player_model_path(&player).expect("player model path should resolve");

    assert!(
        path.to_string_lossy()
            .to_ascii_lowercase()
            .contains("bloodelffemale"),
        "expected bloodelf female model path, got {}",
        path.display()
    );
}

#[test]
fn terrain_messages_are_processed_before_inworld_transition() {
    assert!(terrain_messages_allowed_in_state(
        crate::game_state::GameState::CharSelect
    ));
}

#[test]
fn queue_despawn_if_exists_removes_live_entity() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let entity = app.world_mut().spawn_empty().id();

    let _ = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            queue_despawn_if_exists(&mut commands, entity);
        });
    app.update();

    assert!(app.world().get_entity(entity).is_err());
}

#[test]
fn queue_despawn_if_exists_ignores_missing_entity() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let entity = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(entity).despawn();

    let _ = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            queue_despawn_if_exists(&mut commands, entity);
        });
    app.update();

    assert!(app.world().get_entity(entity).is_err());
}

#[test]
fn npc_visibility_policy_hides_debug_pedestals_and_turkeys() {
    assert_eq!(npc_visibility_policy(26741), NpcVisibilityPolicy::Hidden);
    assert_eq!(npc_visibility_policy(32820), NpcVisibilityPolicy::Hidden);
}

#[test]
fn npc_visibility_policy_only_shows_spirit_healer_when_dead() {
    assert_eq!(npc_visibility_policy(6491), NpcVisibilityPolicy::DeadOnly);
}

#[test]
fn sync_local_alive_state_tracks_local_player_health() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<LocalAliveState>();
    app.add_systems(Update, sync_local_alive_state);
    app.world_mut().spawn((
        LocalPlayer,
        NetHealth {
            current: 0.0,
            max: 100.0,
        },
    ));

    app.update();

    assert!(!app.world().resource::<LocalAliveState>().0);
}

#[test]
fn disconnect_during_game_menu_reconnects_without_bouncing_to_login() {
    let mut app = disconnect_app_with_state(crate::game_state::GameState::GameMenu);
    let (client, replicated) = populate_inworld_disconnect_entities(&mut app);
    trigger_disconnect_entity(&mut app, client);

    app.update();
    app.update();

    // Should transition to InWorld (not Login) and arm reconnect.
    let state = app
        .world()
        .resource::<State<crate::game_state::GameState>>();
    assert_eq!(
        *state.get(),
        crate::game_state::GameState::InWorld,
        "GameMenu disconnect should transition to InWorld, not Login"
    );
    assert_eq!(
        app.world().resource::<ReconnectState>().phase,
        ReconnectPhase::PendingConnect
    );
    assert!(app.world().get_entity(client).is_err());
    assert!(app.world().get_entity(replicated).is_err());
}
