use bevy::prelude::*;
use lightyear::prelude::client::{Client, Disconnected};
use shared::protocol::ForcedDisconnect;

use super::*;

pub(crate) fn handle_client_disconnected(
    trigger: On<Add, Disconnected>,
    disconnected_q: Query<&Disconnected, With<Client>>,
    state: Res<State<crate::game_state::GameState>>,
    auth_token: Option<Res<AuthToken>>,
    selected: Option<Res<SelectedCharacterId>>,
    reconnect: Option<ResMut<ReconnectState>>,
    mut forced_disconnect: ResMut<PendingForcedDisconnect>,
    mut auth_feedback: ResMut<AuthUiFeedback>,
    mut next_state: ResMut<NextState<crate::game_state::GameState>>,
    mut commands: Commands,
) {
    let Ok(disconnected) = disconnected_q.get(trigger.entity) else {
        return;
    };
    let forced_notice = forced_disconnect.0.take();
    let auth_token_label = auth_token
        .as_deref()
        .map(|token| crate::networking_auth::token_debug_label(token.0.as_deref()))
        .unwrap_or_else(|| "resource-missing".to_string());
    let selected_name = selected.as_deref().and_then(|s| s.character_name.clone());
    let selected_id = selected.as_deref().and_then(|s| s.character_id);
    let reconnect_phase = reconnect.as_deref().map(|s| s.phase);
    let reason = forced_notice
        .as_ref()
        .map(|notice| notice.message.as_str())
        .or(disconnected.reason.as_deref())
        .unwrap_or("connection lost");
    warn!(
        "Client entity {:?} disconnected in {:?}: {reason}; token={} selected_id={selected_id:?} selected_name={selected_name:?} reconnect_phase={reconnect_phase:?}",
        trigger.entity,
        state.get(),
        auth_token_label,
    );
    handle_disconnect_by_state(
        &state,
        DisconnectInputs {
            auth_token,
            selected,
            reconnect,
            selected_name: selected_name.as_deref(),
        },
        forced_notice,
        &mut auth_feedback,
        &mut next_state,
        &mut commands,
        trigger.entity,
    );
}

struct DisconnectInputs<'a, 'b, 'c, 'd> {
    auth_token: Option<Res<'a, AuthToken>>,
    selected: Option<Res<'b, SelectedCharacterId>>,
    reconnect: Option<ResMut<'c, ReconnectState>>,
    selected_name: Option<&'d str>,
}

fn handle_disconnect_by_state(
    state: &Res<State<crate::game_state::GameState>>,
    inputs: DisconnectInputs<'_, '_, '_, '_>,
    forced_notice: Option<ForcedDisconnect>,
    auth_feedback: &mut ResMut<AuthUiFeedback>,
    next_state: &mut ResMut<NextState<crate::game_state::GameState>>,
    commands: &mut Commands,
    entity: Entity,
) {
    let DisconnectInputs {
        auth_token,
        selected,
        reconnect,
        selected_name,
    } = inputs;
    if let Some(notice) = forced_notice {
        handle_forced_disconnect(
            state.get(),
            reconnect,
            notice,
            auth_feedback,
            next_state,
            commands,
        );
        return;
    }
    match *state.get() {
        crate::game_state::GameState::CharSelect => {
            handle_charselect_disconnect(auth_token, reconnect, auth_feedback, commands);
        }
        crate::game_state::GameState::Login => handle_disconnect_from_login(auth_feedback),
        state if is_inworld_disconnect_state(state) => handle_inworld_disconnect(
            auth_token,
            selected,
            reconnect,
            auth_feedback,
            next_state,
            commands,
            selected_name,
        ),
        crate::game_state::GameState::Connecting => handle_disconnect_while_connecting(entity),
        _ => handle_disconnect_to_login_fallback(state.get(), entity, auth_feedback, next_state),
    }
}

fn is_inworld_disconnect_state(state: crate::game_state::GameState) -> bool {
    matches!(
        state,
        crate::game_state::GameState::InWorld
            | crate::game_state::GameState::GameMenu
            | crate::game_state::GameState::Loading
            | crate::game_state::GameState::CampsitePopup
    )
}

fn handle_forced_disconnect(
    state: &crate::game_state::GameState,
    reconnect: Option<ResMut<ReconnectState>>,
    notice: ForcedDisconnect,
    auth_feedback: &mut ResMut<AuthUiFeedback>,
    next_state: &mut ResMut<NextState<crate::game_state::GameState>>,
    commands: &mut Commands,
) {
    if let Some(mut reconnect) = reconnect
        && reconnect.is_active()
    {
        reconnect.phase = ReconnectPhase::Inactive;
        reconnect.terrain_refresh_seen = false;
    }
    commands.queue(crate::networking_reconnect::reset_network_world);
    auth_feedback.0 = Some(notice.message);
    if *state != crate::game_state::GameState::Login {
        next_state.set(crate::game_state::GameState::Login);
    }
}

fn handle_disconnect_from_login(auth_feedback: &mut ResMut<AuthUiFeedback>) {
    info!("Disconnect handling: already in Login, surfacing connection-lost message");
    auth_feedback.0 = Some("Connection lost.".to_string());
}

fn handle_disconnect_while_connecting(entity: Entity) {
    info!(
        "Disconnect handling: ignoring transient disconnect while still connecting for client entity {:?}",
        entity
    );
}

fn handle_disconnect_to_login_fallback(
    state: &crate::game_state::GameState,
    entity: Entity,
    auth_feedback: &mut ResMut<AuthUiFeedback>,
    next_state: &mut ResMut<NextState<crate::game_state::GameState>>,
) {
    warn!(
        "Disconnect handling: transitioning from {:?} to Login due to disconnect on client entity {:?}",
        state, entity
    );
    auth_feedback.0 = Some("Connection lost.".to_string());
    next_state.set(crate::game_state::GameState::Login);
}

fn handle_charselect_disconnect(
    auth_token: Option<Res<AuthToken>>,
    reconnect: Option<ResMut<ReconnectState>>,
    auth_feedback: &mut ResMut<AuthUiFeedback>,
    commands: &mut Commands,
) {
    if auth_token
        .as_deref()
        .and_then(|t| t.0.as_deref())
        .is_none_or(|t| t.trim().is_empty())
    {
        info!("Disconnect handling: CharSelect has no saved auth token, staying offline");
        auth_feedback.0 = Some("Connection lost. Char select is now offline.".to_string());
        return;
    }
    let Some(mut reconnect) = reconnect else {
        warn!("Disconnect handling: CharSelect missing ReconnectState, staying offline");
        auth_feedback.0 = Some("Connection lost. Char select is now offline.".to_string());
        return;
    };
    commands.insert_resource(LoginMode::Login);
    commands.insert_resource(LoginUsername(String::new()));
    commands.insert_resource(LoginPassword(String::new()));
    commands.queue(crate::networking_reconnect::reset_network_world);
    reconnect.phase = ReconnectPhase::PendingConnect;
    reconnect.terrain_refresh_seen = false;
    auth_feedback.0 = None;
    info!(
        "Disconnect handling: queued CharSelect reconnect with phase {:?}",
        reconnect.phase
    );
}

fn handle_inworld_disconnect(
    auth_token: Option<Res<AuthToken>>,
    selected: Option<Res<SelectedCharacterId>>,
    reconnect: Option<ResMut<ReconnectState>>,
    auth_feedback: &mut ResMut<AuthUiFeedback>,
    next_state: &mut ResMut<NextState<crate::game_state::GameState>>,
    commands: &mut Commands,
    selected_name: Option<&str>,
) {
    if auth_token
        .as_deref()
        .and_then(|t| t.0.as_deref())
        .is_none_or(|t| t.trim().is_empty())
    {
        warn!("Disconnect handling: no saved auth token available, returning to Login");
        auth_feedback.0 = Some("Connection lost.".to_string());
        next_state.set(crate::game_state::GameState::Login);
        return;
    }
    let Some(mut reconnect) = reconnect else {
        warn!("Disconnect handling: ReconnectState missing, returning to Login");
        auth_feedback.0 = Some("Connection lost.".to_string());
        next_state.set(crate::game_state::GameState::Login);
        return;
    };
    if let Some(name) = selected.as_deref().and_then(|s| s.character_name.clone()) {
        commands.insert_resource(crate::scenes::char_select::PreselectedCharName(name));
    }
    next_state.set(crate::game_state::GameState::InWorld);
    commands.insert_resource(crate::scenes::char_select::AutoEnterWorld);
    commands.insert_resource(LoginMode::Login);
    commands.insert_resource(LoginUsername(String::new()));
    commands.insert_resource(LoginPassword(String::new()));
    commands.queue(crate::networking_reconnect::reset_network_world);
    reconnect.phase = ReconnectPhase::PendingConnect;
    reconnect.terrain_refresh_seen = false;
    auth_feedback.0 = None;
    info!(
        "Disconnect handling: queued reconnect with phase {:?}, preselected_name={selected_name:?}",
        reconnect.phase
    );
}
