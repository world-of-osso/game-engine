//! Source-minimum real-UDP account peers alongside one native Godot client.
//! No simulated match state, synthetic roster or server shortcuts.
use game_engine_network::{BridgeConfig, Event, NetworkBridge, ProtocolMessage, replica::Replica};
use shared::battleground::BgTeam;
use shared::protocol::*;
use std::{
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
struct Peer {
    index: usize,
    username: String,
    battleground_id: u32,
    bridge: NetworkBridge,
    entered: bool,
    map: Option<u32>,
    snapshot: Option<PvpSnapshot>,
    replica: Replica,
}
impl Peer {
    fn receive(&mut self, message: ProtocolMessage) -> Result<(), String> {
        if message.is::<LoginResponse>() {
            let reply = message
                .downcast::<LoginResponse>()
                .map_err(|_| "login type")?;
            if !reply.success {
                return Err(format!("{} login: {:?}", self.username, reply.error));
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
            self.bridge.send::<_, TransferChannel>(WorldPortAck)?;
            println!("{} WorldPortAck map{}", self.username, destination.map_id);
        } else if message.is::<PvpStateUpdate>() {
            let update = message
                .downcast::<PvpStateUpdate>()
                .map_err(|_| "pvp type")?;
            if let Some(error) = update.error {
                return Err(format!("{} PvP: {error}", self.username));
            }
            if let Some(snapshot) = update.snapshot {
                self.snapshot = Some(snapshot);
            }
        }
        Ok(())
    }
    fn receive_events(&mut self) -> Result<(), String> {
        for event in self.bridge.drain_events()? {
            match event {
                Event::Message(message) => self.receive(message)?,
                Event::ReplicationStarted(schema) => self.replica = Replica::new(schema),
                Event::Replication(batch) => self
                    .replica
                    .apply(batch)
                    .map_err(|error| error.to_string())?,
                Event::Disconnected(reason) => {
                    return Err(format!("{} disconnected: {reason:?}", self.username));
                }
                Event::ProtocolRejected(reason) => return Err(reason),
                _ => {}
            }
        }
        Ok(())
    }

    fn action(&self, action: &str) -> Result<(), String> {
        if !self.entered {
            return Err(format!("{} has not entered", self.username));
        }
        match action {
            "queue" => self.bridge.send::<_, PvpChannel>(QueueForBattleground {
                battleground_id: self.battleground_id,
            }),
            "accept" | "decline" => {
                self.bridge
                    .send::<_, PvpChannel>(RespondToBattlegroundInvite {
                        match_id: self.battleground()?.match_id,
                        accepted: action == "accept",
                    })
            }
            "leave" => self.bridge.send::<_, PvpChannel>(LeaveBattleground {
                match_id: self.battleground()?.match_id,
            }),
            "attack-stop" => self.bridge.send::<_, CombatChannel>(AttackStop),
            _ if action.starts_with("attack:") => self.attack_source(action),
            _ => Err(format!("Unknown command {action}")),
        }
    }

    fn battleground(&self) -> Result<&BattlegroundSnapshot, String> {
        self.snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.battleground.as_ref())
            .ok_or_else(|| "No authoritative battleground".into())
    }

    fn attack_source(&self, action: &str) -> Result<(), String> {
        let (entry, team) = parse_source_attack(action)?;
        let bg = self.battleground()?;
        if bg.team != team {
            return Ok(());
        }
        if bg.phase != shared::battleground::BgPhase::InProgress || !bg.accepted {
            return Err(format!("{} has no active accepted match", self.username));
        }
        let target = self
            .replica
            .units()
            .find_map(|unit| {
                let npc = unit.get::<shared::components::Npc>()?;
                let health = unit.get::<shared::components::Health>()?;
                (npc.template_id == entry && health.current > 0.0).then_some(unit.server_id)
            })
            .ok_or_else(|| format!("{} cannot see living source NPC{entry}", self.username))?;
        self.bridge.send::<_, CombatChannel>(SetTarget {
            target_entity: Some(target),
        })?;
        self.bridge
            .send::<_, CombatChannel>(AttackSwing { target })?;
        println!(
            "{} submitted normal attack on NPC{entry} entity{target}",
            self.username
        );
        Ok(())
    }

    fn observed_npcs(&self) -> Vec<serde_json::Value> {
        self.replica
            .units()
            .filter_map(|unit| {
                let npc = unit.get::<shared::components::Npc>()?;
                let position = unit.get::<shared::components::Position>()?;
                let health = unit
                    .get::<shared::components::Health>()
                    .map(|health| serde_json::json!({"current":health.current,"max":health.max}));
                Some(
                    serde_json::json!({"server_id":unit.server_id,"entry":npc.template_id,
                "name":npc.name,"position":[position.x,position.y,position.z],"health":health}),
                )
            })
            .collect()
    }

    fn write_state(&self, directory: &Path) -> Result<(), String> {
        let value = serde_json::json!({"entered": self.entered,"map":self.map,
            "snapshot":self.snapshot,"npcs":self.observed_npcs()});
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
    let battleground_id = std::env::var("BG_BATTLEGROUND_ID")
        .map_err(|e| e.to_string())?
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    let template = shared::battleground::template_by_id(battleground_id)
        .ok_or("Unknown source battleground")?;
    let prefix = std::env::var("BG_ACCOUNT_PREFIX").map_err(|e| e.to_string())?;
    if !prefix.starts_with("fb_") {
        return Err("Only private fb_* accounts are allowed".into());
    }
    let count = usize::from(template.min_players_per_team) * 2 - 1;
    let mut peers = Vec::new();
    let clock = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos() as u64;
    for index in 1..=count {
        let bridge = BridgeConfig::new()
            .receive::<LoginResponse>()
            .receive::<EnterWorldResponse>()
            .receive::<NewWorld>()
            .receive::<PvpStateUpdate>()
            .connect(address, clock + index as u64)?;
        let username = format!("{prefix}{index}");
        bridge.send::<_, AuthChannel>(LoginRequest {
            token: None,
            username: username.clone(),
            password: "fbtest".into(),
        })?;
        peers.push(Peer {
            index,
            username,
            battleground_id,
            bridge,
            entered: false,
            map: None,
            snapshot: None,
            replica: Replica::default(),
        });
        // Each real account completes entry before another worker starts. Large
        // rosters must not starve earlier handshakes during parallel app setup.
        let entry_deadline = Instant::now() + Duration::from_secs(30);
        while !peers.last().unwrap().entered {
            if Instant::now() >= entry_deadline {
                return Err(format!("{prefix}{index} did not enter within 30 seconds"));
            }
            for peer in &mut peers {
                peer.receive_events()?;
                peer.write_state(&directory)?;
            }
            thread::sleep(Duration::from_millis(50));
        }
    }
    let mut last = String::new();
    let deadline = Instant::now() + Duration::from_secs(1800);
    while Instant::now() < deadline {
        for peer in &mut peers {
            peer.receive_events()?;
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
fn parse_source_attack(action: &str) -> Result<(u32, BgTeam), String> {
    let fields: Vec<_> = action.split(':').collect();
    let ["attack", entry, team] = fields.as_slice() else {
        return Err("expected attack:<source-NPC-entry>:<A|B>".into());
    };
    let entry: u32 = entry
        .parse()
        .map_err(|_| "source NPC entry must be an integer")?;
    if entry == 0 {
        return Err("source NPC entry must be nonzero".into());
    }
    let team = match *team {
        "A" => BgTeam::A,
        "B" => BgTeam::B,
        _ => return Err("attack requires authoritative team A or B".into()),
    };
    Ok((entry, team))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_attack_command_retains_npc_identity_and_authoritative_team() {
        assert_eq!(
            parse_source_attack("attack:11946:A"),
            Ok((11946, BgTeam::A))
        );
        assert_eq!(
            parse_source_attack("attack:11948:B"),
            Ok((11948, BgTeam::B))
        );
    }

    #[test]
    fn source_attack_command_rejects_malformed_and_unfiltered_attacks() {
        for input in [
            "attack",
            "attack:11946",
            "attack:0:A",
            "attack:bad:A",
            "attack:11946:Q",
            "attack:11946:A:all",
            "kill:11946:A",
        ] {
            assert!(parse_source_attack(input).is_err(), "{input}");
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
