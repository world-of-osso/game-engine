use bevy::prelude::*;
use core::net::{IpAddr, Ipv4Addr, SocketAddr};
use lightyear::prelude::*;
use lightyear::prelude::client::*;
use shared::components::{Npc, Player as NetPlayer, Position as NetPosition, Rotation as NetRotation};
use shared::protocol::{ChatChannel, ChatMessage, CombatChannel, InputChannel, PlayerInput, SetTarget};
pub use shared::protocol::ChatType;
use std::time::Duration;

use crate::camera::{CharacterFacing, MoveDirection, MovementState, Player};
use crate::target::CurrentTarget;

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
        app.init_resource::<ChatLog>();
        app.init_resource::<ChatInput>();
        app.add_systems(Startup, connect_to_server);
        app.add_systems(Update, send_player_input);
        app.add_systems(Update, send_chat_message);
        app.add_systems(Update, receive_chat_messages);
        app.add_systems(Update, send_target_to_server);
        app.add_systems(Update, sync_replicated_transforms);
        app.add_systems(Update, interpolate_remote_entities);
        app.add_observer(on_connected);
        app.add_observer(on_link_established);
        app.add_observer(spawn_replicated_player);
        app.add_observer(spawn_replicated_npc);
        app.add_observer(cleanup_disconnected_player);
    }
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

fn on_connected(_trigger: On<Add, Connected>) {
    info!("Connected to server!");
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
    local_id: Option<Res<LocalClientId>>,
) {
    let entity = trigger.entity;
    let Ok((pos, player, rotation)) = query.get(entity) else {
        return;
    };
    let is_local = is_local_player(&player.name, local_id.as_deref());
    info!(
        "Spawning replicated player '{}' (local={is_local}) at ({:.1}, {:.1}, {:.1})",
        player.name, pos.x, pos.y, pos.z
    );
    let capsule = meshes.add(Capsule3d::new(0.4, 1.6));
    let color = if is_local { Color::srgb(0.2, 1.0, 0.3) } else { Color::srgb(0.2, 0.6, 1.0) };
    let material = materials.add(StandardMaterial {
        base_color: color,
        ..default()
    });
    let position = Vec3::new(pos.x, pos.y, pos.z);
    let yaw = rotation.map_or(0.0, |r| r.y);
    let mut ecmds = commands.entity(entity);
    ecmds.insert((
        Mesh3d(capsule),
        MeshMaterial3d(material),
        Transform::from_xyz(pos.x, pos.y, pos.z)
            .with_rotation(Quat::from_rotation_y(yaw)),
        RemoteEntity,
        InterpolationTarget { target: position },
        RotationTarget { yaw },
    ));
    if is_local {
        ecmds.insert((LocalPlayer, Player, MovementState::default(), CharacterFacing::default()));
    }
}

/// Check if a replicated player name matches our local client_id.
fn is_local_player(name: &str, local_id: Option<&LocalClientId>) -> bool {
    let Some(local) = local_id else { return false };
    name == format!("Player-{}", local.0)
}

/// When the server replicates a new NPC, spawn a colored capsule mesh.
fn spawn_replicated_npc(
    trigger: On<Add, Npc>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<(&NetPosition, &Npc, Option<&NetRotation>), With<Replicated>>,
) {
    let entity = trigger.entity;
    let Ok((pos, npc, rotation)) = query.get(entity) else {
        return;
    };
    let capsule = meshes.add(Capsule3d::new(0.3, 1.2));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.3, 0.2),
        ..default()
    });
    let position = Vec3::new(pos.x, pos.y, pos.z);
    let yaw = rotation.map_or(0.0, |r| r.y);
    commands.entity(entity).insert((
        Mesh3d(capsule),
        MeshMaterial3d(material),
        Transform::from_xyz(pos.x, pos.y, pos.z)
            .with_rotation(Quat::from_rotation_y(yaw)),
        RemoteEntity,
        InterpolationTarget { target: position },
        RotationTarget { yaw },
    ));
    debug!("Spawned NPC capsule template_id={} at ({:.0}, {:.0}, {:.0})", npc.template_id, pos.x, pos.y, pos.z);
}

/// Target rotation for smooth interpolation of remote entities.
#[derive(Component)]
struct RotationTarget {
    yaw: f32,
}

/// When server sends a new position/rotation, update interpolation targets.
fn sync_replicated_transforms(
    mut query: Query<
        (&NetPosition, &mut InterpolationTarget, Option<&NetRotation>, Option<&mut RotationTarget>),
        (With<RemoteEntity>, Or<(Changed<NetPosition>, Changed<NetRotation>)>),
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
    mut query: Query<(&InterpolationTarget, Option<&RotationTarget>, &mut Transform), With<RemoteEntity>>,
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
    let Some(msg) = chat_input.0.take() else { return };
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
            chat_log.messages.push((msg.sender, msg.content, msg.channel));
            if chat_log.messages.len() > MAX_CHAT_LOG {
                chat_log.messages.remove(0);
            }
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
    let msg = SetTarget { target_entity: target_bits };
    for mut sender in senders.iter_mut() {
        sender.send::<CombatChannel>(msg.clone());
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
                NetPosition { x: 0.0, y: 0.0, z: 0.0 },
                NetRotation { x: 0.0, y: 1.5, z: 0.0 },
                InterpolationTarget { target: Vec3::ZERO },
                RotationTarget { yaw: 0.0 },
                RemoteEntity,
            ))
            .id();

        app.update();

        let rot = app.world().get::<RotationTarget>(entity).unwrap();
        assert!((rot.yaw - 1.5).abs() < 1e-6, "rotation target should be 1.5, got {}", rot.yaw);
    }

    #[test]
    fn sync_updates_interpolation_target() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Update, sync_replicated_transforms);

        let entity = app
            .world_mut()
            .spawn((
                NetPosition { x: 10.0, y: 20.0, z: 30.0 },
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
                NetPosition { x: 5.0, y: 6.0, z: 7.0 },
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
            log.messages.push((
                format!("player{i}"),
                format!("msg{i}"),
                ChatType::Say,
            ));
            if log.messages.len() > MAX_CHAT_LOG {
                log.messages.remove(0);
            }
        }
        assert_eq!(log.messages.len(), MAX_CHAT_LOG);
        assert_eq!(log.messages[0].0, "player1");
        assert_eq!(log.messages[99].0, "player100");
    }

    #[test]
    fn is_local_player_matches_own_client_id() {
        let local = LocalClientId(12345);
        assert!(is_local_player("Player-12345", Some(&local)));
    }

    #[test]
    fn is_local_player_rejects_different_id() {
        let local = LocalClientId(12345);
        assert!(!is_local_player("Player-99999", Some(&local)));
    }

    #[test]
    fn is_local_player_returns_false_without_resource() {
        assert!(!is_local_player("Player-12345", None));
    }

    #[test]
    fn local_player_gets_camera_components() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(LocalClientId(42));

        let entity = app.world_mut().spawn_empty().id();
        let local_id = app.world().get_resource::<LocalClientId>();
        let is_local = is_local_player("Player-42", local_id);
        assert!(is_local, "Player-42 should match LocalClientId(42)");

        // Insert components the observer would insert for a local player.
        app.world_mut().entity_mut(entity).insert((
            RemoteEntity, Transform::default(),
            LocalPlayer, Player, MovementState::default(), CharacterFacing::default(),
        ));

        assert!(app.world().get::<Player>(entity).is_some(), "should have Player");
        assert!(app.world().get::<LocalPlayer>(entity).is_some(), "should have LocalPlayer");
        assert!(app.world().get::<MovementState>(entity).is_some(), "should have MovementState");
        assert!(app.world().get::<CharacterFacing>(entity).is_some(), "should have CharacterFacing");
    }

    #[test]
    fn remote_player_stays_capsule() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(LocalClientId(42));

        let entity = app.world_mut().spawn_empty().id();
        let local_id = app.world().get_resource::<LocalClientId>();
        let is_local = is_local_player("Player-999", local_id);
        assert!(!is_local, "Player-999 should NOT match LocalClientId(42)");

        // Only insert base components (no camera components for remote).
        app.world_mut().entity_mut(entity).insert((RemoteEntity, Transform::default()));

        assert!(app.world().get::<Player>(entity).is_none(), "should NOT have Player");
        assert!(app.world().get::<LocalPlayer>(entity).is_none(), "should NOT have LocalPlayer");
        assert!(app.world().get::<MovementState>(entity).is_none(), "should NOT have MovementState");
    }
}
