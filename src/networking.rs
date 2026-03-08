use std::path::Path;
use std::time::Duration;

use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use core::net::{IpAddr, Ipv4Addr, SocketAddr};
use game_engine::targeting::CurrentTarget;
use lightyear::prelude::client::*;
use lightyear::prelude::*;
use shared::components::{
    ModelDisplay, Npc, Player as NetPlayer, Position as NetPosition, Rotation as NetRotation, Zone,
};
pub use shared::protocol::ChatType;
use shared::protocol::{
    ChatChannel, ChatMessage, CollectionSnapshot, CombatChannel, CombatEvent, CombatEventType,
    CombatLogEventKindSnapshot, CombatLogSnapshot, GroupCommandResponse, GroupRoleSnapshot,
    GroupRosterSnapshot, InputChannel, LoadTerrain, PlayerInput, ProfessionSnapshot,
    QuestLogSnapshot, QuestRepeatability as QuestRepeatabilitySnapshot, SetTarget,
};

pub use crate::networking_auth::{
    AuthToken, AuthUiFeedback, CharacterList, LoginMode, LoginPassword, LoginUsername,
    SelectedCharacterId, load_auth_token,
};

use crate::camera::{CharacterFacing, MoveDirection, MovementState, Player};
use crate::creature_display::CreatureDisplayMap;
use crate::terrain::AdtManager;
use game_engine::status::{
    CollectionMountEntry, CollectionPetEntry, CollectionStatusSnapshot, CombatLogEntry,
    CombatLogEventKind, CombatLogStatusSnapshot, GroupMemberEntry, GroupRole, GroupStatusSnapshot,
    MapStatusSnapshot, ProfessionRecipeEntry, ProfessionStatusSnapshot, QuestEntry,
    QuestLogStatusSnapshot, QuestObjectiveEntry, QuestRepeatability,
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
const MAX_CHAT_LOG: usize = 100;

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
const MAX_COMBAT_LOG: usize = 200;

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
    app.add_systems(
        Update,
        (
            send_player_input,
            send_chat_message,
            receive_chat_messages,
            send_target_to_server,
            track_player_zone,
            sync_map_status_snapshot,
            receive_quest_log_snapshot,
            receive_group_roster_snapshot,
        )
            .run_if(in_state(GameState::InWorld)),
    );
    app.add_systems(
        Update,
        (
            receive_group_command_response,
            receive_combat_log_snapshot,
            receive_combat_events,
            receive_collection_snapshot,
            receive_profession_snapshot,
            receive_load_terrain,
            sync_replicated_transforms,
            interpolate_remote_entities,
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
    let netcode = match NetcodeClient::new(auth, NetcodeConfig::default()) {
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

fn on_link_established(trigger: On<Add, LinkOf>, mut commands: Commands) {
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

/// Send movement input to the server every frame.
fn send_player_input(
    player_q: Query<(&MovementState, &CharacterFacing), With<Player>>,
    mut senders: Query<&mut MessageSender<PlayerInput>>,
) {
    let Ok((movement, facing)) = player_q.single() else {
        return;
    };

    let direction = movement_to_direction(movement, facing);
    if direction == [0.0, 0.0, 0.0] && !movement.jumping {
        return; // don't spam idle inputs
    }

    let input = PlayerInput {
        direction,
        facing_yaw: facing.yaw,
        jumping: movement.jumping,
        running: movement.running,
    };

    for mut sender in senders.iter_mut() {
        sender.send::<InputChannel>(input.clone());
    }
}

/// Convert local MovementState + CharacterFacing into a world-space direction vector.
fn movement_to_direction(movement: &MovementState, facing: &CharacterFacing) -> [f32; 3] {
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
    let is_local = is_local_player_entity(entity, selected.as_deref());
    info!(
        "Spawning replicated player '{}' (local={is_local}) at ({:.1}, {:.1}, {:.1})",
        player.name, pos.x, pos.y, pos.z
    );
    let position = Vec3::new(pos.x, pos.y, pos.z);
    let yaw = rotation.map_or(0.0, |r| r.y);
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

/// Check if a replicated entity matches our selected character's player_entity bits.
fn is_local_player_entity(entity: Entity, selected: Option<&SelectedCharacterId>) -> bool {
    let Some(sel) = selected else { return false };
    let Some(bits) = sel.0 else { return false };
    entity.to_bits() == bits
}

/// When the server replicates a new NPC, try to load its M2 model; fall back to capsule.
fn spawn_replicated_npc(
    trigger: On<Add, Npc>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut inv_bp: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    query: Query<
        (
            &NetPosition,
            &Npc,
            Option<&NetRotation>,
            Option<&ModelDisplay>,
        ),
        With<Replicated>,
    >,
    display_map: Option<Res<CreatureDisplayMap>>,
) {
    let entity = trigger.entity;
    let Ok((pos, npc, rotation, model_display)) = query.get(entity) else {
        return;
    };
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
    let m2_loaded = try_spawn_npc_model(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut images,
        &mut inv_bp,
        entity,
        model_display,
        display_map.as_deref(),
    );
    if !m2_loaded {
        spawn_npc_capsule(&mut commands, &mut meshes, &mut materials, entity);
    }
    debug!(
        "Spawned NPC template_id={} m2={m2_loaded} at ({:.0}, {:.0}, {:.0})",
        npc.template_id, pos.x, pos.y, pos.z
    );
}

/// Try to resolve display_id → FDID → M2 file and attach meshes. Returns true on success.
fn try_spawn_npc_model(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    inv_bp: &mut Assets<SkinnedMeshInverseBindposes>,
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
    let m2_path_str = format!("data/models/{fdid}.m2");
    let m2_path = Path::new(&m2_path_str);
    if !m2_path.exists() {
        return false;
    }
    crate::m2_spawn::spawn_m2_on_entity(
        commands,
        meshes,
        materials,
        images,
        inv_bp,
        m2_path,
        entity,
        &skin_fdids,
    )
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

/// When server sends a new position/rotation, update interpolation targets.
fn sync_replicated_transforms(
    mut query: Query<
        (
            &NetPosition,
            &mut InterpolationTarget,
            Option<&NetRotation>,
            Option<&mut RotationTarget>,
        ),
        (
            With<RemoteEntity>,
            Or<(Changed<NetPosition>, Changed<NetRotation>)>,
        ),
    >,
) {
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

/// Send a queued chat message to the server.
fn send_chat_message(
    mut chat_input: ResMut<ChatInput>,
    mut senders: Query<&mut MessageSender<ChatMessage>>,
) {
    let Some(msg) = chat_input.0.take() else {
        return;
    };
    for mut sender in senders.iter_mut() {
        sender.send::<ChatChannel>(msg.clone());
    }
}

/// Receive chat messages from the server and append to the chat log.
fn receive_chat_messages(
    mut receivers: Query<&mut MessageReceiver<ChatMessage>>,
    mut chat_log: ResMut<ChatLog>,
) {
    for mut receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            chat_log
                .messages
                .push((msg.sender, msg.content, msg.channel));
            if chat_log.messages.len() > MAX_CHAT_LOG {
                chat_log.messages.remove(0);
            }
        }
    }
}

/// Receive LoadTerrain messages from the server and initialize/stream the AdtManager.
fn receive_load_terrain(
    mut receivers: Query<&mut MessageReceiver<LoadTerrain>>,
    mut adt_manager: ResMut<AdtManager>,
) {
    for mut receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            let key = (msg.initial_tile_y, msg.initial_tile_x);
            if adt_manager.map_name.is_empty() {
                info!(
                    "Server requested terrain: {} tile ({}, {})",
                    msg.map_name, msg.initial_tile_y, msg.initial_tile_x
                );
                adt_manager.map_name = msg.map_name;
                adt_manager.initial_tile = key;
            } else if adt_manager.loaded.contains_key(&key)
                || adt_manager.pending.contains(&key)
                || adt_manager.failed.contains(&key)
            {
                continue;
            } else {
                debug!(
                    "Server requested additional tile ({}, {})",
                    msg.initial_tile_y, msg.initial_tile_x
                );
                adt_manager.server_requested.insert(key);
            }
        }
    }
}

fn receive_quest_log_snapshot(
    mut receivers: Query<&mut MessageReceiver<QuestLogSnapshot>>,
    mut snapshot: ResMut<QuestLogStatusSnapshot>,
) {
    for mut receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            snapshot.entries = msg.entries.into_iter().map(map_quest_entry).collect();
            snapshot.watched_quest_ids = msg.watched_quest_ids;
        }
    }
}

fn map_quest_entry(entry: shared::protocol::QuestEntrySnapshot) -> QuestEntry {
    QuestEntry {
        quest_id: entry.quest_id,
        title: entry.title,
        zone: entry.zone,
        completed: entry.completed,
        repeatability: map_repeatability(entry.repeatability),
        objectives: entry
            .objectives
            .into_iter()
            .map(|obj| QuestObjectiveEntry {
                text: obj.text,
                current: obj.current,
                required: obj.required,
                completed: obj.completed,
            })
            .collect(),
    }
}

fn map_repeatability(value: QuestRepeatabilitySnapshot) -> QuestRepeatability {
    match value {
        QuestRepeatabilitySnapshot::Normal => QuestRepeatability::Normal,
        QuestRepeatabilitySnapshot::Daily => QuestRepeatability::Daily,
        QuestRepeatabilitySnapshot::Weekly => QuestRepeatability::Weekly,
    }
}

fn receive_group_roster_snapshot(
    mut receivers: Query<&mut MessageReceiver<GroupRosterSnapshot>>,
    mut snapshot: ResMut<GroupStatusSnapshot>,
) {
    for mut receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            snapshot.is_raid = msg.is_raid;
            snapshot.ready_count = msg.ready_count;
            snapshot.total_count = msg.total_count;
            snapshot.members = msg.members.into_iter().map(map_group_member).collect();
        }
    }
}

fn map_group_member(member: shared::protocol::GroupMemberSnapshot) -> GroupMemberEntry {
    GroupMemberEntry {
        name: member.name,
        role: match member.role {
            GroupRoleSnapshot::Tank => GroupRole::Tank,
            GroupRoleSnapshot::Healer => GroupRole::Healer,
            GroupRoleSnapshot::Damage => GroupRole::Damage,
            GroupRoleSnapshot::None => GroupRole::None,
        },
        is_leader: member.is_leader,
        online: member.online,
        subgroup: member.subgroup,
    }
}

fn receive_group_command_response(
    mut receivers: Query<&mut MessageReceiver<GroupCommandResponse>>,
    mut snapshot: ResMut<GroupStatusSnapshot>,
) {
    for mut receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            snapshot.last_server_message = Some(msg.message);
        }
    }
}

fn receive_combat_log_snapshot(
    mut receivers: Query<&mut MessageReceiver<CombatLogSnapshot>>,
    mut snapshot: ResMut<CombatLogStatusSnapshot>,
) {
    for mut receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            snapshot.entries = msg.entries.into_iter().map(map_combat_entry).collect();
            if snapshot.entries.len() > MAX_COMBAT_LOG {
                let start = snapshot.entries.len() - MAX_COMBAT_LOG;
                snapshot.entries = snapshot.entries.split_off(start);
            }
        }
    }
}

fn map_combat_entry(entry: shared::protocol::CombatLogEntrySnapshot) -> CombatLogEntry {
    CombatLogEntry {
        kind: match entry.kind {
            CombatLogEventKindSnapshot::Damage => CombatLogEventKind::Damage,
            CombatLogEventKindSnapshot::Heal => CombatLogEventKind::Heal,
            CombatLogEventKindSnapshot::Interrupt => CombatLogEventKind::Interrupt,
            CombatLogEventKindSnapshot::AuraApplied => CombatLogEventKind::AuraApplied,
            CombatLogEventKindSnapshot::Death => CombatLogEventKind::Death,
        },
        source: entry.source,
        target: entry.target,
        spell: entry.spell,
        amount: entry.amount,
        aura: entry.aura,
        text: entry.text,
    }
}

fn receive_combat_events(
    mut receivers: Query<&mut MessageReceiver<CombatEvent>>,
    mut snapshot: ResMut<CombatLogStatusSnapshot>,
) {
    for mut receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            let (kind, amount, text) = match msg.event_type {
                CombatEventType::MeleeDamage => (
                    CombatLogEventKind::Damage,
                    Some(msg.damage.round() as i32),
                    format!(
                        "{} hit {} for {}",
                        msg.attacker,
                        msg.target,
                        msg.damage.round() as i32
                    ),
                ),
                CombatEventType::Death => (
                    CombatLogEventKind::Death,
                    None,
                    format!("{} died", msg.target),
                ),
                CombatEventType::Respawn => (
                    CombatLogEventKind::AuraApplied,
                    None,
                    format!("{} respawned", msg.target),
                ),
            };
            append_combat_entry(
                &mut snapshot,
                CombatLogEntry {
                    kind,
                    source: msg.attacker.to_string(),
                    target: msg.target.to_string(),
                    spell: None,
                    amount,
                    aura: None,
                    text,
                },
            );
        }
    }
}

fn append_combat_entry(snapshot: &mut CombatLogStatusSnapshot, entry: CombatLogEntry) {
    snapshot.entries.push(entry);
    if snapshot.entries.len() > MAX_COMBAT_LOG {
        let overflow = snapshot.entries.len() - MAX_COMBAT_LOG;
        snapshot.entries.drain(0..overflow);
    }
}

fn receive_collection_snapshot(
    mut receivers: Query<&mut MessageReceiver<CollectionSnapshot>>,
    mut snapshot: ResMut<CollectionStatusSnapshot>,
) {
    for mut receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            snapshot.mounts = msg
                .mounts
                .into_iter()
                .map(|mount| CollectionMountEntry {
                    mount_id: mount.mount_id,
                    name: mount.name,
                    known: mount.known,
                })
                .collect();
            snapshot.pets = msg
                .pets
                .into_iter()
                .map(|pet| CollectionPetEntry {
                    pet_id: pet.pet_id,
                    name: pet.name,
                    known: pet.known,
                })
                .collect();
        }
    }
}

fn receive_profession_snapshot(
    mut receivers: Query<&mut MessageReceiver<ProfessionSnapshot>>,
    mut snapshot: ResMut<ProfessionStatusSnapshot>,
) {
    for mut receiver in receivers.iter_mut() {
        for msg in receiver.receive() {
            snapshot.recipes = msg
                .recipes
                .into_iter()
                .map(|recipe| ProfessionRecipeEntry {
                    spell_id: recipe.spell_id,
                    profession: recipe.profession,
                    name: recipe.name,
                    craftable: recipe.craftable,
                    cooldown: recipe.cooldown,
                })
                .collect();
        }
    }
}

/// When CurrentTarget changes, send a SetTarget message to the server.
fn send_target_to_server(
    current: Res<CurrentTarget>,
    mut senders: Query<&mut MessageSender<SetTarget>>,
) {
    if !current.is_changed() {
        return;
    }
    let target_bits = current.0.map(|e| e.to_bits());
    let msg = SetTarget {
        target_entity: target_bits,
    };
    for mut sender in senders.iter_mut() {
        sender.send::<CombatChannel>(msg.clone());
    }
}

/// Watch for Zone component changes on the local player and update the CurrentZone resource.
fn track_player_zone(
    player_q: Query<&Zone, (With<Player>, Changed<Zone>)>,
    mut current_zone: ResMut<CurrentZone>,
) {
    if let Ok(zone) = player_q.single() {
        if current_zone.zone_id != zone.id {
            info!("Entered zone {}", zone.id);
            current_zone.zone_id = zone.id;
        }
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
mod tests {
    use super::*;
    use std::f32::consts::{FRAC_PI_2, PI};

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
        let dir =
            movement_to_direction(&make_state(MoveDirection::Forward), &make_facing(FRAC_PI_2));
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

    #[test]
    fn sync_updates_rotation_target() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Update, sync_replicated_transforms);

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
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Update, sync_replicated_transforms);

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
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Update, sync_replicated_transforms);

        let entity = app
            .world_mut()
            .spawn((
                NetPosition {
                    x: 5.0,
                    y: 6.0,
                    z: 7.0,
                },
                InterpolationTarget { target: Vec3::ZERO },
                // no RemoteEntity marker
            ))
            .id();

        app.update();

        let interp = app.world().get::<InterpolationTarget>(entity).unwrap();
        assert_eq!(interp.target, Vec3::ZERO);
    }

    #[test]
    fn interpolation_moves_toward_target() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Update, interpolate_remote_entities);

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

    fn test_entity() -> Entity {
        let mut world = World::new();
        world.spawn_empty().id()
    }

    #[test]
    fn is_local_player_entity_matches_selected() {
        let entity = test_entity();
        let selected = SelectedCharacterId(Some(entity.to_bits()));
        assert!(is_local_player_entity(entity, Some(&selected)));
    }

    #[test]
    fn is_local_player_entity_rejects_different() {
        let mut world = World::new();
        let entity = world.spawn_empty().id();
        let other = world.spawn_empty().id();
        let selected = SelectedCharacterId(Some(other.to_bits()));
        assert!(!is_local_player_entity(entity, Some(&selected)));
    }

    #[test]
    fn is_local_player_entity_none_without_resource() {
        let entity = test_entity();
        assert!(!is_local_player_entity(entity, None));
    }

    #[test]
    fn is_local_player_entity_none_when_not_selected() {
        let entity = test_entity();
        let selected = SelectedCharacterId(None);
        assert!(!is_local_player_entity(entity, Some(&selected)));
    }
}
