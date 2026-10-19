//! Real authenticated peer for the private voluntary leadership native fixture.
use game_engine_network::{BridgeConfig, Event, NetworkBridge, ProtocolMessage};
use shared::protocol::*;
use std::{
    fs,
    net::SocketAddr,
    path::PathBuf,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Peer {
    bridge: NetworkBridge,
    directory: PathBuf,
    character: String,
    target: String,
    entered: bool,
    transfers: u32,
    snapshots: u32,
}

impl Peer {
    fn select_character(&self, message: ProtocolMessage) -> Result<(), String> {
        let reply = message
            .downcast::<LoginResponse>()
            .map_err(|_| "login decode")?;
        if !reply.success {
            return Err(format!("Private peer login refused: {:?}", reply.error));
        }
        let character = reply
            .characters
            .iter()
            .find(|row| row.name == self.character)
            .ok_or("Actual private peer character missing")?;
        self.bridge.send::<_, AuthChannel>(SelectCharacter {
            character_id: character.character_id,
        })
    }

    fn publish_state(&mut self, message: ProtocolMessage) -> Result<(), String> {
        let reply = message
            .downcast::<GuildRanksState>()
            .map_err(|_| "guild state decode")?;
        self.snapshots += 1;
        let json = serde_json::to_vec_pretty(&reply).map_err(|error| error.to_string())?;
        let pending = self.directory.join("peer-state.pending");
        fs::write(&pending, &json).map_err(|error| error.to_string())?;
        fs::rename(pending, self.directory.join("peer-state.json"))
            .map_err(|error| error.to_string())?;
        fs::write(
            self.directory
                .join(format!("peer-state-{}.json", self.snapshots)),
            json,
        )
        .map_err(|error| error.to_string())?;
        println!(
            "GUILD_LEADERSHIP_PEER_STATE character={} own_rank={} error={:?}",
            self.character, reply.own_rank, reply.error
        );
        if let Some(error) = reply.error {
            return Err(format!("Actual peer guild request refused: {error:?}"));
        }
        Ok(())
    }

    fn receive(&mut self, message: ProtocolMessage) -> Result<(), String> {
        if message.is::<LoginResponse>() {
            return self.select_character(message);
        }
        if message.is::<NewWorld>() {
            return self.bridge.send::<_, TransferChannel>(WorldPortAck);
        }
        if message.is::<EnterWorldResponse>() {
            let reply = message
                .downcast::<EnterWorldResponse>()
                .map_err(|_| "entry decode")?;
            if !reply.success {
                return Err(format!("Private peer entry refused: {:?}", reply.error));
            }
            self.entered = true;
            self.bridge
                .send::<_, GuildChannel>(GuildRankRequest::Query)?;
            fs::write(self.directory.join("peer-ready.go"), b"ready")
                .map_err(|error| error.to_string())?;
        } else if message.is::<GuildRanksState>() {
            self.publish_state(message)?;
        }
        Ok(())
    }

    fn send_marked_transfer(&mut self) -> Result<(), String> {
        let next = self.transfers + 1;
        if self.entered && self.directory.join(format!("transfer-{next}.go")).exists() {
            self.bridge
                .send::<_, GuildChannel>(GuildRankRequest::TransferLeadership {
                    character_name: self.target.clone(),
                })?;
            self.transfers = next;
            println!(
                "GUILD_LEADERSHIP_PEER_TRANSFER actor={} target={} number={next}",
                self.character, self.target
            );
        }
        Ok(())
    }

    fn drive(&mut self) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(1200);
        loop {
            if self.directory.join("done.go").exists() {
                return Ok(());
            }
            for event in self.bridge.drain_events()? {
                match event {
                    Event::Message(message) => self.receive(message)?,
                    Event::Disconnected(reason) => {
                        return Err(format!("Private peer disconnected: {reason:?}"));
                    }
                    Event::ProtocolRejected(reason) => return Err(reason),
                    _ => {}
                }
            }
            self.send_marked_transfer()?;
            if Instant::now() >= deadline {
                return Err("Private leadership peer exceeded1200s".into());
            }
            thread::sleep(Duration::from_millis(25));
        }
    }
}

fn read_required_env(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|error| format!("{name}: {error}"))
}

fn read_private_server() -> Result<SocketAddr, String> {
    let server: SocketAddr = read_required_env("GUILD_TRANSFER_SERVER")?
        .parse()
        .map_err(|error| format!("Private address: {error}"))?;
    if !server.ip().is_loopback() || server.port() == 5000 {
        return Err("Explicit private loopback port required; never5000".into());
    }
    Ok(server)
}

fn read_private_account() -> Result<String, String> {
    let account = read_required_env("GUILD_TRANSFER_ACCOUNT")?;
    if !account.starts_with("fb_guildcompletion_") {
        return Err("Private fixture account required".into());
    }
    Ok(account)
}

fn read_private_directory() -> Result<PathBuf, String> {
    let directory = PathBuf::from(read_required_env("GUILD_TRANSFER_DIRECTORY")?);
    if !directory.is_dir() || directory.join("peer-ready.go").exists() {
        return Err("Fresh owned peer directory required".into());
    }
    Ok(directory)
}

fn connect_peer(server: SocketAddr, account: String) -> Result<NetworkBridge, String> {
    let client_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos() as u64;
    let bridge = BridgeConfig::new()
        .receive::<LoginResponse>()
        .receive::<NewWorld>()
        .receive::<EnterWorldResponse>()
        .receive::<GuildRanksState>()
        .connect(server, client_id)?;
    bridge.send::<_, AuthChannel>(LoginRequest {
        token: None,
        username: account,
        password: "fbtest".into(),
    })?;
    Ok(bridge)
}

fn run() -> Result<(), String> {
    let server = read_private_server()?;
    let account = read_private_account()?;
    let directory = read_private_directory()?;
    let character = read_required_env("GUILD_TRANSFER_CHARACTER")?;
    let target = read_required_env("GUILD_TRANSFER_TARGET")?;
    let bridge = connect_peer(server, account)?;
    let mut peer = Peer {
        bridge,
        directory,
        character,
        target,
        entered: false,
        transfers: 0,
        snapshots: 0,
    };
    let result = peer.drive();
    let stopped = peer.bridge.stop();
    result.and(stopped)?;
    println!(
        "PASS GUILD_LEADERSHIP_PEER transfers={} snapshots={}",
        peer.transfers, peer.snapshots
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("FAIL GUILD_LEADERSHIP_PEER {error}");
        std::process::exit(1);
    }
}
