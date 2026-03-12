use std::time::Duration;

use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use core::net::{IpAddr, Ipv4Addr, SocketAddr};
use lightyear::prelude::client::*;
use lightyear::prelude::*;
use shared::components::{
    ModelDisplay, Npc, Player as NetPlayer, Position as NetPosition, Rotation as NetRotation,
};
use shared::protocol::ChatMessage;
pub use shared::protocol::ChatType;

pub use crate::networking_auth::{
    AuthToken, AuthUiFeedback, CharacterList, LoginMode, LoginPassword, LoginUsername,
    SelectedCharacterId, load_auth_token,
};

use crate::camera::{CharacterFacing, MoveDirection, MovementState, Player};
use crate::creature_display::CreatureDisplayMap;
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

/// Maximum number of messages stored in the chat log.
pub(crate) const MAX_CHAT_LOG: usize = 100;

/// Tracks the zone the local player is currently in (replicated from server).
#[derive(Resource, Default)]
pub struct CurrentZone {
    pub zone_id: u32,
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
    app.init_resource::<ChatLog>();
    app.init_resource::<ChatInput>();
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
    register_gameplay_net_systems(app);
    register_auth_net_systems(app);
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
        (
            msg::receive_group_command_response,
            msg::receive_combat_log_snapshot,
            msg::receive_combat_events,
            msg::receive_collection_snapshot,
            msg::receive_profession_snapshot,
            msg::receive_load_terrain,
            sync_replicated_transforms,
            interpolate_remote_entities,
            tag_local_player,
        )
            .run_if(in_state(GameState::InWorld)),
    );
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
    app.add_observer(spawn_replicated_player);
    app.add_observer(spawn_replicated_npc);
    app.add_observer(cleanup_disconnected_player);
}

fn connect_to_server(mut commands: Commands, server_addr: Res<ServerAddr>) {
    let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), CLIENT_PORT);
    let client_id = rand_client_id();
    commands.insert_resource(LocalClientId(client_id));
    let auth = Authentication::Manual {
        server_addr: server_addr.0,
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
            PeerAddr(server_addr.0),
            UdpIo::default(),
            netcode,
        ))
        .id();
    commands.trigger(Connect { entity });
    info!("Connecting to server at {}...", server_addr.0);
}

fn on_link_established(trigger: On<Add, Connected>, mut commands: Commands) {
    commands
        .entity(trigger.entity)
        .insert(ReplicationReceiver::default());
}

fn on_connected(
    _trigger: On<Add, Connected>,
    auth_token: Res<AuthToken>,
    username: Res<LoginUsername>,
    password: Res<LoginPassword>,
    login_mode: Res<LoginMode>,
    mut login_senders: Query<&mut MessageSender<shared::protocol::LoginRequest>>,
    mut register_senders: Query<&mut MessageSender<shared::protocol::RegisterRequest>>,
) {
    info!("Connected to server!");
    crate::networking_auth::send_auth_request(
        &auth_token,
        &username,
        &password,
        &login_mode,
        &mut login_senders,
        &mut register_senders,
    );
}

/// Convert local MovementState + CharacterFacing into a world-space direction vector.
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
    let position = Vec3::new(pos.x, pos.y, pos.z);
    let yaw = rotation.map_or(std::f32::consts::PI, |r| r.y);
    let (capsule, material) = build_player_capsule(&mut meshes, &mut materials, is_local);
    let mut ecmds = commands.entity(entity);
    ecmds.insert((
        Mesh3d(capsule),
        MeshMaterial3d(material),
        Transform::from_translation(position).with_rotation(Quat::from_rotation_y(yaw)),
        RemoteEntity,
        InterpolationTarget { target: position },
        RotationTarget { yaw },
    ));
    if is_local {
        ecmds.insert((
            LocalPlayer,
            Player,
            MovementState::default(),
            CharacterFacing::default(),
            crate::collision::CharacterPhysics::default(),
        ));
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

/// Check if a replicated player is our local character by matching name.
fn is_local_player_entity(player_name: &str, selected: Option<&SelectedCharacterId>) -> bool {
    let Some(sel) = selected else { return false };
    let Some(ref name) = sel.character_name else { return false };
    name == player_name
}

/// Retroactively tag the local player when SelectedCharacterId arrives after replication.
fn tag_local_player(
    mut commands: Commands,
    selected: Option<Res<SelectedCharacterId>>,
    players: Query<(Entity, &NetPlayer), (With<Replicated>, Without<LocalPlayer>)>,
) {
    let Some(sel) = selected else { return };
    let Some(ref name) = sel.character_name else { return };
    for (entity, player) in players.iter() {
        if player.name == *name {
            info!("Tagging local player '{}'", name);
            commands.entity(entity).insert((
                LocalPlayer,
                Player,
                MovementState::default(),
                CharacterFacing::default(),
                crate::collision::CharacterPhysics::default(),
            ));
        }
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
    let mut assets = crate::m2_spawn::SpawnAssets {
        meshes: &mut npc_assets.meshes,
        materials: &mut npc_assets.materials,
        images: &mut npc_assets.images,
        inverse_bindposes: &mut npc_assets.inv_bp,
    };
    let m2_loaded = try_spawn_npc_model(
        &mut commands,
        &mut assets,
        entity,
        model_display,
        display_map.as_deref(),
    );
    if !m2_loaded {
        spawn_npc_capsule(
            &mut commands,
            &mut npc_assets.meshes,
            &mut npc_assets.materials,
            entity,
        );
    }
    debug!(
        "Spawned NPC template_id={} m2={m2_loaded} at ({:.0}, {:.0}, {:.0})",
        npc.template_id, pos.x, pos.y, pos.z
    );
}

fn insert_npc_transform(
    commands: &mut Commands,
    entity: Entity,
    pos: &NetPosition,
    rotation: Option<&NetRotation>,
) {
    let position = Vec3::new(pos.x, pos.y, pos.z);
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

/// Try to resolve display_id → FDID → M2 file and attach meshes. Returns true on success.
fn try_spawn_npc_model(
    commands: &mut Commands,
    assets: &mut crate::m2_spawn::SpawnAssets<'_>,
    entity: Entity,
    model_display: Option<&ModelDisplay>,
    display_map: Option<&CreatureDisplayMap>,
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
    crate::m2_spawn::spawn_m2_on_entity(commands, assets, &m2_path, entity, &skin_fdids)
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
        interp.target = Vec3::new(pos.x, pos.y, pos.z);
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
        With<RemoteEntity>,
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
    query: Query<Entity, With<RemoteEntity>>,
    mut commands: Commands,
) {
    let entity = trigger.entity;
    if query.get(entity).is_ok() {
        info!("Remote entity disconnected, despawning {entity:?}");
        commands.entity(entity).despawn();
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
