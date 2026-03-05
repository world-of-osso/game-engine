use bevy::prelude::*;
use core::net::{IpAddr, Ipv4Addr, SocketAddr};
use lightyear::prelude::*;
use lightyear::prelude::client::*;
use shared::components::{Player as NetPlayer, Position as NetPosition};
use shared::protocol::{InputChannel, PlayerInput};
use std::time::Duration;

use crate::camera::{CharacterFacing, MoveDirection, MovementState, Player};

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
        app.add_systems(Update, log_replicated_positions);
        app.add_observer(on_connected);
        app.add_observer(on_link_established);
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

/// Log positions of replicated entities for diagnostics.
fn log_replicated_positions(
    query: Query<(&NetPosition, &NetPlayer), (With<Replicated>, Changed<NetPosition>)>,
) {
    for (pos, player) in query.iter() {
        debug!(
            "Server position for '{}': ({:.1}, {:.1}, {:.1})",
            player.name, pos.x, pos.y, pos.z
        );
    }
}

fn rand_client_id() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}
