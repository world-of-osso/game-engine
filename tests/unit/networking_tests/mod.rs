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
use shared::protocol::ForcedDisconnect;

mod disconnect;
mod movement;
mod player_misc;
mod status_updates;
mod sync_interp;

fn make_state(direction: MoveDirection) -> MovementState {
    MovementState {
        direction,
        ..Default::default()
    }
}

fn make_facing(yaw: f32) -> CharacterFacing {
    CharacterFacing { yaw }
}

fn charselect_disconnect_app(token: Option<&str>) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.insert_state(crate::game_state::GameState::CharSelect);
    app.init_resource::<AuthUiFeedback>();
    app.init_resource::<ReconnectState>();
    app.init_resource::<PendingForcedDisconnect>();
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

fn disconnect_app_with_state(state: crate::game_state::GameState) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.insert_state(state);
    app.init_resource::<AuthUiFeedback>();
    app.init_resource::<ReconnectState>();
    app.init_resource::<PendingForcedDisconnect>();
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
    assert_inworld_reconnect_phase(app);
    assert_inworld_reconnect_selection(app);
    assert_inworld_entities_cleared(app, client, replicated);
    assert_inworld_ui_state_cleared(app);
}

fn assert_inworld_reconnect_phase(app: &App) {
    let state = app
        .world()
        .resource::<State<crate::game_state::GameState>>();
    assert_eq!(*state.get(), crate::game_state::GameState::InWorld);
    assert_eq!(
        app.world().resource::<ReconnectState>().phase,
        ReconnectPhase::PendingConnect
    );
}

fn assert_inworld_reconnect_selection(app: &App) {
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
}

fn assert_inworld_entities_cleared(app: &App, client: Entity, replicated: Entity) {
    assert!(app.world().get_entity(client).is_err());
    assert!(app.world().get_entity(replicated).is_err());
    assert!(
        app.world()
            .resource::<game_engine::targeting::CurrentTarget>()
            .0
            .is_none()
    );
}

fn assert_inworld_ui_state_cleared(app: &App) {
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

fn sync_test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, sync_replicated_transforms);
    app
}

fn interp_test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, interpolate_remote_entities);
    app
}

fn selected_with_name(name: &str) -> SelectedCharacterId {
    SelectedCharacterId {
        character_id: Some(1),
        character_name: Some(name.to_string()),
    }
}

fn net_player(name: &str) -> NetPlayer {
    NetPlayer {
        name: name.into(),
        race: 1,
        class: 1,
        appearance: CharacterAppearance::default(),
    }
}
