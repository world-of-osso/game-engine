use std::path::PathBuf;
use std::time::Duration;

use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use bevy::ui::{AlignItems, BackgroundColor, JustifyContent, Node, PositionType, Val};
use core::net::{IpAddr, Ipv4Addr, SocketAddr};
use lightyear::prelude::client::*;
use lightyear::prelude::*;
use shared::components::{
    EquipmentAppearance as NetEquipmentAppearance, Health as NetHealth, ModelDisplay, Npc,
    Player as NetPlayer, Position as NetPosition, Rotation as NetRotation,
};
use shared::protocol::ChatMessage;
pub use shared::protocol::ChatType;

pub use crate::networking_auth::{
    AuthToken, AuthUiFeedback, CharacterList, LoginMode, LoginPassword, LoginUsername,
    SelectedCharacterId, load_auth_token,
};

use crate::camera::{CharacterFacing, MoveDirection, MovementState, Player};
use crate::character_customization::CharacterCustomizationSelection;
use crate::character_models::{ensure_named_model_bundle, race_model_wow_path};
use crate::creature_display::CreatureDisplayMap;
use crate::equipment_appearance;
use crate::m2_effect_material::M2EffectMaterial;
use game_engine::asset::char_texture::CharTextureData;
use game_engine::customization_data::CustomizationDb;
use game_engine::outfit_data::OutfitData;
use game_engine::status::{
    CollectionStatusSnapshot, CombatLogStatusSnapshot, GroupStatusSnapshot, MapStatusSnapshot,
    ProfessionStatusSnapshot, QuestLogStatusSnapshot,
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
struct InterpolationTarget {
    target: Vec3,
}

#[derive(Component)]
pub(crate) struct ReplicatedVisualEntity;

#[derive(Component, Clone, Debug, PartialEq)]
pub(crate) struct ResolvedModelAssetInfo {
    pub model_path: String,
    pub skin_path: Option<String>,
    pub display_scale: Option<f32>,
}

#[derive(Component, Clone, Debug, PartialEq, Eq)]
struct AppliedPlayerAppearance {
    selection: CharacterCustomizationSelection,
    equipment: NetEquipmentAppearance,
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
        app.add_plugins(crate::inworld_scene_tree::InWorldSceneTreePlugin);
    }
}

fn register_net_resources(app: &mut App) {
    app.init_resource::<CurrentZone>();
    app.init_resource::<LocalAliveState>();
    app.init_resource::<ChatLog>();
    app.init_resource::<ChatInput>();
    app.init_resource::<ReconnectState>();
    app.insert_resource(AuthToken(load_auth_token()));
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
            drive_inworld_reconnect,
            update_reconnect_overlay,
            finish_reconnect_when_world_ready,
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
            sync_map_status_snapshot,
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
            msg::receive_collection_snapshot,
            msg::receive_profession_snapshot,
            sync_replicated_transforms,
            sync_replicated_player_customization,
            interpolate_remote_entities,
        )
            .run_if(in_state(GameState::InWorld)),
    );
    app.add_systems(
        Update,
        (
            tag_local_player,
            sync_local_alive_state,
            apply_npc_visibility_policy,
        )
            .chain()
            .run_if(in_state(GameState::InWorld)),
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
            auth::receive_enter_world_response,
            auth::receive_register_response,
        ),
    );
}

fn sync_map_status_snapshot(
    mut snapshot: ResMut<MapStatusSnapshot>,
    current_zone: Res<CurrentZone>,
    player_query: Query<&Transform, With<Player>>,
) {
    snapshot.zone_id = current_zone.zone_id;
    if let Some(transform) = player_query.iter().next() {
        snapshot.player_x = transform.translation.x;
        snapshot.player_z = transform.translation.z;
    }
}

fn register_net_observers(app: &mut App) {
    app.add_observer(on_connected);
    app.add_observer(on_link_established);
    app.add_observer(handle_client_disconnected);
    app.add_observer(spawn_replicated_player);
    app.add_observer(spawn_replicated_npc);
    app.add_observer(cleanup_disconnected_player);
}

fn connect_to_server(mut commands: Commands, server_addr: Res<ServerAddr>) {
    connect_to_server_inner(&mut commands, server_addr.0);
}

fn connect_to_server_inner(commands: &mut Commands, server_addr: SocketAddr) {
    let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), CLIENT_PORT);
    let client_id = rand_client_id();
    commands.insert_resource(LocalClientId(client_id));
    let auth = Authentication::Manual {
        server_addr,
        client_id,
        private_key: [0; 32], // matches server default
        protocol_id: 0,       // matches server default
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
    let selected_name = selected
        .as_deref()
        .and_then(|selected| selected.character_name.as_deref());
    let selected_id = selected
        .as_deref()
        .and_then(|selected| selected.character_id);
    let reconnect_phase = reconnect.as_deref().map(|state| state.phase);
    let reason = disconnected.reason.as_deref().unwrap_or("connection lost");
    warn!(
        "Client entity {:?} disconnected in {:?}: {reason}; token={} selected_id={selected_id:?} selected_name={selected_name:?} reconnect_phase={reconnect_phase:?}",
        trigger.entity,
        state.get(),
        auth_token_label,
    );

    match *state.get() {
        crate::game_state::GameState::CharSelect => {
            info!("Disconnect handling: preserving CharSelect and surfacing offline message");
            auth_feedback.0 = Some("Connection lost. Char select is now offline.".to_string());
        }
        crate::game_state::GameState::Login => {
            info!("Disconnect handling: already in Login, surfacing connection-lost message");
            auth_feedback.0 = Some("Connection lost.".to_string());
        }
        crate::game_state::GameState::InWorld => {
            if auth_token
                .as_deref()
                .and_then(|token| token.0.as_deref())
                .is_none_or(|token| token.trim().is_empty())
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
            if let Some(name) = selected
                .as_deref()
                .and_then(|selected| selected.character_name.clone())
            {
                commands.insert_resource(crate::char_select::PreselectedCharName(name));
            }
            commands.insert_resource(crate::char_select::AutoEnterWorld);
            commands.insert_resource(LoginMode::Login);
            commands.insert_resource(LoginUsername(String::new()));
            commands.insert_resource(LoginPassword(String::new()));
            commands.queue(reset_network_world);
            reconnect.phase = ReconnectPhase::PendingConnect;
            reconnect.terrain_refresh_seen = false;
            auth_feedback.0 = None;
            info!(
                "Disconnect handling: queued reconnect with phase {:?}, preselected_name={selected_name:?}",
                reconnect.phase
            );
        }
        crate::game_state::GameState::Connecting => {
            info!(
                "Disconnect handling: ignoring transient disconnect while still connecting for client entity {:?}",
                trigger.entity
            );
        }
        crate::game_state::GameState::CharCreate
        | crate::game_state::GameState::CampsitePopup
        | crate::game_state::GameState::TrashButton
        | crate::game_state::GameState::Loading
        | crate::game_state::GameState::Reconnecting => {
            warn!(
                "Disconnect handling: transitioning from {:?} to Login due to disconnect on client entity {:?}",
                state.get(),
                trigger.entity
            );
            auth_feedback.0 = Some("Connection lost.".to_string());
            next_state.set(crate::game_state::GameState::Login);
        }
    }
}

/// Convert local MovementState + CharacterFacing into a world-space direction vector.
/// Returns direction in Bevy coordinates (X-right, Y-up, Z-back).
pub(crate) fn movement_to_direction(
    movement: &MovementState,
    facing: &CharacterFacing,
) -> [f32; 3] {
    let forward = [facing.yaw.sin(), 0.0, facing.yaw.cos()];
    let right = [-forward[2], 0.0, forward[0]];

    let mut dir = [0.0f32; 3];
    match movement.direction {
        MoveDirection::Forward => {
            dir[0] += forward[0];
            dir[2] += forward[2];
        }
        MoveDirection::Backward => {
            dir[0] -= forward[0];
            dir[2] -= forward[2];
        }
        MoveDirection::Left => {
            dir[0] -= right[0];
            dir[2] -= right[2];
        }
        MoveDirection::Right => {
            dir[0] += right[0];
            dir[2] += right[2];
        }
        MoveDirection::None => {}
    }
    dir
}

/// When the server replicates a new player, spawn a visible capsule mesh.
/// If this is our own player, attach camera components so the WowCamera follows it.
fn spawn_replicated_player(
    trigger: On<Add, NetPlayer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut effect_materials: ResMut<Assets<M2EffectMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut inv_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    creature_display_map: Res<CreatureDisplayMap>,
    query: Query<(&NetPosition, &NetPlayer, Option<&NetRotation>), With<Replicated>>,
    selected: Option<Res<SelectedCharacterId>>,
) {
    let entity = trigger.entity;
    let Ok((pos, player, rotation)) = query.get(entity) else {
        return;
    };
    let is_local = is_local_player_entity(&player.name, selected.as_deref());
    info!(
        "Spawning replicated player '{}' (local={is_local}) at ({:.1}, {:.1}, {:.1})",
        player.name, pos.x, pos.y, pos.z
    );
    let position = net_position_to_bevy(pos);
    let yaw = rotation.map_or(std::f32::consts::PI, |r| r.y);
    commands.entity(entity).insert((
        Transform::from_translation(position).with_rotation(Quat::from_rotation_y(yaw)),
        Visibility::default(),
        ReplicatedVisualEntity,
        RemoteEntity,
        InterpolationTarget { target: position },
        RotationTarget { yaw },
    ));
    let model_spawned = if let Some(model_path) = resolve_player_model_path(player) {
        let model_spawned = crate::m2_scene::spawn_full_m2_on_entity(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut effect_materials,
            &mut images,
            &mut inv_bp,
            &model_path,
            &creature_display_map,
            entity,
        );
        if model_spawned {
            commands.entity(entity).insert(ResolvedModelAssetInfo {
                model_path: model_path.display().to_string(),
                skin_path: crate::asset::m2::ensure_primary_skin_path(&model_path)
                    .map(|path| path.display().to_string()),
                display_scale: None,
            });
        }
        model_spawned
    } else {
        false
    };
    if !model_spawned {
        let (capsule, material) = build_player_capsule(&mut meshes, &mut materials, is_local);
        commands
            .entity(entity)
            .insert((Mesh3d(capsule), MeshMaterial3d(material)));
    }
}

fn build_player_capsule(
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    is_local: bool,
) -> (Handle<Mesh>, Handle<StandardMaterial>) {
    let capsule = meshes.add(Capsule3d::new(0.4, 1.6));
    let color = if is_local {
        Color::srgb(0.2, 1.0, 0.3)
    } else {
        Color::srgb(0.2, 0.6, 1.0)
    };
    let material = materials.add(StandardMaterial {
        base_color: color,
        ..default()
    });
    (capsule, material)
}

fn net_position_to_bevy(pos: &NetPosition) -> Vec3 {
    // Server already stores positions in Bevy space — no conversion needed.
    Vec3::new(pos.x, pos.y, pos.z)
}

fn net_player_customization_selection(player: &NetPlayer) -> CharacterCustomizationSelection {
    CharacterCustomizationSelection {
        race: player.race,
        class: player.class,
        sex: player.appearance.sex,
        appearance: player.appearance,
    }
}

fn resolve_player_model_path(player: &NetPlayer) -> Option<PathBuf> {
    race_model_wow_path(player.race, player.appearance.sex).and_then(ensure_named_model_bundle)
}

#[allow(clippy::too_many_arguments)]
fn sync_replicated_player_customization(
    mut commands: Commands,
    customization_db: Res<CustomizationDb>,
    char_tex: Res<CharTextureData>,
    outfit_data: Res<OutfitData>,
    player_query: Query<
        (
            Entity,
            &NetPlayer,
            Option<&NetEquipmentAppearance>,
            Option<&AppliedPlayerAppearance>,
            Option<&Children>,
        ),
        With<ReplicatedVisualEntity>,
    >,
    parent_query: Query<&ChildOf>,
    geoset_query: Query<(Entity, &crate::m2_spawn::GeosetMesh, &ChildOf)>,
    mut visibility_query: Query<&mut Visibility>,
    material_query: Query<(
        Entity,
        &MeshMaterial3d<StandardMaterial>,
        Option<&crate::m2_spawn::BatchTextureType>,
        &ChildOf,
    )>,
    mut equipment_query: Query<&mut crate::equipment::Equipment>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, player, equipment_appearance, applied, children) in &player_query {
        let selection = net_player_customization_selection(player);
        let equipment_snapshot = equipment_appearance.cloned().unwrap_or_default();
        if applied.is_some_and(|applied| {
            applied.selection == selection && applied.equipment == equipment_snapshot
        }) {
            continue;
        }
        if children.is_none_or(|children| children.is_empty()) {
            continue;
        }
        let resolved_equipment =
            equipment_appearance::resolve_equipment_appearance(&equipment_snapshot, &outfit_data);
        crate::character_customization::apply_character_customization(
            selection,
            &customization_db,
            &char_tex,
            &outfit_data,
            Some(&resolved_equipment),
            entity,
            &mut images,
            &mut materials,
            &parent_query,
            &geoset_query,
            &mut visibility_query,
            &material_query,
        );
        if let Ok(mut equipment) = equipment_query.get_mut(entity) {
            equipment_appearance::apply_runtime_equipment(&mut equipment, &resolved_equipment);
        }
        commands.entity(entity).insert(AppliedPlayerAppearance {
            selection,
            equipment: equipment_snapshot,
        });
    }
}

/// Check if a replicated player is our local character by matching name.
fn is_local_player_entity(player_name: &str, selected: Option<&SelectedCharacterId>) -> bool {
    let Some(sel) = selected else { return false };
    let Some(ref name) = sel.character_name else {
        return false;
    };
    name == player_name
}

fn choose_local_player_entity<'a>(
    selected_name: &str,
    players: impl Iterator<Item = (Entity, &'a NetPlayer)>,
) -> (Option<Entity>, usize) {
    let mut matches = Vec::new();
    for (entity, player) in players {
        if player.name == selected_name {
            matches.push(entity);
        }
    }
    matches.sort_by_key(|entity| entity.to_bits());
    (matches.last().copied(), matches.len())
}

/// Retroactively tag the local player when SelectedCharacterId arrives after replication.
#[allow(clippy::type_complexity)]
fn tag_local_player(
    mut commands: Commands,
    selected: Option<Res<SelectedCharacterId>>,
    players: Query<(Entity, &NetPlayer, Has<LocalPlayer>), With<Replicated>>,
) {
    let Some(sel) = selected else { return };
    let Some(ref name) = sel.character_name else {
        return;
    };
    if players.iter().any(|(_, p, local)| local && p.name == *name) {
        return;
    }
    let (chosen, match_count) =
        choose_local_player_entity(name, players.iter().map(|(e, p, _)| (e, p)));
    if match_count > 1 {
        warn!(
            "Found {match_count} replicated players named '{}'; choosing newest entity as local",
            name
        );
    }
    for (entity, player, is_local) in players.iter() {
        apply_local_player_tag(&mut commands, entity, player, is_local, chosen, name);
    }
}

fn apply_local_player_tag(
    commands: &mut Commands,
    entity: Entity,
    player: &NetPlayer,
    is_local: bool,
    chosen: Option<Entity>,
    name: &str,
) {
    let should_be_local = Some(entity) == chosen && player.name == name;
    if should_be_local && !is_local {
        info!("Tagging local player '{}' on entity {:?}", name, entity);
        commands.entity(entity).insert((
            LocalPlayer,
            Player,
            MovementState::default(),
            CharacterFacing::default(),
            crate::collision::CharacterPhysics::default(),
        ));
    } else if !should_be_local && is_local {
        commands.entity(entity).remove::<(
            LocalPlayer,
            Player,
            MovementState,
            CharacterFacing,
            crate::collision::CharacterPhysics,
        )>();
    }
}

fn sync_local_alive_state(
    mut local_alive: ResMut<LocalAliveState>,
    local_player_query: Query<&NetHealth, With<LocalPlayer>>,
) {
    local_alive.0 = local_player_query
        .iter()
        .next()
        .map(|health| health.current > 0.0)
        .unwrap_or(true);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NpcVisibilityPolicy {
    Always,
    Hidden,
    DeadOnly,
}

fn npc_visibility_policy(template_id: u32) -> NpcVisibilityPolicy {
    match template_id {
        6491 => NpcVisibilityPolicy::DeadOnly, // Spirit Healer
        32820 => NpcVisibilityPolicy::Hidden,  // Wild Turkey clutter near spawn
        26724 | 26738 | 26739 | 26740..=26745 | 26747..=26759 | 26765 | 33252 => {
            NpcVisibilityPolicy::Hidden // [DND] TAR pedestals and other debug vendors
        }
        _ => NpcVisibilityPolicy::Always,
    }
}

fn apply_npc_visibility_policy(
    local_alive: Res<LocalAliveState>,
    mut npcs: Query<(&Npc, &mut Visibility), With<Replicated>>,
) {
    for (npc, mut visibility) in &mut npcs {
        let should_show = match npc_visibility_policy(npc.template_id) {
            NpcVisibilityPolicy::Always => true,
            NpcVisibilityPolicy::Hidden => false,
            NpcVisibilityPolicy::DeadOnly => !local_alive.0,
        };
        *visibility = if should_show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

type NpcReplicatedQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static NetPosition,
        &'static Npc,
        Option<&'static NetRotation>,
        Option<&'static ModelDisplay>,
    ),
    With<Replicated>,
>;

#[derive(bevy::ecs::system::SystemParam)]
struct NpcSpawnAssets<'w> {
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    effect_materials: ResMut<'w, Assets<M2EffectMaterial>>,
    images: ResMut<'w, Assets<Image>>,
    inv_bp: ResMut<'w, Assets<SkinnedMeshInverseBindposes>>,
}

/// When the server replicates a new NPC, try to load its M2 model; fall back to capsule.
fn spawn_replicated_npc(
    trigger: On<Add, Npc>,
    mut commands: Commands,
    mut npc_assets: NpcSpawnAssets,
    query: NpcReplicatedQuery,
    display_map: Option<Res<CreatureDisplayMap>>,
) {
    let entity = trigger.entity;
    let Ok((pos, npc, rotation, model_display)) = query.get(entity) else {
        return;
    };
    insert_npc_transform(&mut commands, entity, pos, rotation);
    let display_scale = npc_display_scale(model_display, display_map.as_deref());
    let visual_root = spawn_npc_visual_root(&mut commands, entity, display_scale);
    let mut assets = crate::m2_spawn::SpawnAssets {
        meshes: &mut npc_assets.meshes,
        materials: &mut npc_assets.materials,
        effect_materials: &mut npc_assets.effect_materials,
        images: &mut npc_assets.images,
        inverse_bindposes: &mut npc_assets.inv_bp,
    };
    let m2_loaded = try_spawn_npc_model(
        &mut commands,
        &mut assets,
        visual_root,
        entity,
        model_display,
        display_map.as_deref(),
        display_scale,
    );
    if !m2_loaded {
        spawn_npc_capsule(
            &mut commands,
            &mut npc_assets.meshes,
            &mut npc_assets.materials,
            visual_root,
        );
    }
    debug!(
        "Spawned NPC template_id={} m2={m2_loaded} at ({:.0}, {:.0}, {:.0})",
        npc.template_id, pos.x, pos.y, pos.z
    );
}

fn spawn_npc_visual_root(commands: &mut Commands, entity: Entity, scale: f32) -> Entity {
    let visual_root = commands
        .spawn((
            Name::new("NpcVisualRoot"),
            Transform::from_scale(Vec3::splat(scale.max(0.01))),
            Visibility::default(),
        ))
        .id();
    commands.entity(entity).add_child(visual_root);
    visual_root
}

fn insert_npc_transform(
    commands: &mut Commands,
    entity: Entity,
    pos: &NetPosition,
    rotation: Option<&NetRotation>,
) {
    let position = net_position_to_bevy(pos);
    let yaw = rotation.map_or(0.0, |r| r.y);
    let transform = Transform::from_translation(position).with_rotation(Quat::from_rotation_y(yaw));
    commands.entity(entity).insert((
        transform,
        Visibility::default(),
        RemoteEntity,
        InterpolationTarget { target: position },
        RotationTarget { yaw },
    ));
}

fn npc_display_scale(
    model_display: Option<&ModelDisplay>,
    display_map: Option<&CreatureDisplayMap>,
) -> f32 {
    let display_id = model_display.map(|md| md.display_id).unwrap_or(0);
    display_map
        .and_then(|dm| dm.get_scale(display_id))
        .filter(|scale| *scale > 0.0)
        .unwrap_or(1.0)
}

/// Try to resolve display_id → FDID → M2 file and attach meshes. Returns true on success.
fn try_spawn_npc_model(
    commands: &mut Commands,
    assets: &mut crate::m2_spawn::SpawnAssets<'_>,
    visual_root: Entity,
    entity: Entity,
    model_display: Option<&ModelDisplay>,
    display_map: Option<&CreatureDisplayMap>,
    display_scale: f32,
) -> bool {
    let display_id = model_display.map(|md| md.display_id).unwrap_or(0);
    if display_id == 0 {
        return false;
    }
    let fdid = display_map.and_then(|dm| dm.get_fdid(display_id));
    let Some(fdid) = fdid else { return false };
    let skin_fdids = display_map
        .and_then(|dm| dm.get_skin_fdids(display_id))
        .unwrap_or([0, 0, 0]);
    let Some(m2_path) = crate::asset::casc_resolver::ensure_model(fdid) else {
        return false;
    };
    commands.entity(entity).insert(ResolvedModelAssetInfo {
        model_path: m2_path.display().to_string(),
        skin_path: crate::asset::m2::ensure_primary_skin_path(&m2_path)
            .map(|path| path.display().to_string()),
        display_scale: Some(display_scale),
    });
    crate::m2_spawn::spawn_m2_on_entity(commands, assets, &m2_path, visual_root, &skin_fdids)
}

/// Attach a capsule mesh as fallback for NPCs without M2 models.
fn spawn_npc_capsule(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    entity: Entity,
) {
    let capsule = meshes.add(Capsule3d::new(0.3, 1.2));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.3, 0.2),
        ..default()
    });
    commands
        .entity(entity)
        .insert((Mesh3d(capsule), MeshMaterial3d(material)));
}

/// Target rotation for smooth interpolation of remote entities.
#[derive(Component)]
struct RotationTarget {
    yaw: f32,
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
            let target_rot = Quat::from_rotation_y(rot.yaw);
            transform.rotation = transform.rotation.slerp(target_rot, t);
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

fn drive_inworld_reconnect(
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
    connect_to_server_inner(&mut commands, server_addr.0);
}

fn finish_reconnect_when_world_ready(
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
    let client_entities: Vec<_> = world
        .query_filtered::<Entity, With<Client>>()
        .iter(world)
        .collect();
    for entity in client_entities {
        if let Ok(entity_mut) = world.get_entity_mut(entity) {
            entity_mut.despawn();
        }
    }

    let replicated_entities: Vec<_> = world
        .query_filtered::<Entity, With<Replicated>>()
        .iter(world)
        .collect();
    for entity in replicated_entities {
        if let Ok(entity_mut) = world.get_entity_mut(entity) {
            entity_mut.despawn();
        }
    }

    let local_only_entities: Vec<_> = world
        .query_filtered::<Entity, (With<LocalPlayer>, Without<Replicated>)>()
        .iter(world)
        .collect();
    for entity in local_only_entities {
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

    if let Some(mut current_target) =
        world.get_resource_mut::<game_engine::targeting::CurrentTarget>()
    {
        current_target.0 = None;
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

    reset_resource::<game_engine::status::NetworkStatusSnapshot>(world);
    reset_resource::<game_engine::status::CharacterStatsSnapshot>(world);
    reset_resource::<game_engine::status::CollectionStatusSnapshot>(world);
    reset_resource::<game_engine::status::CombatLogStatusSnapshot>(world);
    reset_resource::<game_engine::status::GroupStatusSnapshot>(world);
    reset_resource::<game_engine::status::MapStatusSnapshot>(world);
    reset_resource::<game_engine::status::ProfessionStatusSnapshot>(world);
    reset_resource::<game_engine::status::QuestLogStatusSnapshot>(world);
}

fn reset_resource<T: Resource + Default>(world: &mut World) {
    if let Some(mut resource) = world.get_resource_mut::<T>() {
        *resource = T::default();
    }
}

fn rand_client_id() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

#[cfg(test)]
#[path = "networking_tests.rs"]
mod tests;
