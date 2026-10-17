//! Private5891 real guild officer beside the rendered member's effect proof.
use game_engine_network::{BridgeConfig, Event, NetworkBridge, ProtocolMessage};
use shared::protocol::*;
use std::{
    fs,
    path::PathBuf,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Officer {
    bridge: NetworkBridge,
    directory: PathBuf,
    entered: bool,
    invites: u32,
    removals: u32,
}

impl Officer {
    fn marker(&self, name: &str) -> PathBuf {
        self.directory.join(name)
    }

    fn receive(&mut self, message: ProtocolMessage) -> Result<(), String> {
        if message.is::<LoginResponse>() {
            return self.select_owner(message);
        }
        if message.is::<NewWorld>() {
            return self.bridge.send::<_, TransferChannel>(WorldPortAck);
        }
        if message.is::<EnterWorldResponse>() {
            let reply = message
                .downcast::<EnterWorldResponse>()
                .map_err(|_| "owner entry decode")?;
            if !reply.success {
                return Err(format!("Owner entry refused: {:?}", reply.error));
            }
            self.entered = true;
            fs::write(self.marker("officer-ready.go"), b"ready").map_err(|e| e.to_string())?;
        } else if message.is::<GuildInviteResponse>() {
            let reply = message
                .downcast::<GuildInviteResponse>()
                .map_err(|_| "invitation reply decode")?;
            if !reply.success {
                return Err(format!("Real invitation refused: {:?}", reply.error));
            }
            println!("OFFICER_INVITED {}", self.invites);
        } else if message.is::<GuildRanksState>() {
            let reply = message
                .downcast::<GuildRanksState>()
                .map_err(|_| "officer rank reply decode")?;
            if let Some(error) = reply.error {
                return Err(format!("Real officer request refused: {error:?}"));
            }
            if self.removals > 0
                && reply
                    .members
                    .iter()
                    .all(|row| row.character_name != "Gcperkspeer")
            {
                fs::write(
                    self.marker(&format!("removed-{}.go", self.removals)),
                    b"removed",
                )
                .map_err(|e| e.to_string())?;
                println!("OFFICER_REMOVED {}", self.removals);
            }
        }
        Ok(())
    }

    fn select_owner(&self, message: ProtocolMessage) -> Result<(), String> {
        let reply = message
            .downcast::<LoginResponse>()
            .map_err(|_| "owner login decode")?;
        if !reply.success {
            return Err(format!("Owner login refused: {:?}", reply.error));
        }
        let owner = reply
            .characters
            .iter()
            .find(|row| row.name == "Gcperkslead")
            .ok_or("Actual private owner character missing")?;
        self.bridge.send::<_, AuthChannel>(SelectCharacter {
            character_id: owner.character_id,
        })
    }

    fn send_marked_requests(&mut self) -> Result<(), String> {
        if !self.entered {
            return Ok(());
        }
        let next_invite = self.invites + 1;
        if self.marker(&format!("invite-{next_invite}.go")).exists() {
            self.bridge.send::<_, AuthChannel>(GuildInviteRequest {
                target_character_name: "Gcperkspeer".into(),
            })?;
            self.invites = next_invite;
        }
        let next_removal = self.removals + 1;
        if self.marker(&format!("remove-{next_removal}.go")).exists() {
            self.bridge
                .send::<_, GuildChannel>(GuildRankRequest::RemoveMember {
                    character_name: "Gcperkspeer".into(),
                })?;
            self.removals = next_removal;
        }
        Ok(())
    }

    fn drive(&mut self) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(1200);
        loop {
            for event in self.bridge.drain_events()? {
                match event {
                    Event::Message(message) => self.receive(message)?,
                    Event::Disconnected(reason) => {
                        return Err(format!("Owner disconnected: {reason:?}"));
                    }
                    Event::ProtocolRejected(reason) => return Err(reason),
                    _ => {}
                }
            }
            self.send_marked_requests()?;
            if self.marker("done.go").exists() {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err("Officer effect-proof deadline exceeded".into());
            }
            thread::sleep(Duration::from_millis(25));
        }
    }
}

fn run() -> Result<(), String> {
    let directory =
        PathBuf::from(std::env::var("GUILD_PERKS_OFFICER_DIR").map_err(|e| e.to_string())?);
    if directory.join("officer-ready.go").exists() {
        return Err("Fresh officer proof directory required".into());
    }
    let client_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos() as u64;
    let bridge = BridgeConfig::new()
        .receive::<LoginResponse>()
        .receive::<NewWorld>()
        .receive::<EnterWorldResponse>()
        .receive::<GuildInviteResponse>()
        .receive::<GuildRanksState>()
        .connect("127.0.0.1:5891".parse().unwrap(), client_id)?;
    bridge.send::<_, AuthChannel>(LoginRequest {
        token: None,
        username: "fb_guildcompletion_perkslead".into(),
        password: "fbtest".into(),
    })?;
    let mut officer = Officer {
        bridge,
        directory,
        entered: false,
        invites: 0,
        removals: 0,
    };
    let result = officer.drive();
    let stopped = officer.bridge.stop();
    result.and(stopped)?;
    println!(
        "PASS GUILD_PERKS_OFFICER invites={} removals={}",
        officer.invites, officer.removals
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("FAIL GUILD_PERKS_OFFICER {error}");
        std::process::exit(1);
    }
}
