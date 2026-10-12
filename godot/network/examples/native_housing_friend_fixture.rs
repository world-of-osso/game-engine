//! Private5870 setup/proof through real AddFriend + housing requests, never a friend grant.
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
    friend: String,
    output: PathBuf,
    entered: bool,
    social_known: bool,
    is_friend: bool,
    added: bool,
    queried: bool,
}

impl Peer {
    fn receive(&mut self, message: ProtocolMessage) -> Result<(), String> {
        if message.is::<LoginResponse>() {
            let reply = message
                .downcast::<LoginResponse>()
                .map_err(|_| "login type")?;
            if !reply.success {
                return Err(format!("Login refused: {:?}", reply.error));
            }
            self.bridge.send::<_, AuthChannel>(SelectCharacter {
                character_id: reply.characters.first().ok_or("empty roster")?.character_id,
            })?;
        } else if message.is::<EnterWorldResponse>() {
            let reply = message
                .downcast::<EnterWorldResponse>()
                .map_err(|_| "entry type")?;
            if !reply.success {
                return Err(format!("World entry: {:?}", reply.error));
            }
            self.entered = true;
        } else if message.is::<NewWorld>() {
            self.bridge.send::<_, TransferChannel>(WorldPortAck)?;
        } else if message.is::<FriendsStateUpdate>() {
            self.receive_friends(message)?;
        } else if message.is::<HousingReply>() && self.queried {
            self.write_housing_receipt(message)?;
        }
        Ok(())
    }

    fn receive_friends(&mut self, message: ProtocolMessage) -> Result<(), String> {
        let reply = message
            .downcast::<FriendsStateUpdate>()
            .map_err(|_| "friends type")?;
        if let Some(error) = reply.error {
            return Err(error);
        }
        if let Some(snapshot) = reply.snapshot {
            self.social_known = true;
            self.is_friend = snapshot
                .entries
                .iter()
                .any(|entry| entry.name.eq_ignore_ascii_case(&self.friend));
        }
        Ok(())
    }

    fn write_housing_receipt(&self, message: ProtocolMessage) -> Result<(), String> {
        let reply = message
            .downcast::<HousingReply>()
            .map_err(|_| "housing type")?;
        if let Some(error) = reply.error {
            return Err(error);
        }
        if !reply
            .state
            .plots
            .iter()
            .any(|plot| plot.owner_type == HousingPlotOwnerType::Friend)
        {
            return Err("Housing snapshot omitted actual persisted friend plot".into());
        }
        fs::write(
            &self.output,
            serde_json::to_vec_pretty(&reply.state).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())
    }

    fn send_next_request(&mut self) -> Result<(), String> {
        if !self.entered || !self.social_known {
            return Ok(());
        }
        if self.is_friend && !self.queried {
            self.bridge.send::<_, HousingChannel>(HousingRequest {
                request_id: 1,
                command: HousingCommand::ListPlots { neighborhood_id: 1 },
            })?;
            self.queried = true;
        } else if !self.is_friend && !self.added {
            self.bridge.send::<_, FriendsChannel>(AddFriend {
                name: self.friend.clone(),
            })?;
            self.added = true;
        }
        Ok(())
    }
}

fn connect_peer() -> Result<Peer, String> {
    let address = std::env::var("GODOT_TEST_SERVER").map_err(|e| e.to_string())?;
    let account = std::env::var("HOUSING_ACCOUNT").map_err(|e| e.to_string())?;
    let friend = std::env::var("HOUSING_FRIEND").map_err(|e| e.to_string())?;
    let output = PathBuf::from(std::env::var("HOUSING_FRIEND_RECEIPT").map_err(|e| e.to_string())?);
    if address != "127.0.0.1:5870" || !account.starts_with("fb_housing_") {
        return Err("Private5870/fb_housing_* required".into());
    }
    if output.exists() {
        return Err("Friendship proof requires a fresh receipt path.".into());
    }
    let client_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos() as u64;
    let bridge = BridgeConfig::new()
        .receive::<LoginResponse>()
        .receive::<EnterWorldResponse>()
        .receive::<NewWorld>()
        .receive::<FriendsStateUpdate>()
        .receive::<HousingReply>()
        .connect(
            address
                .parse()
                .map_err(|e: std::net::AddrParseError| e.to_string())?,
            client_id,
        )?;
    bridge.send::<_, AuthChannel>(LoginRequest {
        token: None,
        username: account,
        password: "fbtest".into(),
    })?;
    Ok(Peer {
        bridge,
        friend,
        output,
        entered: false,
        social_known: false,
        is_friend: false,
        added: false,
        queried: false,
    })
}

fn run() -> Result<(), String> {
    let mut peer = connect_peer()?;
    let deadline = Instant::now() + Duration::from_secs(180);
    while Instant::now() < deadline {
        for event in peer.bridge.drain_events()? {
            match event {
                Event::Message(message) => peer.receive(message)?,
                Event::Disconnected(reason) => {
                    return Err(format!("Housing friend peer disconnected: {reason:?}"));
                }
                Event::ProtocolRejected(reason) => return Err(reason),
                _ => {}
            }
        }
        peer.send_next_request()?;
        if peer.output.exists() {
            println!("PASS: real persisted friendship and Retail housing friend relationship");
            return Ok(());
        }
        thread::sleep(Duration::from_millis(25));
    }
    Err("Housing friendship deadline".into())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("HOUSING_FRIEND_ERROR {error}");
        std::process::exit(1);
    }
}
