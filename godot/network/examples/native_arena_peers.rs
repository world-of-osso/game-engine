//! Real-UDP skirmish2v2/3v3 and actual-group rated2v2 peers alongside a native client.
//! No simulated match state, synthetic roster or server shortcuts.
use game_engine_network::{BridgeConfig, Event, NetworkBridge, ProtocolMessage};
use shared::protocol::*;
use std::{
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
struct Peer {
    index: usize,
    mode: ArenaQueueMode,
    bridge: NetworkBridge,
    address: SocketAddr,
    reconnects: u32,
    player_entity: Option<u64>,
    terrain: Option<String>,
    entered: bool,
    map: Option<u32>,
    snapshot: Option<PvpSnapshot>,
    group: Option<GroupRosterSnapshot>,
    spec_id: Option<u32>,
}
impl Peer {
    fn connect(index: usize, mode: ArenaQueueMode, address: SocketAddr) -> Result<Self, String> {
        let bridge = connect_peer_bridge(index, address)?;
        Ok(Self {
            index,
            mode,
            bridge,
            address,
            reconnects: 0,
            player_entity: None,
            terrain: None,
            entered: false,
            map: None,
            snapshot: None,
            group: None,
            spec_id: None,
        })
    }

    fn reconnect(&mut self) -> Result<(), String> {
        self.bridge.disconnect()?;
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if self
                .bridge
                .drain_events()?
                .iter()
                .any(|event| matches!(event, Event::Disconnected(_)))
            {
                break;
            }
            if Instant::now() >= deadline {
                return Err(format!("fb_arena{} did not disconnect", self.index));
            }
            thread::sleep(Duration::from_millis(50));
        }
        self.bridge.stop()?;
        let reconnects = self.reconnects + 1;
        let mut replacement = Self::connect(self.index, self.mode, self.address)?;
        replacement.reconnects = reconnects;
        *self = replacement;
        Ok(())
    }

    fn receive(&mut self, message: ProtocolMessage) -> Result<(), String> {
        if message.is::<LoginResponse>() {
            let reply = message
                .downcast::<LoginResponse>()
                .map_err(|_| "login type")?;
            if !reply.success {
                return Err(format!("fb_arena{} login: {:?}", self.index, reply.error));
            }
            let character = if self.mode == ArenaQueueMode::SoloShuffle && self.index <= 2 {
                let name = if self.index == 1 {
                    "Arenahealone"
                } else {
                    "Arenahealtwo"
                };
                reply
                    .characters
                    .iter()
                    .find(|character| character.name == name)
                    .ok_or("Required source-role healer character absent")?
            } else {
                reply.characters.first().ok_or("Empty roster")?
            };
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
            self.player_entity = reply.player_entity;
            self.bridge.send::<_, PvpChannel>(QueryPvpStatus)?;
            if self.mode == ArenaQueueMode::SoloShuffle {
                self.bridge.send::<_, TalentChannel>(SetSpecialization {
                    spec_id: if self.index <= 2 { 257 } else { 71 },
                })?;
            }
        } else if message.is::<SpecializationChanged>() {
            self.spec_id = Some(
                message
                    .downcast::<SpecializationChanged>()
                    .map_err(|_| "specialization type")?
                    .spec_id,
            );
        } else if message.is::<LoadTerrain>() {
            let terrain = message
                .downcast::<LoadTerrain>()
                .map_err(|_| "terrain type")?;
            self.terrain = Some(terrain.map_name);
        } else if message.is::<NewWorld>() {
            let destination = message
                .downcast::<NewWorld>()
                .map_err(|_| "transfer type")?;
            self.map = Some(destination.map_id);
            self.bridge.send::<_, TransferChannel>(WorldPortAck)?;
            println!(
                "fb_arena{} WorldPortAck map{}",
                self.index, destination.map_id
            );
        } else if message.is::<GroupInvitePrompt>() {
            self.bridge
                .send::<_, GroupChannel>(RespondGroupInvite { accept: true })?;
        } else if message.is::<GroupRosterSnapshot>() {
            self.group = Some(
                message
                    .downcast::<GroupRosterSnapshot>()
                    .map_err(|_| "group type")?,
            );
        } else if message.is::<PvpStateUpdate>() {
            let update = message
                .downcast::<PvpStateUpdate>()
                .map_err(|_| "pvp type")?;
            if let Some(error) = update.error {
                return Err(format!("fb_arena{} PvP: {error}", self.index));
            }
            if let Some(snapshot) = update.snapshot {
                self.snapshot = Some(snapshot);
            }
        }
        Ok(())
    }
    fn action(&mut self, action: &str) -> Result<(), String> {
        if !self.entered {
            return Err(format!("fb_arena{} has not entered", self.index));
        }
        if let Some(target) = action.strip_prefix("attack:") {
            return self.bridge.send::<_, CombatChannel>(AttackSwing {
                target: target.parse().map_err(|_| "attack target entity")?,
            });
        }
        if let Some(cast) = action.strip_prefix("cast:") {
            return self.cast_action(cast);
        }
        match action {
            "reconnect" => self.reconnect(),
            "group" if self.mode == ArenaQueueMode::Rated2v2 && self.index == 2 => {
                self.bridge.send::<_, GroupChannel>(GroupInviteIntent {
                    name: "Arenathree".into(),
                })
            }
            "group" => Ok(()),
            "queue" if self.mode == ArenaQueueMode::Rated2v2 && self.index == 2 => {
                self.bridge.send::<_, PvpChannel>(QueueForRatedPvp {
                    bracket: PvpBracketSnapshot::Arena2v2,
                })
            }
            "queue" if self.mode == ArenaQueueMode::Rated2v2 => Ok(()),
            "queue" => self
                .bridge
                .send::<_, PvpChannel>(QueueForArena { mode: self.mode }),
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
            "forfeit" if self.mode == ArenaQueueMode::Rated2v2 && self.index == 1 => Ok(()),
            // Opposing rated peers already returned when they forfeited; only the retained ally leaves now.
            "leave" if self.mode == ArenaQueueMode::Rated2v2 && self.index != 1 => Ok(()),
            "leave" | "forfeit" => {
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
    fn cast_action(&self, value: &str) -> Result<(), String> {
        let (spell, target) = value
            .split_once(':')
            .ok_or("Expected cast:<spell-id>:<target-entity>")?;
        self.bridge.send::<_, CombatChannel>(SpellCastIntent {
            spell_id: Some(spell.parse().map_err(|_| "spell ID")?),
            spell: String::new(),
            target_entity: Some(target.parse().map_err(|_| "cast target entity")?),
            destination: None,
            target_item_guid: None,
            witness: None,
        })
    }
    fn write_state(&self, directory: &Path) -> Result<(), String> {
        let value = serde_json::json!({"spec_id":self.spec_id,"entered": self.entered,"map":self.map,"terrain":self.terrain,"player_entity":self.player_entity,"reconnects":self.reconnects,"snapshot":self.snapshot,"group":self.group});
        fs::write(
            directory.join(format!("peer{}.json", self.index)),
            serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())
    }
}
fn run() -> Result<(), String> {
    let directory = PathBuf::from(std::env::var("ARENA_DIRECTORY").map_err(|e| e.to_string())?);
    let address = std::env::var("ARENA_ENDPOINT")
        .map_err(|e| e.to_string())?
        .parse::<std::net::SocketAddr>()
        .map_err(|e| e.to_string())?;
    if !address.ip().is_loopback() || address.port() != 5941 {
        return Err("Private UDP5941 only".into());
    }
    let (mode, team_size) = match std::env::var("ARENA_MODE").as_deref() {
        Ok("2v2") => (ArenaQueueMode::Skirmish2v2, 2),
        Ok("3v3") => (ArenaQueueMode::Skirmish3v3, 3),
        Ok("rated2v2") => (ArenaQueueMode::Rated2v2, 2),
        Ok("shuffle") => (ArenaQueueMode::SoloShuffle, 3),
        _ => return Err("ARENA_MODE must explicitly select 2v2, 3v3, rated2v2 or shuffle".into()),
    };
    let mut peers = (1..team_size * 2)
        .map(|index| Peer::connect(index, mode, address))
        .collect::<Result<Vec<_>, _>>()?;
    let mut last = String::new();
    let deadline = Instant::now() + Duration::from_secs(3600);
    while Instant::now() < deadline {
        for peer in &mut peers {
            for event in peer.bridge.drain_events()? {
                match event {
                    Event::Message(message) => peer.receive(message)?,
                    Event::Disconnected(reason) => {
                        return Err(format!("fb_arena{} disconnected: {reason:?}", peer.index));
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
                submit_action(&mut peers, action)?;
                println!("submitted {last}");
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
        thread::sleep(Duration::from_millis(50));
    }
    Err("Live peers exceeded one-hour integration deadline".into())
}
fn connect_peer_bridge(index: usize, address: SocketAddr) -> Result<NetworkBridge, String> {
    let client_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos() as u64;
    let bridge = BridgeConfig::new()
        .receive::<LoginResponse>()
        .receive::<EnterWorldResponse>()
        .receive::<LoadTerrain>()
        .receive::<NewWorld>()
        .receive::<PvpStateUpdate>()
        .receive::<GroupInvitePrompt>()
        .receive::<GroupRosterSnapshot>()
        .receive::<SpecializationChanged>()
        .connect(address, client_id)?;
    bridge.send::<_, AuthChannel>(LoginRequest {
        token: None,
        username: format!("fb_arena{index}"),
        password: "fbtest".into(),
    })?;
    Ok(bridge)
}

fn submit_action(peers: &mut [Peer], action: &str) -> Result<(), String> {
    if let Some(selected) = action.strip_prefix("peer:") {
        let (index, action) = selected
            .split_once(':')
            .ok_or("Expected peer:<index>:<action>")?;
        let index = index.parse::<usize>().map_err(|error| error.to_string())?;
        let peer = peers
            .iter_mut()
            .find(|peer| peer.index == index)
            .ok_or_else(|| format!("No private peer {index}"))?;
        return peer.action(action);
    }
    for peer in peers {
        peer.action(action)?;
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
