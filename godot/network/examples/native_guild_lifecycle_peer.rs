//! One private5891 target beside the real native officer UI.
//! Accepts actual invitations; marker requests send ordinary Leave packets, not grants.
use game_engine_network::{BridgeConfig, Event, NetworkBridge, ProtocolMessage};
use shared::protocol::*;
use std::{
    fs,
    path::PathBuf,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Peer {
    bridge: NetworkBridge,
    directory: PathBuf,
    name: String,
    inviter: String,
    entered: bool,
    joined: u64,
    leave_sequence: u64,
    privacy_requested: u64,
    denial_expected: bool,
    member_read_expected: bool,
}

impl Peer {
    fn receive(&mut self, message: ProtocolMessage) -> Result<(), String> {
        if message.is::<LoginResponse>() {
            return self.login(message);
        }
        if message.is::<EnterWorldResponse>() {
            let reply = message
                .downcast::<EnterWorldResponse>()
                .map_err(|_| "decode entry")?;
            if !reply.success {
                return Err(format!("Guild peer entry refused: {:?}", reply.error));
            }
            self.entered = true;
            return self.write(
                "peer-ready.json",
                &serde_json::json!({"entered": true, "name": self.name}),
            );
        }
        if message.is::<NewWorld>() {
            return self.bridge.send::<_, TransferChannel>(WorldPortAck);
        }
        if message.is::<GuildInvitation>() {
            let reply = message
                .downcast::<GuildInvitation>()
                .map_err(|_| "decode invitation")?;
            if reply.guild_name != "Guild Lifecycle" || reply.inviter_name != self.inviter {
                return Err(format!("Unexpected real invitation: {reply:?}"));
            }
            return self.bridge.send::<_, AuthChannel>(GuildAcceptInviteRequest);
        }
        if message.is::<GuildAcceptInviteResponse>() {
            return self.accepted(message);
        }
        if message.is::<GuildRanksState>() {
            return self.ranks(message);
        }
        if message.is::<GuildEventLog>() {
            let reply = message
                .downcast::<GuildEventLog>()
                .map_err(|_| "decode history refusal")?;
            if self.member_read_expected {
                if reply.error.is_some() || reply.events.len() < 2 {
                    return Err(format!(
                        "Ordinary member could not read actual history: {reply:?}"
                    ));
                }
                self.member_read_expected = false;
                return self.write("member-read.json", &reply);
            }
            if reply.error != Some(GuildRankError::NotInGuild) || !reply.events.is_empty() {
                return Err(format!(
                    "Removed peer received private guild history: {reply:?}"
                ));
            }
            return self.write(&format!("privacy-{}.json", self.privacy_requested), &reply);
        }
        Ok(())
    }

    fn login(&self, message: ProtocolMessage) -> Result<(), String> {
        let reply = message
            .downcast::<LoginResponse>()
            .map_err(|_| "decode login")?;
        if !reply.success {
            return Err(format!("Guild peer login refused: {:?}", reply.error));
        }
        let character = reply
            .characters
            .iter()
            .find(|entry| entry.name == self.name)
            .ok_or_else(|| format!("Real guild peer character {} missing", self.name))?;
        self.bridge.send::<_, AuthChannel>(SelectCharacter {
            character_id: character.character_id,
        })
    }

    fn accepted(&mut self, message: ProtocolMessage) -> Result<(), String> {
        let reply = message
            .downcast::<GuildAcceptInviteResponse>()
            .map_err(|_| "decode acceptance")?;
        if !reply.success {
            return Err(format!("Guild peer acceptance refused: {:?}", reply.error));
        }
        self.joined += 1;
        self.write(
            "joined.json",
            &serde_json::json!({"joined": self.joined, "name": self.name}),
        )?;
        if self.joined == 1 {
            self.member_read_expected = true;
            self.bridge
                .send::<_, GuildChannel>(GuildRankRequest::QueryEventLog)?;
            self.denial_expected = true;
            self.bridge
                .send::<_, GuildChannel>(GuildRankRequest::Promote {
                    character_name: self.inviter.clone(),
                })?;
        }
        Ok(())
    }

    fn ranks(&mut self, message: ProtocolMessage) -> Result<(), String> {
        let reply = message
            .downcast::<GuildRanksState>()
            .map_err(|_| "decode rank response")?;
        if let Some(error) = reply.error {
            if !self.denial_expected || error != GuildRankError::Permissions {
                return Err(format!("Unexpected guild peer refusal: {error:?}"));
            }
            self.denial_expected = false;
            return self.write("permission-refusal.json", &reply);
        }
        if reply.ranks.is_empty() && self.leave_sequence > self.privacy_requested {
            self.privacy_requested = self.leave_sequence;
            self.bridge
                .send::<_, GuildChannel>(GuildRankRequest::QueryEventLog)?;
        }
        Ok(())
    }

    fn poll(&mut self) -> Result<(), String> {
        for event in self.bridge.drain_events()? {
            match event {
                Event::Message(message) => self.receive(message)?,
                Event::Disconnected(reason) => {
                    return Err(format!("Guild peer disconnected: {reason:?}"));
                }
                Event::ProtocolRejected(reason) => return Err(reason),
                _ => {}
            }
        }
        let next = self.leave_sequence + 1;
        if self.entered && self.directory.join(format!("leave-{next}.go")).exists() {
            self.bridge
                .send::<_, GuildChannel>(GuildRankRequest::Leave)?;
            self.leave_sequence = next;
        }
        Ok(())
    }

    fn write<T: serde::Serialize>(&self, filename: &str, value: &T) -> Result<(), String> {
        let path = self.directory.join(filename);
        let temporary = path.with_extension("new");
        fs::write(
            &temporary,
            serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?,
        )
        .map_err(|error| format!("Write {}: {error}", temporary.display()))?;
        fs::rename(&temporary, &path)
            .map_err(|error| format!("Publish {}: {error}", path.display()))
    }
}

fn connect() -> Result<Peer, String> {
    let endpoint = std::env::var("GODOT_TEST_SERVER").map_err(|error| error.to_string())?;
    let account = std::env::var("GUILD_LOG_PEER_ACCOUNT").map_err(|error| error.to_string())?;
    if endpoint != "127.0.0.1:5891" || !account.starts_with("fb_guildcompletion_") {
        return Err("Private5891/fb_guildcompletion_* only".into());
    }
    let directory =
        PathBuf::from(std::env::var("GUILD_LOG_DIAGNOSTICS").map_err(|error| error.to_string())?);
    if directory.join("peer-ready.json").exists() {
        return Err("Fresh peer evidence directory required".into());
    }
    let client_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos() as u64;
    let bridge = BridgeConfig::new()
        .receive::<LoginResponse>()
        .receive::<EnterWorldResponse>()
        .receive::<NewWorld>()
        .receive::<GuildInvitation>()
        .receive::<GuildAcceptInviteResponse>()
        .receive::<GuildRanksState>()
        .receive::<GuildEventLog>()
        .connect(
            endpoint
                .parse()
                .map_err(|error: std::net::AddrParseError| error.to_string())?,
            client_id,
        )?;
    bridge.send::<_, AuthChannel>(LoginRequest {
        token: None,
        username: account,
        password: "fbtest".into(),
    })?;
    Ok(Peer {
        bridge,
        directory,
        name: std::env::var("GUILD_LOG_PEER_NAME").map_err(|error| error.to_string())?,
        inviter: std::env::var("GUILD_LOG_ACTOR_NAME").map_err(|error| error.to_string())?,
        entered: false,
        joined: 0,
        leave_sequence: 0,
        privacy_requested: 0,
        denial_expected: false,
        member_read_expected: false,
    })
}

fn run() -> Result<(), String> {
    let mut peer = connect()?;
    // Functional100-action proof, not a latency benchmark; private rendering may run at5FPS.
    let deadline = Instant::now() + Duration::from_secs(1800);
    let result = loop {
        if let Err(error) = peer.poll() {
            break Err(error);
        }
        if peer.directory.join("done.go").exists() {
            if peer.joined != 5
                || peer.privacy_requested != 2
                || peer.denial_expected
                || peer.member_read_expected
            {
                break Err(format!(
                    "Incomplete real lifecycle proof: joins={} privacy={} denied_pending={}",
                    peer.joined, peer.privacy_requested, peer.denial_expected
                ));
            }
            println!("PASS GUILD_LIFECYCLE_PEER joins=5 private_refusals=2 permission_refusal=1");
            break Ok(());
        }
        if Instant::now() >= deadline {
            break Err("Guild peer lifecycle deadline".into());
        }
        thread::sleep(Duration::from_millis(25));
    };
    let stopped = peer.bridge.stop();
    if let Err(error) = &stopped {
        eprintln!("Guild peer cleanup: {error}");
    }
    result.and(stopped)
}

fn main() {
    if let Err(error) = run() {
        eprintln!("GUILD_LIFECYCLE_PEER_ERROR {error}");
        std::process::exit(1);
    }
}
