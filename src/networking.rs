use bevy::prelude::*;
use core::net::{IpAddr, Ipv4Addr, SocketAddr};
use lightyear::prelude::*;
use lightyear::prelude::client::*;
use shared::components::{Npc, Player as NetPlayer, Position as NetPosition};
use shared::protocol::{InputChannel, PlayerInput};
use std::time::Duration;

use crate::camera::{CharacterFacing, MoveDirection, MovementState, Player};

/// Marker for entities spawned from server replication.
#[derive(Component)]
struct RemoteEntity;

/// Target position for smooth interpolation of remote entities.
#[derive(Component)]
struct InterpolationTarget {
    target: Vec3,
}

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
        app.add_systems(Startup, connect_to_server);
        app.add_systems(Update, send_player_input);
        app.add_systems(Update, sync_replicated_transforms);
        app.add_systems(Update, interpolate_remote_entities);
        app.add_observer(on_connected);
        app.add_observer(on_link_established);
        app.add_observer(spawn_replicated_player);
        app.add_observer(spawn_replicated_npc);
    }
}

fn connect_to_server(mut commands: Commands, server_addr: Res<ServerAddr>) {
    let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), CLIENT_PORT);
    let auth = Authentication::Manual {
        server_addr: server_addr.0,
        client_id: rand_client_id(),
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
fn spawn_replicated_player(
    trigger: On<Add, NetPlayer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<(&NetPosition, &NetPlayer), With<Replicated>>,
) {
    let entity = trigger.entity;
    let Ok((pos, player)) = query.get(entity) else {
        return;
    };
    info!("Spawning replicated player '{}' at ({:.1}, {:.1}, {:.1})", player.name, pos.x, pos.y, pos.z);
    let capsule = meshes.add(Capsule3d::new(0.4, 1.6));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.6, 1.0),
        ..default()
    });
    let position = Vec3::new(pos.x, pos.y, pos.z);
    commands.entity(entity).insert((
        Mesh3d(capsule),
        MeshMaterial3d(material),
        Transform::from_xyz(pos.x, pos.y, pos.z),
        RemoteEntity,
        InterpolationTarget { target: position },
    ));
}

/// When the server replicates a new NPC, spawn a colored capsule mesh.
fn spawn_replicated_npc(
    trigger: On<Add, Npc>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<(&NetPosition, &Npc), With<Replicated>>,
) {
    let entity = trigger.entity;
    let Ok((pos, npc)) = query.get(entity) else {
        return;
    };
    let capsule = meshes.add(Capsule3d::new(0.3, 1.2));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.3, 0.2),
        ..default()
    });
    let position = Vec3::new(pos.x, pos.y, pos.z);
    commands.entity(entity).insert((
        Mesh3d(capsule),
        MeshMaterial3d(material),
        Transform::from_xyz(pos.x, pos.y, pos.z),
        RemoteEntity,
        InterpolationTarget { target: position },
    ));
    debug!("Spawned NPC capsule template_id={} at ({:.0}, {:.0}, {:.0})", npc.template_id, pos.x, pos.y, pos.z);
}

/// When server sends a new position, update the interpolation target (not the transform directly).
fn sync_replicated_transforms(
    mut query: Query<
        (&NetPosition, &mut InterpolationTarget),
        (With<RemoteEntity>, Changed<NetPosition>),
    >,
) {
    for (pos, mut interp) in query.iter_mut() {
        interp.target = Vec3::new(pos.x, pos.y, pos.z);
    }
}

/// Smoothly lerp remote entity transforms toward their interpolation targets each frame.
fn interpolate_remote_entities(
    time: Res<Time>,
    mut query: Query<(&InterpolationTarget, &mut Transform), With<RemoteEntity>>,
) {
    let t = (INTERPOLATION_SPEED * time.delta_secs()).min(1.0);
    for (interp, mut transform) in query.iter_mut() {
        transform.translation = transform.translation.lerp(interp.target, t);
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
}
