//! Nine lightweight real-UDP account peers alongside one native Godot client.
//! No simulated match state, synthetic roster or server shortcuts.
use game_engine_network::{BridgeConfig, Event, NetworkBridge, ProtocolMessage};
use shared::protocol::*;
use std::{
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
struct Peer {
    index: usize,
    bridge: NetworkBridge,
    entered: bool,
    map: Option<u32>,
    snapshot: Option<PvpSnapshot>,
}
impl Peer {
    fn receive(&mut self, message: ProtocolMessage) -> Result<(), String> {
        if message.is::<LoginResponse>() {
            let reply = message
                .downcast::<LoginResponse>()
                .map_err(|_| "login type")?;
            if !reply.success {
                return Err(format!("fb_bg{} login: {:?}", self.index, reply.error));
            }
            let character = reply.characters.first().ok_or("Empty roster")?;
            self.bridge.send::<_, AuthChannel>(SelectCharacter {
                character_id: character.character_id,
            })?;
        } else if message.is::<EnterWorldResponse>() {
            let reply = message
                .downcast::<EnterWorldResponse>()
                .map_err(|_| "entry type")?;
            if !reply.success {
                return Err(format!("world entry: {:?}", reply.error));
            }
            self.entered = true;
            self.bridge.send::<_, PvpChannel>(QueryPvpStatus)?;
        } else if message.is::<NewWorld>() {
            let destination = message
                .downcast::<NewWorld>()
                .map_err(|_| "transfer type")?;
            self.map = Some(destination.map_id);
            self.bridge.send::<_, TerrainChannel>(WorldPortAck)?;
            println!("fb_bg{} WorldPortAck map{}", self.index, destination.map_id);
        } else if message.is::<PvpStateUpdate>() {
            let update = message
                .downcast::<PvpStateUpdate>()
                .map_err(|_| "pvp type")?;
            if let Some(error) = update.error {
                return Err(format!("fb_bg{} PvP: {error}", self.index));
            }
            if let Some(snapshot) = update.snapshot {
                self.snapshot = Some(snapshot);
            }
        }
        Ok(())
    }
    fn action(&self, action: &str) -> Result<(), String> {
        if !self.entered {
            return Err(format!("fb_bg{} has not entered", self.index));
        }
        match action {
            "queue" => self.bridge.send::<_, PvpChannel>(QueueForBattleground {
                battleground_id: 1014,
            }),
            "accept" | "decline" => {
                let match_id = self
                    .snapshot
                    .as_ref()
                    .and_then(|s| s.battleground.as_ref())
                    .ok_or("No authoritative invitation")?
                    .match_id;
                self.bridge
                    .send::<_, PvpChannel>(RespondToBattlegroundInvite {
                        match_id,
                        accepted: action == "accept",
                    })
            }
            "leave" => {
                let match_id = self
                    .snapshot
                    .as_ref()
                    .and_then(|s| s.battleground.as_ref())
                    .ok_or("No authoritative match")?
                    .match_id;
                self.bridge
                    .send::<_, PvpChannel>(LeaveBattleground { match_id })
            }
            _ => Err(format!("Unknown command {action}")),
        }
    }
    fn write_state(&self, directory: &Path) -> Result<(), String> {
        let value =
            serde_json::json!({"entered": self.entered,"map":self.map,"snapshot":self.snapshot});
        fs::write(
            directory.join(format!("peer{}.json", self.index)),
            serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())
    }
}
fn run() -> Result<(), String> {
    let directory = PathBuf::from(std::env::var("BG_DIRECTORY").map_err(|e| e.to_string())?);
    let address = std::env::var("BG_ENDPOINT")
        .map_err(|e| e.to_string())?
        .parse::<std::net::SocketAddr>()
        .map_err(|e| e.to_string())?;
    if !address.ip().is_loopback() || !(5710..=5712).contains(&address.port()) {
        return Err("Private UDP5710–5712 only".into());
    }
    let mut peers = Vec::new();
    let clock = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos() as u64;
    for index in 1..=9 {
        let bridge = BridgeConfig::new()
            .receive::<LoginResponse>()
            .receive::<EnterWorldResponse>()
            .receive::<NewWorld>()
            .receive::<PvpStateUpdate>()
            .connect(address, clock + index as u64)?;
        bridge.send::<_, AuthChannel>(LoginRequest {
            token: None,
            username: format!("fb_bg{index}"),
            password: "fbtest".into(),
        })?;
        peers.push(Peer {
            index,
            bridge,
            entered: false,
            map: None,
            snapshot: None,
        });
    }
    let mut last = String::new();
    let deadline = Instant::now() + Duration::from_secs(1800);
    while Instant::now() < deadline {
        for peer in &mut peers {
            for event in peer.bridge.drain_events()? {
                match event {
                    Event::Message(message) => peer.receive(message)?,
                    Event::Disconnected(reason) => {
                        return Err(format!("fb_bg{} disconnected: {reason:?}", peer.index));
                    }
                    Event::ProtocolRejected(reason) => return Err(reason),
                    _ => {}
                }
            }
            peer.write_state(&directory)?;
        }
        match fs::read_to_string(directory.join("peers-command.txt")) {
            Ok(command) if command != last => {
                last = command;
                let action = last.split_whitespace().next().ok_or("Empty command")?;
                if action == "quit" {
                    return Ok(());
                }
                for peer in &peers {
                    peer.action(action)?;
                }
                println!("submitted {last}");
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
        thread::sleep(Duration::from_millis(50));
    }
    Err("Live peers exceeded 30-minute deadline".into())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
