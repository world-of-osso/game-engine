//! Three real NetworkBridge peers for private release-server tick measurements.
//! Accounts/positions are prepared through the private server admin socket.
use game_engine_network::{BridgeConfig, Event, NetworkBridge, ProtocolMessage};
use shared::protocol::*;
use std::{
    net::SocketAddr,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Peer {
    name: String,
    bridge: NetworkBridge,
    entered: bool,
}

impl Peer {
    fn receive(&mut self, message: ProtocolMessage) -> Result<(), String> {
        if message.is::<LoginResponse>() {
            let reply = message
                .downcast::<LoginResponse>()
                .map_err(|_| "login type")?;
            if !reply.success {
                return Err(format!("{} login: {:?}", self.name, reply.error));
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
                return Err(format!("{} entry: {:?}", self.name, reply.error));
            }
            self.entered = true;
            println!("ENTERED {}", self.name);
        } else if message.is::<NewWorld>() {
            self.bridge.send::<_, TransferChannel>(WorldPortAck)?;
        }
        Ok(())
    }

    fn poll(&mut self) -> Result<(), String> {
        for event in self.bridge.drain_events()? {
            match event {
                Event::Message(message) => self.receive(message)?,
                Event::Disconnected(reason) => {
                    return Err(format!("{} disconnected: {reason:?}", self.name));
                }
                Event::ProtocolRejected(reason) => return Err(reason),
                _ => {}
            }
        }
        Ok(())
    }
}

fn connect_peers(address: SocketAddr) -> Result<Vec<Peer>, String> {
    let clock = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos() as u64;
    (0..3)
        .map(|index| {
            let name = format!("fb_server_tick_perf{index}");
            let bridge = BridgeConfig::new()
                .receive::<LoginResponse>()
                .receive::<EnterWorldResponse>()
                .receive::<NewWorld>()
                .connect(address, clock + index)?;
            bridge.send::<_, AuthChannel>(LoginRequest {
                token: None,
                username: name.clone(),
                password: "fbtest".into(),
            })?;
            Ok(Peer {
                name,
                bridge,
                entered: false,
            })
        })
        .collect()
}

fn run() -> Result<(), String> {
    let address: SocketAddr = std::env::var("TICK_PERF_ENDPOINT")
        .map_err(|e| e.to_string())?
        .parse()
        .map_err(|e: std::net::AddrParseError| e.to_string())?;
    if !address.ip().is_loopback() || address.port() == 5000 {
        return Err("private loopback endpoint required; UDP5000 prohibited".into());
    }
    let mut peers = connect_peers(address)?;
    let started = Instant::now();
    let entry_deadline = Duration::from_secs(60);
    let measurement_duration = Duration::from_secs(180);
    while started.elapsed() < measurement_duration {
        for peer in &mut peers {
            peer.poll()?;
        }
        if started.elapsed() > entry_deadline && peers.iter().any(|peer| !peer.entered) {
            return Err("peer world-entry deadline exceeded".into());
        }
        thread::sleep(Duration::from_millis(25));
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("TICK_PERF_PEER_ERROR {error}");
        std::process::exit(1);
    }
}
