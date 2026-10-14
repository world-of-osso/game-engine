//! One real-UDP peer for quest acceptance setup and native party/Share proof.
use game_engine_network::{BridgeConfig, Event, NetworkBridge, ProtocolMessage, replica::Replica};
use shared::{components::Npc, protocol::*};
use std::{
    collections::BTreeMap,
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Peer {
    bridge: NetworkBridge,
    replica: Replica,
    character: String,
    entered: bool,
    have_log: bool,
    entries: BTreeMap<u32, QuestEntrySnapshot>,
    details: Option<QuestGiverQuestDetails>,
    roster: Vec<String>,
}
impl Peer {
    fn connect(account: &str) -> Result<Self, String> {
        if !account.starts_with("fb_quest") {
            return Err("Only disposable quest accounts are allowed".into());
        }
        let address: SocketAddr = std::env::var("GODOT_TEST_SERVER")
            .map_err(|e| e.to_string())?
            .parse()
            .map_err(|e| format!("Private server address: {e}"))?;
        if !address.ip().is_loopback() || address.port() != 5801 {
            return Err("Fixture requires private loopback UDP5801".into());
        }
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos() as u64;
        let bridge = BridgeConfig::new()
            .receive::<LoginResponse>()
            .receive::<EnterWorldResponse>()
            .receive::<NewWorld>()
            .receive::<QuestLogSnapshot>()
            .receive::<QuestLogUpdate>()
            .receive::<QuestGiverQuestDetails>()
            .receive::<QuestFailed>()
            .receive::<GroupRosterSnapshot>()
            .receive::<GroupCommandResponse>()
            .connect(address, nonce)?;
        bridge.send::<_, AuthChannel>(LoginRequest {
            token: None,
            username: account.into(),
            password: "fbtest".into(),
        })?;
        Ok(Self {
            bridge,
            replica: Replica::default(),
            character: String::new(),
            entered: false,
            have_log: false,
            entries: BTreeMap::new(),
            details: None,
            roster: vec![],
        })
    }
    fn poll(&mut self) -> Result<(), String> {
        for event in self.bridge.drain_events()? {
            match event {
                Event::Message(message) => self.receive(message)?,
                Event::ReplicationStarted(schema) => self.replica = Replica::new(schema),
                Event::Replication(batch) => {
                    self.replica.apply(batch).map_err(|e| e.to_string())?
                }
                Event::Disconnected(reason) => {
                    return Err(format!("Quest peer disconnected: {reason:?}"));
                }
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
                .map_err(|_| "Login type")?;
            if !reply.success {
                return Err(format!("Quest peer login: {:?}", reply.error));
            }
            let character = reply
                .characters
                .first()
                .ok_or("Empty quest fixture roster")?;
            self.character = character.name.clone();
            self.bridge.send::<_, AuthChannel>(SelectCharacter {
                character_id: character.character_id,
            })?;
        } else if message.is::<EnterWorldResponse>() {
            let reply = message
                .downcast::<EnterWorldResponse>()
                .map_err(|_| "Entry type")?;
            if !reply.success {
                return Err(format!("Quest peer entry: {:?}", reply.error));
            }
            self.entered = true;
        } else if message.is::<NewWorld>() {
            self.bridge.send::<_, TransferChannel>(WorldPortAck)?;
        } else {
            self.receive_quest_or_group(message)?;
        }
        Ok(())
    }
    fn receive_quest_or_group(&mut self, message: ProtocolMessage) -> Result<(), String> {
        if message.is::<QuestLogSnapshot>() {
            let snapshot = message
                .downcast::<QuestLogSnapshot>()
                .map_err(|_| "Quest snapshot type")?;
            self.entries = snapshot
                .entries
                .into_iter()
                .map(|entry| (entry.quest_id, entry))
                .collect();
            self.have_log = true;
        } else if message.is::<QuestLogUpdate>() {
            let update = message
                .downcast::<QuestLogUpdate>()
                .map_err(|_| "Quest update type")?;
            for id in update.removed {
                self.entries.remove(&id);
            }
            self.entries.extend(
                update
                    .changed
                    .into_iter()
                    .map(|entry| (entry.quest_id, entry)),
            );
        } else if message.is::<QuestGiverQuestDetails>() {
            self.details = Some(
                message
                    .downcast::<QuestGiverQuestDetails>()
                    .map_err(|_| "Details type")?,
            );
        } else if message.is::<QuestFailed>() {
            return Err(format!(
                "Real quest request refused: {:?}",
                message
                    .downcast::<QuestFailed>()
                    .map_err(|_| "Quest refusal type")?
            ));
        } else if message.is::<GroupRosterSnapshot>() {
            self.roster = message
                .downcast::<GroupRosterSnapshot>()
                .map_err(|_| "Roster type")?
                .members
                .into_iter()
                .map(|member| member.name)
                .collect();
            println!("QUEST_PEER_ROSTER {:?}", self.roster);
        } else if message.is::<GroupCommandResponse>() {
            println!(
                "QUEST_PEER_GROUP {:?}",
                message
                    .downcast::<GroupCommandResponse>()
                    .map_err(|_| "Group response type")?
            );
        }
        Ok(())
    }
    fn wait(&mut self, what: &str, ready: impl Fn(&Self) -> bool) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(90);
        loop {
            self.poll()?;
            if ready(self) {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(format!("Quest peer timed out: {what}"));
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
    fn prepare(&mut self, quest: u32, npc_entry: u32, output: &Path) -> Result<(), String> {
        self.wait("world and owned log", |p| p.entered && p.have_log)?;
        if !self.entries.contains_key(&quest) {
            self.teleport_to_giver(npc_entry)?;
            self.wait("source quest giver replication", |p| {
                p.find_giver(npc_entry).is_some()
            })?;
            let npc = self.find_giver(npc_entry).ok_or("Quest giver vanished")?;
            self.bridge.send::<_, QuestChannel>(QuestGiverQueryQuest {
                npc,
                quest_id: quest,
            })?;
            self.wait("real quest details", |p| {
                p.details.as_ref().is_some_and(|d| d.quest_id == quest)
            })?;
            self.bridge.send::<_, QuestChannel>(QuestGiverAcceptQuest {
                npc,
                quest_id: quest,
            })?;
            self.wait("authoritative accepted entry", |p| {
                p.entries.contains_key(&quest)
            })?;
        }
        let entry = self
            .entries
            .get(&quest)
            .ok_or("Accepted entry disappeared")?;
        if entry.description.is_empty() {
            return Err("Owned quest entry lost source story".into());
        }
        fs::write(
            output,
            serde_json::to_vec_pretty(entry).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        println!(
            "PASS QUEST_PEER_ACCEPT character={} quest={} pushable={} rewards={}",
            self.character, quest, entry.pushable, entry.rewards.money
        );
        self.disconnect()
    }
    fn disconnect(&mut self) -> Result<(), String> {
        self.bridge.disconnect()?;
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            for event in self.bridge.drain_events()? {
                if matches!(event, Event::Disconnected(_)) {
                    return Ok(());
                }
            }
            thread::sleep(Duration::from_millis(10));
        }
        Err("Quest peer disconnect was not acknowledged".into())
    }
    fn close_party(&mut self, leave_sent: bool) -> Result<(), String> {
        if !self.roster.is_empty() {
            if !leave_sent {
                self.bridge.send::<_, GroupChannel>(LeaveGroup)?;
            }
            self.wait("party cleanup roster empty", |peer| peer.roster.is_empty())?;
        }
        self.disconnect()
    }
    fn find_giver(&self, entry: u32) -> Option<u64> {
        self.replica
            .units()
            .find(|unit| {
                unit.get::<Npc>()
                    .is_some_and(|npc| npc.template_id == entry)
            })
            .map(|unit| unit.server_id)
    }
    fn teleport_to_giver(&self, entry: u32) -> Result<(), String> {
        // Same authored giver setup coordinates as tests/world_quest_flow.gd.
        let coordinates = match entry {
            197 => ["-8915.4", "-137.5", "81.0"],
            240 => ["-9462.5", "74.0", "56.8"],
            _ => return Err("Unsupported fixture giver".into()),
        };
        let binary = std::env::var("QF_ADMIN").map_err(|e| e.to_string())?;
        let output = Command::new(binary)
            .args([
                "teleport",
                self.character.as_str(),
                "0",
                coordinates[0],
                coordinates[1],
                coordinates[2],
            ])
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(format!(
                "Quest giver travel setup: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(())
    }
    fn party(&mut self, target: &str, directory: &Path) -> Result<(), String> {
        self.wait("party peer world entry", |p| p.entered)?;
        fs::create_dir_all(directory).map_err(|e| e.to_string())?;
        fs::write(directory.join("ready"), &self.character).map_err(|e| e.to_string())?;
        let deadline = Instant::now() + Duration::from_secs(240);
        let mut sent_invite = false;
        let mut sent_leave = false;
        while Instant::now() < deadline {
            self.poll()?;
            if directory.join("invite").exists() && !sent_invite {
                self.bridge.send::<_, GroupChannel>(GroupInviteIntent {
                    name: target.into(),
                })?;
                sent_invite = true;
            }
            if self.roster.len() == 2 {
                fs::write(directory.join("grouped"), self.roster.join(","))
                    .map_err(|e| e.to_string())?;
            }
            if directory.join("leave").exists() && !sent_leave {
                self.bridge.send::<_, GroupChannel>(LeaveGroup)?;
                sent_leave = true;
            }
            if sent_leave && self.roster.is_empty() {
                fs::write(directory.join("left"), "server roster empty")
                    .map_err(|e| e.to_string())?;
            }
            if directory.join("stop").exists() {
                return self.close_party(sent_leave);
            }
            thread::sleep(Duration::from_millis(10));
        }
        Err("Party peer control timeout".into())
    }
}
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    let mode = args.get(1).ok_or(
        "prepare <account> <quest> <giver> <receipt> | party <account> <target> <directory>",
    )?;
    let account = args.get(2).ok_or("Missing private account")?;
    let mut peer = Peer::connect(account)?;
    match mode.as_str() {
        "prepare" => {
            let quest = args
                .get(3)
                .ok_or("Missing quest")?
                .parse()
                .map_err(|e| format!("Quest id: {e}"))?;
            let giver = args
                .get(4)
                .ok_or("Missing giver")?
                .parse()
                .map_err(|e| format!("Giver id: {e}"))?;
            peer.prepare(
                quest,
                giver,
                &PathBuf::from(args.get(5).ok_or("Missing receipt path")?),
            )
        }
        "party" => peer.party(
            args.get(3).ok_or("Missing target")?,
            &PathBuf::from(args.get(4).ok_or("Missing control directory")?),
        ),
        _ => Err("Unknown quest peer mode".into()),
    }
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
