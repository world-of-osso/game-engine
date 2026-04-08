use std::time::Duration;

use bevy::prelude::*;
use bevy::ui::{AlignItems, BackgroundColor, JustifyContent, Node, PositionType, Val};
use core::net::{IpAddr, Ipv4Addr, SocketAddr};
use lightyear::prelude::client::*;
use lightyear::prelude::*;
use shared::components::{Position as NetPosition, Rotation as NetRotation};
use shared::protocol::ChatMessage;
pub use shared::protocol::ChatType;

pub use crate::networking_auth::{
    AuthToken, AuthUiFeedback, CharacterList, LoginMode, LoginPassword, LoginUsername,
    SelectedCharacterId, load_auth_token,
};

use crate::camera::{CharacterFacing, MovementState};
use game_engine::status::{
    CollectionStatusSnapshot, CombatLogStatusSnapshot, CurrenciesStatusSnapshot,
    GroupStatusSnapshot, GuildVaultStatusSnapshot, InventorySearchSnapshot, MapStatusSnapshot,
    ProfessionStatusSnapshot, QuestLogStatusSnapshot, ReputationsStatusSnapshot,
    WarbankStatusSnapshot,
};

/// Marker for entities spawned from server replication.
#[derive(Component)]
pub struct RemoteEntity;

/// Marker for the local player entity (the one this client controls).
#[derive(Component)]
pub struct LocalPlayer;

/// Our client_id, stored at connection time so we can identify our own replicated player.
#[derive(Resource)]
pub struct LocalClientId(pub u64);

/// Target position for smooth interpolation of remote entities.
#[derive(Component)]
pub(crate) struct InterpolationTarget {
    pub(crate) target: Vec3,
}

#[derive(Component)]
pub(crate) struct ReplicatedVisualEntity;

#[derive(Component, Clone, Debug, PartialEq)]
pub(crate) struct ResolvedModelAssetInfo {
    pub model_path: String,
    pub skin_path: Option<String>,
    pub display_scale: Option<f32>,
}

/// Maximum number of messages stored in the chat log.
pub(crate) const MAX_CHAT_LOG: usize = 100;

/// Tracks the zone the local player is currently in (replicated from server).
#[derive(Resource, Default)]
pub struct CurrentZone {
    pub zone_id: u32,
}

/// Whether the local player is currently alive according to replicated health.
#[derive(Resource, Debug, Clone, Copy)]
pub struct LocalAliveState(pub bool);

impl Default for LocalAliveState {
    fn default() -> Self {
        Self(true)
    }
}

/// Chat log storing received messages: (sender, content, chat_type).
#[derive(Resource, Default)]
pub struct ChatLog {
    pub messages: Vec<(String, String, ChatType)>,
}

/// Resource for other systems to queue outgoing chat messages.
#[derive(Resource, Default)]
pub struct ChatInput(pub Option<ChatMessage>);

/// Interpolation speed: 1 / interval between server ticks (~100ms at 20Hz).
const INTERPOLATION_SPEED: f32 = 10.0;

const CLIENT_PORT: u16 = 0; // OS-assigned ephemeral port
const TICK_RATE_HZ: f64 = 20.0;
pub(crate) const MAX_COMBAT_LOG: usize = 200;

/// Resource holding the server address to connect to.
#[derive(Resource)]
pub struct ServerAddr(pub SocketAddr);

/// Original server string before DNS resolution (e.g. "game.worldofosso.com:5000").
/// Used for per-server token storage keying.
#[derive(Resource)]
pub struct ServerHostname(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReconnectPhase {
    #[default]
    Inactive,
    PendingConnect,
    AwaitingWorld,
}

#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ReconnectState {
    pub phase: ReconnectPhase,
    pub terrain_refresh_seen: bool,
}

impl ReconnectState {
    pub fn is_active(&self) -> bool {
        self.phase != ReconnectPhase::Inactive
    }

    fn overlay_text(&self) -> &'static str {
        match self.phase {
            ReconnectPhase::Inactive => "",
            ReconnectPhase::PendingConnect => "Reconnecting...",
            ReconnectPhase::AwaitingWorld => "Re-synchronizing world...",
        }
    }
}

#[derive(Component)]
struct ReconnectOverlayRoot;

#[derive(Component)]
struct ReconnectOverlayText;

/// Target rotation for smooth interpolation of remote entities.
#[derive(Component)]
pub(crate) struct RotationTarget {
    pub(crate) yaw: f32,
}

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ClientPlugins {
            tick_duration: Duration::from_secs_f64(1.0 / TICK_RATE_HZ),
        });
        app.add_plugins(shared::ProtocolPlugin);
        register_net_resources(app);
        register_net_systems(app);
        register_net_observers(app);
        app.add_plugins(crate::scenes::inworld_tree::InWorldSceneTreePlugin);
    }
}

fn register_net_resources(app: &mut App) {
    app.init_resource::<CurrentZone>();
    app.init_resource::<LocalAliveState>();
    app.init_resource::<ChatLog>();
    app.init_resource::<ChatInput>();
    app.insert_resource(game_engine::chat_data::ChatState {
        max_messages: MAX_CHAT_LOG,
        ..Default::default()
    });
    app.insert_resource(game_engine::chat_data::WhisperState {
        max_recent: 10,
        ..Default::default()
    });
    app.init_resource::<ReconnectState>();
    let server = app
        .world()
        .get_resource::<ServerHostname>()
        .map(|h| h.0.clone());
    app.insert_resource(AuthToken(load_auth_token(server.as_deref())));
    app.init_resource::<AuthUiFeedback>();
    app.init_resource::<CharacterList>();
    app.init_resource::<SelectedCharacterId>();
    app.init_resource::<LoginUsername>();
    app.init_resource::<LoginPassword>();
    app.init_resource::<LoginMode>();
    app.init_resource::<QuestLogStatusSnapshot>();
    app.init_resource::<GroupStatusSnapshot>();
    app.init_resource::<CombatLogStatusSnapshot>();
    app.init_resource::<CollectionStatusSnapshot>();
    app.init_resource::<ProfessionStatusSnapshot>();
    app.init_resource::<MapStatusSnapshot>();
    app.init_resource::<CurrenciesStatusSnapshot>();
    app.init_resource::<ReputationsStatusSnapshot>();
    app.init_resource::<GuildVaultStatusSnapshot>();
    app.init_resource::<WarbankStatusSnapshot>();
    app.init_resource::<InventorySearchSnapshot>();
}

fn register_net_systems(app: &mut App) {
    app.add_systems(
        OnEnter(crate::game_state::GameState::Connecting),
        connect_to_server,
    );
    app.add_systems(Startup, spawn_reconnect_overlay);
    register_gameplay_net_systems(app);
    register_auth_net_systems(app);
    app.add_systems(
        Update,
        (
            crate::networking_reconnect::drive_inworld_reconnect,
            update_reconnect_overlay,
            crate::networking_reconnect::finish_reconnect_when_world_ready,
        ),
    );
}

fn register_gameplay_net_systems(app: &mut App) {
    use crate::game_state::GameState;
    use crate::networking_messages as msg;
    app.add_systems(
        Update,
        (
            msg::send_player_input,
            msg::send_chat_message,
            msg::receive_chat_messages,
            msg::send_target_to_server,
            msg::track_player_zone,
            crate::status_sync::sync_map_status_snapshot,
            msg::receive_quest_log_snapshot,
            msg::receive_group_roster_snapshot,
        )
            .run_if(in_state(GameState::InWorld)),
    );
    register_inworld_sync_systems(app);
}

fn register_inworld_sync_systems(app: &mut App) {
    use crate::game_state::GameState;
    use crate::networking_messages as msg;
    app.add_systems(
        Update,
        msg::receive_load_terrain.run_if(should_receive_load_terrain),
    );
    app.add_systems(
        Update,
        (
            msg::receive_group_command_response,
            msg::receive_combat_log_snapshot,
            msg::receive_combat_events,
            msg::receive_collection_state_update,
            msg::receive_profession_snapshot,
            msg::receive_reputation_snapshot,
            msg::receive_guild_vault_snapshot,
            msg::receive_warbank_snapshot,
            msg::receive_inventory_search_snapshot,
            sync_replicated_transforms,
            crate::networking_player::sync_replicated_player_customization,
            interpolate_remote_entities,
        )
            .run_if(in_state(GameState::InWorld)),
    );
    register_entity_tag_systems(app);
}

fn register_entity_tag_systems(app: &mut App) {
    use crate::game_state::GameState;
    app.add_systems(
        Update,
        (
            crate::networking_player::tag_local_player,
            crate::networking_auth::sync_selected_character_roster_entry,
            crate::networking_player::sync_local_alive_state,
            crate::networking_npc::apply_npc_visibility_policy,
        )
            .chain()
            .run_if(in_state(GameState::Loading).or(in_state(GameState::InWorld))),
    );
}

fn terrain_messages_allowed_in_state(state: crate::game_state::GameState) -> bool {
    matches!(
        state,
        crate::game_state::GameState::CharSelect
            | crate::game_state::GameState::Loading
            | crate::game_state::GameState::InWorld
    )
}

fn should_receive_load_terrain(state: Res<State<crate::game_state::GameState>>) -> bool {
    terrain_messages_allowed_in_state(*state.get())
}

fn register_auth_net_systems(app: &mut App) {
    use crate::networking_auth as auth;
    app.add_systems(
        Update,
        (
            auth::receive_login_response,
            auth::receive_create_character_response,
            auth::receive_delete_character_response,
            auth::receive_character_list_update,
            auth::receive_enter_world_response,
            auth::receive_register_response,
        ),
    );
}

fn register_net_observers(app: &mut App) {
    app.add_observer(on_connected);
    app.add_observer(on_link_established);
    app.add_observer(handle_client_disconnected);
    app.add_observer(crate::networking_player::spawn_replicated_player);
    app.add_observer(crate::networking_npc::spawn_replicated_npc);
    app.add_observer(cleanup_disconnected_player);
}

fn connect_to_server(mut commands: Commands, server_addr: Res<ServerAddr>) {
    connect_to_server_inner(&mut commands, server_addr.0);
}

pub(crate) fn connect_to_server_inner(commands: &mut Commands, server_addr: SocketAddr) {
    let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), CLIENT_PORT);
    let client_id = crate::networking_reconnect::rand_client_id();
    commands.insert_resource(LocalClientId(client_id));
    let auth = Authentication::Manual {
        server_addr,
        client_id,
        private_key: [0; 32],
        protocol_id: 0,
    };
    let netcode = match NetcodeClient::new(
        auth,
        NetcodeConfig {
            client_timeout_secs: 60,
            ..NetcodeConfig::default()
        },
    ) {
        Ok(nc) => nc,
        Err(e) => {
            error!("Failed to create netcode client: {e}");
            return;
        }
    };
    let entity = commands
        .spawn((
            LocalAddr(bind_addr),
            PeerAddr(server_addr),
            UdpIo::default(),
            netcode,
        ))
        .id();
    commands.trigger(Connect { entity });
    info!(
        "Connecting to server at {server_addr} with client_entity={entity:?} client_id={client_id}"
    );
}

fn on_link_established(trigger: On<Add, Connected>, mut commands: Commands) {
    info!(
        "Transport link established for client entity {:?}; inserting ReplicationReceiver",
        trigger.entity
    );
    commands
        .entity(trigger.entity)
        .insert(ReplicationReceiver::default());
}

fn on_connected(
    trigger: On<Add, Connected>,
    auth_token: Res<AuthToken>,
    username: Res<LoginUsername>,
    password: Res<LoginPassword>,
    login_mode: Res<LoginMode>,
    reconnect: Option<ResMut<ReconnectState>>,
    mut login_senders: Query<&mut MessageSender<shared::protocol::LoginRequest>>,
    mut register_senders: Query<&mut MessageSender<shared::protocol::RegisterRequest>>,
) {
    let reconnect_phase_before = reconnect.as_deref().map(|r| r.phase);
    info!(
        "Connected to server on client entity {:?}; login_mode={:?} username='{}' password_present={} token={} reconnect_phase_before={:?}",
        trigger.entity,
        *login_mode,
        username.0,
        !password.0.is_empty(),
        crate::networking_auth::token_debug_label(auth_token.0.as_deref()),
        reconnect_phase_before,
    );
    if let Some(mut reconnect) = reconnect
        && reconnect.is_active()
    {
        reconnect.phase = ReconnectPhase::AwaitingWorld;
        info!(
            "Reconnect state advanced to {:?} after successful connect",
            reconnect.phase
        );
    }
    crate::networking_auth::send_auth_request(
        &auth_token,
        &username,
        &password,
        &login_mode,
        &mut login_senders,
        &mut register_senders,
    );
}

fn handle_client_disconnected(
    trigger: On<Add, Disconnected>,
    disconnected_q: Query<&Disconnected, With<Client>>,
    state: Res<State<crate::game_state::GameState>>,
    auth_token: Option<Res<AuthToken>>,
    selected: Option<Res<SelectedCharacterId>>,
    reconnect: Option<ResMut<ReconnectState>>,
    mut auth_feedback: ResMut<AuthUiFeedback>,
    mut next_state: ResMut<NextState<crate::game_state::GameState>>,
    mut commands: Commands,
) {
    let Ok(disconnected) = disconnected_q.get(trigger.entity) else {
        return;
    };
    let auth_token_label = auth_token
        .as_deref()
        .map(|token| crate::networking_auth::token_debug_label(token.0.as_deref()))
        .unwrap_or_else(|| "resource-missing".to_string());
    let selected_name = selected.as_deref().and_then(|s| s.character_name.clone());
    let selected_id = selected.as_deref().and_then(|s| s.character_id);
    let reconnect_phase = reconnect.as_deref().map(|s| s.phase);
    let reason = disconnected.reason.as_deref().unwrap_or("connection lost");
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
    match *state.get() {
        crate::game_state::GameState::CharSelect => {
            handle_disconnect_from_charselect(auth_token, reconnect, auth_feedback, commands);
        }
        crate::game_state::GameState::Login => handle_disconnect_from_login(auth_feedback),
        // All states where the player is in the world should reconnect transparently.
        crate::game_state::GameState::InWorld
        | crate::game_state::GameState::GameMenu
        | crate::game_state::GameState::Loading
        | crate::game_state::GameState::CampsitePopup => handle_disconnect_from_inworld(
            auth_token,
            selected,
            reconnect,
            selected_name,
            auth_feedback,
            next_state,
            commands,
        ),
        crate::game_state::GameState::Connecting => handle_disconnect_while_connecting(entity),
        _ => handle_disconnect_to_login_fallback(state.get(), entity, auth_feedback, next_state),
    }
}

fn handle_disconnect_from_charselect(
    auth_token: Option<Res<AuthToken>>,
    reconnect: Option<ResMut<ReconnectState>>,
    auth_feedback: &mut ResMut<AuthUiFeedback>,
    commands: &mut Commands,
) {
    handle_charselect_disconnect(auth_token, reconnect, auth_feedback, commands);
}

fn handle_disconnect_from_login(auth_feedback: &mut ResMut<AuthUiFeedback>) {
    info!("Disconnect handling: already in Login, surfacing connection-lost message");
    auth_feedback.0 = Some("Connection lost.".to_string());
}

fn handle_disconnect_from_inworld(
    auth_token: Option<Res<AuthToken>>,
    selected: Option<Res<SelectedCharacterId>>,
    reconnect: Option<ResMut<ReconnectState>>,
    selected_name: Option<&str>,
    auth_feedback: &mut ResMut<AuthUiFeedback>,
    next_state: &mut ResMut<NextState<crate::game_state::GameState>>,
    commands: &mut Commands,
) {
    handle_inworld_disconnect(
        auth_token,
        selected,
        reconnect,
        auth_feedback,
        next_state,
        commands,
        selected_name,
    );
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
    // Return to InWorld if currently in a sub-state (GameMenu, Loading, etc.)
    // so the reconnect overlay displays over the 3D scene.
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

pub(crate) fn net_position_to_bevy(pos: &NetPosition) -> Vec3 {
    Vec3::new(pos.x, pos.y, pos.z)
}

pub(crate) fn movement_to_direction(
    movement: &MovementState,
    facing: &CharacterFacing,
) -> [f32; 3] {
    crate::networking_player::movement_to_direction(movement, facing)
}

type SyncTransformFilter = (
    With<RemoteEntity>,
    Without<LocalPlayer>,
    Or<(Changed<NetPosition>, Changed<NetRotation>)>,
);
type SyncTransformQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static NetPosition,
        &'static mut InterpolationTarget,
        Option<&'static NetRotation>,
        Option<&'static mut RotationTarget>,
    ),
    SyncTransformFilter,
>;

/// When server sends a new position/rotation, update interpolation targets.
fn sync_replicated_transforms(mut query: SyncTransformQuery) {
    for (pos, mut interp, rotation, rot_target) in query.iter_mut() {
        interp.target = net_position_to_bevy(pos);
        if let (Some(rot), Some(mut target)) = (rotation, rot_target) {
            target.yaw = rot.y;
        }
    }
}

/// Smoothly lerp remote entity transforms toward their interpolation targets each frame.
fn interpolate_remote_entities(
    time: Res<Time>,
    mut query: Query<
        (
            &InterpolationTarget,
            Option<&RotationTarget>,
            &mut Transform,
        ),
        (With<RemoteEntity>, Without<LocalPlayer>),
    >,
) {
    let t = (INTERPOLATION_SPEED * time.delta_secs()).min(1.0);
    for (interp, rot_target, mut transform) in query.iter_mut() {
        transform.translation = transform.translation.lerp(interp.target, t);
        if let Some(rot) = rot_target {
            transform.rotation = transform.rotation.slerp(Quat::from_rotation_y(rot.yaw), t);
        }
    }
}

/// When a replicated entity loses its Replicated marker (remote disconnect), despawn it.
fn cleanup_disconnected_player(
    trigger: On<Remove, Replicated>,
    query: Query<Entity, With<ReplicatedVisualEntity>>,
    mut commands: Commands,
) {
    let entity = trigger.entity;
    if query.get(entity).is_ok() {
        info!("Remote entity disconnected, despawning {entity:?}");
        queue_despawn_if_exists(&mut commands, entity);
    }
}

fn queue_despawn_if_exists(commands: &mut Commands, entity: Entity) {
    commands.queue(move |world: &mut World| {
        if let Ok(entity_mut) = world.get_entity_mut(entity) {
            entity_mut.despawn();
        }
    });
}

pub(crate) fn gameplay_input_allowed(reconnect: Option<Res<ReconnectState>>) -> bool {
    reconnect.is_none_or(|reconnect| !reconnect.is_active())
}

fn spawn_reconnect_overlay(mut commands: Commands) {
    commands
        .spawn((
            ReconnectOverlayRoot,
            Visibility::Hidden,
            BackgroundColor(Color::srgba(0.03, 0.02, 0.01, 0.82)),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(0.0),
                top: Val::Percent(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                ReconnectOverlayText,
                Text::new("Reconnecting..."),
                TextFont {
                    font_size: 28.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.86, 0.45)),
            ));
        });
}

fn update_reconnect_overlay(
    reconnect: Res<ReconnectState>,
    mut overlay_q: Query<&mut Visibility, With<ReconnectOverlayRoot>>,
    mut text_q: Query<&mut Text, With<ReconnectOverlayText>>,
) {
    if !reconnect.is_changed() {
        return;
    }
    let visible = reconnect.is_active();
    let text = reconnect.overlay_text().to_string();
    for mut visibility in &mut overlay_q {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for mut overlay_text in &mut text_q {
        **overlay_text = text.clone();
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/networking_tests.rs"]
mod tests;
