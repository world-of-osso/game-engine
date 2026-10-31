//! Private-server jump probe: client core vertical math and replicated server positions.
//! This does not exercise the Godot host's input, ground probing, or model animation.
use game_engine_core::player_physics_data::{
    GroundState, VerticalState, apply_gravity_and_ground_snap, update_grounded,
};
use game_engine_network::{BridgeConfig, Event, NetworkBridge, ProtocolMessage, replica::Replica};
use shared::{
    components::{MovementControl, Position},
    protocol::{
        AuthChannel, EnterWorldResponse, InputChannel, LoginRequest, LoginResponse, NewWorld,
        PlayerInput, SelectCharacter, TransferChannel, WorldPortAck,
    },
};
use std::{
    net::SocketAddr,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const STEP: Duration = Duration::from_micros(16_667);

struct Peer {
    bridge: NetworkBridge,
    replica: Replica,
    player: Option<u64>,
}

impl Peer {
    fn poll(&mut self) -> Result<(), String> {
        for event in self.bridge.drain_events()? {
            match event {
                Event::Message(message) => self.receive(message)?,
                Event::ReplicationStarted(schema) => self.replica = Replica::new(schema),
                Event::Replication(batch) => {
                    self.replica.apply(batch).map_err(|e| e.to_string())?
                }
                Event::Disconnected(reason) => return Err(format!("disconnected: {reason:?}")),
                Event::ProtocolRejected(reason) => return Err(reason),
                _ => {}
            }
        }
        Ok(())
    }

    fn receive(&mut self, message: ProtocolMessage) -> Result<(), String> {
        if message.is::<LoginResponse>() {
            let reply = message
                .downcast::<LoginResponse>()
                .map_err(|_| "login type")?;
            if !reply.success {
                return Err(format!("login: {:?}", reply.error));
            }
            let character = reply.characters.first().ok_or("empty roster")?;
            self.bridge.send::<_, AuthChannel>(SelectCharacter {
                character_id: character.character_id,
            })?;
        } else if message.is::<EnterWorldResponse>() {
            let reply = message
                .downcast::<EnterWorldResponse>()
                .map_err(|_| "entry type")?;
            if !reply.success {
                return Err(format!("entry: {:?}", reply.error));
            }
            self.player = reply.player_entity;
        } else if message.is::<NewWorld>() {
            self.bridge.send::<_, TransferChannel>(WorldPortAck)?;
        }
        Ok(())
    }

    fn state(&self) -> Option<(Position, u32)> {
        let unit = self.replica.unit(self.player?)?;
        Some((
            *unit.get::<Position>()?,
            unit.get::<MovementControl>()?.epoch,
        ))
    }

    fn send_height(&self, position: Position, epoch: u32, jumping: bool) -> Result<(), String> {
        self.bridge.send::<_, InputChannel>(PlayerInput {
            direction: [0.0; 3],
            facing_yaw: 0.0,
            jumping,
            running: true,
            swimming: false,
            flying: false,
            position: [position.x, position.y, position.z],
            epoch,
            transport: None,
        })
    }
}

fn connect() -> Result<Peer, String> {
    let endpoint: SocketAddr = std::env::var("JUMP_PROBE_SERVER")
        .map_err(|e| e.to_string())?
        .parse()
        .map_err(|e: std::net::AddrParseError| e.to_string())?;
    let account = std::env::var("JUMP_PROBE_ACCOUNT").map_err(|e| e.to_string())?;
    if !endpoint.ip().is_loopback() || endpoint.port() == 5000 || !account.starts_with("fb_") {
        return Err("private loopback endpoint and fb_ account required".into());
    }
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos() as u64;
    let bridge = BridgeConfig::new()
        .receive::<LoginResponse>()
        .receive::<EnterWorldResponse>()
        .receive::<NewWorld>()
        .connect(endpoint, id)?;
    bridge.send::<_, AuthChannel>(LoginRequest {
        token: None,
        username: account,
        password: "fbtest".into(),
    })?;
    Ok(Peer {
        bridge,
        replica: Replica::default(),
        player: None,
    })
}

fn run() -> Result<(), String> {
    let mut peer = connect()?;
    let deadline = Instant::now() + Duration::from_secs(120);
    while peer.state().is_none() && Instant::now() < deadline {
        peer.poll()?;
        thread::sleep(Duration::from_millis(10));
    }
    let settle_deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < settle_deadline {
        peer.poll()?;
        thread::sleep(Duration::from_millis(10));
    }
    let (mut position, epoch) = peer.state().ok_or("no replicated player")?;
    // The disposable character must be placed on a flat supported floor before this probe.
    let floor = position.y;
    let mut vertical = VerticalState {
        y: floor,
        vertical_velocity: shared::movement::JUMP_IMPULSE,
        grounded: false,
    };
    let started = Instant::now();
    let mut landing = None;
    let mut server_peak = floor;
    for frame in 0..120 {
        let frame_start = Instant::now();
        peer.poll()?;
        vertical.grounded = update_grounded(
            vertical.y,
            GroundState::Supported(floor),
            shared::movement::GROUND_SNAP_THRESHOLD,
            vertical.grounded,
        );
        vertical = apply_gravity_and_ground_snap(
            vertical,
            GroundState::Supported(floor),
            STEP.as_secs_f32(),
            shared::movement::GRAVITY,
            None,
        );
        position.y = vertical.y;
        if landing.is_none() && frame > 0 && vertical.grounded && vertical.vertical_velocity <= 0.0
        {
            landing = Some(started.elapsed());
        }
        peer.send_height(position, epoch, landing.is_none())?;
        let server = peer.state().map(|(position, _)| position.y);
        if let Some(height) = server {
            server_peak = server_peak.max(height);
        }
        println!(
            "frame={frame} elapsed_ms={} y={:.4} velocity={:.4} grounded={} server_y={server:?}",
            started.elapsed().as_millis(),
            vertical.y,
            vertical.vertical_velocity,
            vertical.grounded
        );
        if landing.is_some() && frame > 80 {
            break;
        }
        thread::sleep(STEP.saturating_sub(frame_start.elapsed()));
    }
    let elapsed = landing.ok_or("no landing")?;
    println!("landing_ms={}", elapsed.as_millis());
    if !(Duration::from_millis(500)..=Duration::from_millis(1000)).contains(&elapsed) {
        return Err(format!("unexpected ballistic landing time: {elapsed:?}"));
    }
    if server_peak < floor + 0.3 {
        return Err(format!(
            "server did not replicate the jump: floor={floor}, peak={server_peak}"
        ));
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("JUMP_PROBE_ERROR {error}");
        std::process::exit(1);
    }
}
