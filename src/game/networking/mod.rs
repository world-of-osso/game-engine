mod disconnect;
mod reconnect;

pub(crate) use self::disconnect::handle_client_disconnected;
#[cfg(test)]
pub(crate) use self::reconnect::reset_network_world;
pub(crate) use self::reconnect::{
    advance_network_update_frame, drive_inworld_reconnect, finish_reconnect_when_world_ready,
    flush_pending_network_world_reset, network_world_reset_is_due, rand_client_id,
    request_network_world_reset,
};
use bevy::prelude::*;
use bevy::ui::{AlignItems, BackgroundColor, JustifyContent, Node, PositionType, Val};
use core::net::SocketAddr;
use game_engine::network_runtime::connection::Connected;
use lightyear::prelude::client::Remote;
use shared::components::{Position as NetPosition, Rotation as NetRotation};
pub use shared::protocol::ChatType;
use shared::protocol::{ChatMessage, EmoteIntent, ForcedDisconnect};

pub use crate::networking_auth::{
    AuthToken, AuthUiFeedback, CharacterList, LoginMode, LoginPassword, LoginUsername,
    SelectedCharacterId, load_auth_token,
};

use crate::camera::{CharacterFacing, MovementState};
use game_engine::status::{
    AchievementsStatusSnapshot, BarberShopStatusSnapshot, CalendarStatusSnapshot,
    CollectionStatusSnapshot, CombatLogStatusSnapshot, CurrenciesStatusSnapshot,
    DeathStatusSnapshot, FriendsStatusSnapshot, GroupStatusSnapshot, GuildStatusSnapshot,
    GuildVaultStatusSnapshot, IgnoreListStatusSnapshot, InventorySearchSnapshot, LfgStatusSnapshot,
    MapStatusSnapshot, ProfessionStatusSnapshot, PvpStatusSnapshot, QuestLogStatusSnapshot,
    ReputationsStatusSnapshot, WarbankStatusSnapshot, WhoStatusSnapshot,
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

/// Resource for other systems to queue outgoing social emotes.
#[derive(Resource, Default)]
pub struct EmoteInput(pub Option<EmoteIntent>);

/// Interpolation speed: 1 / interval between server ticks (~100ms at 20Hz).
const INTERPOLATION_SPEED: f32 = 10.0;

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

#[derive(Resource, Default, Clone)]
pub struct PendingForcedDisconnect(pub Option<ForcedDisconnect>);

#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PendingNetworkWorldReset(pub Option<u64>);

#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct NetworkUpdateFrame(pub u64);

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
        game_engine::network_runtime::connection::initialize_connection_bridge(app);
        app.add_plugins(game_engine::network_tick::NetworkTickPlugin);
        register_connection_tick_systems(app);
        register_net_resources(app);
        register_net_systems(app);
        register_net_observers(app);
        app.add_plugins(crate::scenes::inworld_tree::InWorldSceneTreePlugin);
    }
}

fn register_net_resources(app: &mut App) {
    register_zone_and_chat_resources(app);
    register_auth_resources(app);
    register_status_resources(app);
}

fn register_zone_and_chat_resources(app: &mut App) {
    app.init_resource::<CurrentZone>();
    app.init_resource::<LocalAliveState>();
    app.init_resource::<ChatLog>();
    app.init_resource::<ChatInput>();
    app.init_resource::<EmoteInput>();
    app.insert_resource(game_engine::chat_data::ChatState {
        max_messages: MAX_CHAT_LOG,
        ..Default::default()
    });
    app.insert_resource(game_engine::chat_data::WhisperState {
        max_recent: 10,
        ..Default::default()
    });
    app.init_resource::<ReconnectState>();
    app.init_resource::<PendingForcedDisconnect>();
    app.init_resource::<PendingNetworkWorldReset>();
    app.init_resource::<NetworkUpdateFrame>();
}

fn register_auth_resources(app: &mut App) {
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
}

fn register_status_resources(app: &mut App) {
    app.init_resource::<QuestLogStatusSnapshot>();
    app.init_resource::<GroupStatusSnapshot>();
    app.init_resource::<CombatLogStatusSnapshot>();
    app.init_resource::<AchievementsStatusSnapshot>();
    app.init_resource::<BarberShopStatusSnapshot>();
    app.init_resource::<DeathStatusSnapshot>();
    app.init_resource::<CollectionStatusSnapshot>();
    app.init_resource::<ProfessionStatusSnapshot>();
    app.init_resource::<CalendarStatusSnapshot>();
    app.init_resource::<FriendsStatusSnapshot>();
    app.init_resource::<GuildStatusSnapshot>();
    app.init_resource::<WhoStatusSnapshot>();
    app.init_resource::<IgnoreListStatusSnapshot>();
    app.init_resource::<PvpStatusSnapshot>();
    app.init_resource::<LfgStatusSnapshot>();
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
    register_network_lifecycle_systems(app);
}

fn register_network_lifecycle_systems(app: &mut App) {
    use game_engine::network_tick::{NetworkTick, NetworkTickSystems};

    app.add_systems(Update, update_reconnect_overlay);
    app.add_systems(
        NetworkTick,
        (
            flush_pending_network_world_reset.run_if(network_world_reset_is_due),
            drive_inworld_reconnect,
            finish_reconnect_when_world_ready,
        )
            .chain()
            .in_set(NetworkTickSystems::Apply),
    );
    app.add_systems(
        NetworkTick,
        advance_network_update_frame.after(NetworkTickSystems::Send),
    );
}

fn application_in_world(world: &World) -> bool {
    world
        .resource::<State<crate::game_state::GameState>>()
        .get()
        == &crate::game_state::GameState::InWorld
}

fn register_gameplay_net_systems(app: &mut App) {
    use crate::game_state::GameState;
    use crate::networking_messages as msg;
    use game_engine::network_events::{register_message_handler, register_outgoing_handler};
    use game_engine::network_tick::{NetworkTick, NetworkTickSystems};
    use shared::protocol::{EmoteEvent, GroupRosterSnapshot, QuestLogSnapshot};

    register_message_handler::<ChatMessage, _>(
        app,
        msg::receive_chat_messages,
        application_in_world,
    );
    register_message_handler::<EmoteEvent, _>(app, msg::receive_emote_events, application_in_world);
    register_message_handler::<QuestLogSnapshot, _>(
        app,
        msg::receive_quest_log_snapshot,
        application_in_world,
    );
    register_message_handler::<GroupRosterSnapshot, _>(
        app,
        msg::receive_group_roster_snapshot,
        application_in_world,
    );
    register_outgoing_handler(app, msg::send_chat_message, |world| {
        application_in_world(world) && world.resource::<ChatInput>().0.is_some()
    });
    register_outgoing_handler(app, msg::send_emote_intent, |world| {
        application_in_world(world) && world.resource::<EmoteInput>().0.is_some()
    });
    register_outgoing_handler(app, msg::send_player_input, application_in_world);
    register_outgoing_handler(app, msg::send_target_to_server, application_in_world);
    app.add_systems(
        NetworkTick,
        msg::track_player_zone
            .in_set(NetworkTickSystems::Apply)
            .run_if(in_state(GameState::InWorld)),
    );
    register_inworld_sync_systems(app);
}

fn register_inworld_sync_systems(app: &mut App) {
    game_engine::network_events::register_message_handler::<shared::protocol::LoadTerrain, _>(
        app,
        crate::networking_messages::receive_load_terrain,
        |world| {
            terrain_messages_allowed_in_state(
                *world
                    .resource::<State<crate::game_state::GameState>>()
                    .get(),
            ) && crate::game::inworld_scene_stage::effective_inworld_scene_stage(
                world
                    .get_resource::<crate::game::inworld_scene_stage::InWorldSceneStage>()
                    .copied(),
            )
            .includes(crate::game::inworld_scene_stage::InWorldSceneStage::Terrain)
        },
    );
    register_inworld_snapshot_systems(app);
    register_inworld_replication_systems(app);
    register_entity_tag_systems(app);
}

fn register_inworld_snapshot_systems(app: &mut App) {
    use crate::networking_messages as msg;
    use game_engine::network_events::register_message_handler;
    use shared::protocol::*;
    register_message_handler::<GroupCommandResponse, _>(
        app,
        msg::receive_group_command_response,
        application_in_world,
    );
    register_message_handler::<CombatLogSnapshot, _>(
        app,
        msg::receive_combat_log_snapshot,
        application_in_world,
    );
    register_message_handler::<CombatEvent, _>(
        app,
        msg::receive_combat_events,
        application_in_world,
    );
    register_message_handler::<AchievementStateUpdate, _>(
        app,
        msg::receive_achievement_state_update,
        application_in_world,
    );
    register_message_handler::<WorldMapStateUpdate, _>(
        app,
        msg::receive_world_map_state_update,
        application_in_world,
    );
    register_message_handler::<RestStateUpdate, _>(
        app,
        msg::receive_rest_state_update,
        application_in_world,
    );
    register_message_handler::<DurabilityStateUpdate, _>(
        app,
        msg::receive_durability_state_update,
        application_in_world,
    );
    register_message_handler::<ProfessionSnapshot, _>(
        app,
        msg::receive_profession_snapshot,
        application_in_world,
    );
    register_message_handler::<ReputationStateUpdate, _>(
        app,
        msg::receive_reputation_snapshot,
        application_in_world,
    );
}

fn register_inworld_replication_systems(app: &mut App) {
    use crate::game_state::GameState;
    crate::networking_player::register_player_appearance_events(app);
    app.add_systems(
        Update,
        sync_replicated_transforms.run_if(in_state(GameState::InWorld)),
    );
    app.add_systems(
        game_engine::network_tick::NetworkTick,
        crate::networking_player::sync_local_mount_visual_movement
            .in_set(game_engine::network_tick::NetworkTickSystems::Apply)
            .after(crate::networking_player::tag_local_player)
            .run_if(in_state(GameState::InWorld)),
    );
    app.add_systems(
        Update,
        interpolate_remote_entities
            .run_if(in_state(GameState::InWorld))
            .run_if(crate::game::inworld_scene_stage::inworld_scene_stage_allows_npcs),
    );
}

fn register_entity_tag_systems(app: &mut App) {
    use crate::game_state::GameState;
    app.add_systems(
        game_engine::network_tick::NetworkTick,
        (
            crate::networking_player::tag_local_player,
            crate::networking_player::sync_local_alive_state,
        )
            .chain()
            .in_set(game_engine::network_tick::NetworkTickSystems::Apply)
            .run_if(in_state(GameState::Loading).or_else(in_state(GameState::InWorld))),
    );
    app.add_systems(
        Update,
        crate::networking_auth::sync_selected_character_roster_entry
            .run_if(in_state(GameState::Loading).or_else(in_state(GameState::InWorld))),
    );
    crate::networking_npc::register_npc_visibility_policy_systems(app);
}

fn terrain_messages_allowed_in_state(state: crate::game_state::GameState) -> bool {
    matches!(
        state,
        crate::game_state::GameState::CharSelect
            | crate::game_state::GameState::Loading
            | crate::game_state::GameState::InWorld
    )
}

fn register_auth_net_systems(app: &mut App) {
    use crate::networking_auth as auth;
    use game_engine::network_events::register_message_handler;
    use shared::protocol::{
        CharacterListUpdate, CreateCharacterResponse, DeleteCharacterResponse, EnterWorldResponse,
        ForcedDisconnect, LoginResponse, RegisterResponse,
    };

    register_message_handler::<ForcedDisconnect, _>(app, auth::receive_forced_disconnect, |_| true);
    register_message_handler::<LoginResponse, _>(app, auth::receive_login_response, |_| true);
    register_message_handler::<CreateCharacterResponse, _>(
        app,
        auth::receive_create_character_response,
        |_| true,
    );
    register_message_handler::<DeleteCharacterResponse, _>(
        app,
        auth::receive_delete_character_response,
        |_| true,
    );
    register_message_handler::<CharacterListUpdate, _>(
        app,
        auth::receive_character_list_update,
        |_| true,
    );
    register_message_handler::<EnterWorldResponse, _>(
        app,
        auth::receive_enter_world_response,
        |_| true,
    );
    register_message_handler::<RegisterResponse, _>(app, auth::receive_register_response, |_| true);
}

fn register_net_observers(app: &mut App) {
    app.add_observer(on_connected);
    app.add_observer(handle_client_disconnected);
    app.add_observer(crate::networking_player::spawn_replicated_player);
    app.add_observer(crate::networking_npc::spawn_replicated_npc);
    app.add_observer(cleanup_disconnected_player);
}

pub(crate) fn register_connection_tick_systems(app: &mut App) {
    app.add_systems(
        game_engine::network_tick::NetworkTick,
        game_engine::network_runtime::connection::apply_connection_events
            .in_set(game_engine::network_tick::NetworkTickSystems::Receive)
            .after(game_engine::network_events::dispatch_incoming),
    );
}

fn connect_to_server(mut commands: Commands, server_addr: Res<ServerAddr>) {
    connect_to_server_inner(&mut commands, server_addr.0);
}

pub(crate) fn connect_to_server_inner(commands: &mut Commands, server_addr: SocketAddr) {
    let client_id = rand_client_id();
    commands.insert_resource(LocalClientId(client_id));
    commands.queue(move |world: &mut World| {
        let entity = game_engine::network_runtime::connection::start_connection(world, server_addr, client_id)
            .unwrap_or_else(|error| panic!("failed to start network connection: {error}"));
        info!("Connecting to server at {server_addr} with client_entity={entity:?} client_id={client_id}");
        crate::networking_auth::queue_auth_request(world);
    });
}

fn on_connected(
    trigger: On<Add, Connected>,
    auth_token: Res<AuthToken>,
    username: Res<LoginUsername>,
    password: Res<LoginPassword>,
    login_mode: Res<LoginMode>,
    reconnect: Option<ResMut<ReconnectState>>,
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
        let next_translation = transform.translation.lerp(interp.target, t);
        if next_translation != transform.translation {
            transform.translation = next_translation;
        }
        if let Some(rot) = rot_target {
            let target_rotation = Quat::from_rotation_y(rot.yaw);
            let next_rotation = transform.rotation.slerp(target_rotation, t);
            if next_rotation != transform.rotation {
                transform.rotation = next_rotation;
            }
        }
    }
}

/// When a replicated entity loses its Remote marker (remote disconnect), despawn it.
fn cleanup_disconnected_player(
    trigger: On<Remove, Remote>,
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
                    font_size: FontSize::Px(28.0),
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
#[path = "../../../tests/unit/networking_tests/mod.rs"]
mod tests;
