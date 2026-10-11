//! One real-UDP fb_* peer for the private Mythic Mentor two-character proof.
use game_engine_network::{BridgeConfig, Event, ProtocolMessage};
use shared::protocol::*;
use std::{
    fs,
    path::PathBuf,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
fn receive(
    bridge: &game_engine_network::NetworkBridge,
    message: ProtocolMessage,
    directory: &std::path::Path,
) -> Result<(), String> {
    if message.is::<LoginResponse>() {
        let reply = message
            .downcast::<LoginResponse>()
            .map_err(|_| "login type")?;
        if !reply.success {
            return Err(format!("Login refused: {:?}", reply.error));
        }
        let character = reply.characters.first().ok_or("Peer roster empty")?;
        bridge.send::<_, AuthChannel>(SelectCharacter {
            character_id: character.character_id,
        })?;
    } else if message.is::<EnterWorldResponse>() {
        let reply = message
            .downcast::<EnterWorldResponse>()
            .map_err(|_| "entry type")?;
        if !reply.success {
            return Err(format!("World entry: {:?}", reply.error));
        }
        fs::write(directory.join("peer-ready"), "entered").map_err(|e| e.to_string())?;
    } else if message.is::<NewWorld>() {
        bridge.send::<_, TransferChannel>(WorldPortAck)?;
    } else if message.is::<GroupInvitePrompt>() {
        bridge.send::<_, GroupChannel>(RespondGroupInvite { accept: true })?;
    } else if message.is::<PartyKeystoneSnapshot>() {
        let snapshot = message
            .downcast::<PartyKeystoneSnapshot>()
            .map_err(|_| "keys type")?;
        fs::write(
            directory.join("peer-keys.json"),
            serde_json::to_vec_pretty(&snapshot).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn run() -> Result<(), String> {
    let directory = PathBuf::from(std::env::var("MENTOR_RUN").map_err(|e| e.to_string())?);
    let address = std::env::var("GODOT_TEST_SERVER").map_err(|e| e.to_string())?;
    if address != "127.0.0.1:5811" {
        return Err("Private5811 only".into());
    }
    let username = std::env::var("MENTOR_ACCOUNT").map_err(|e| e.to_string())?;
    if !username.starts_with("fb_") {
        return Err("Fixture account required".into());
    }
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos() as u64;
    let mut bridge = BridgeConfig::new()
        .receive::<LoginResponse>()
        .receive::<EnterWorldResponse>()
        .receive::<NewWorld>()
        .receive::<GroupInvitePrompt>()
        .receive::<PartyKeystoneSnapshot>()
        .connect(
            address
                .parse()
                .map_err(|e: std::net::AddrParseError| e.to_string())?,
            id,
        )?;
    bridge.send::<_, AuthChannel>(LoginRequest {
        token: None,
        username,
        password: "fbtest".into(),
    })?;
    let mut last = String::new();
    let deadline = Instant::now() + Duration::from_secs(900);
    while Instant::now() < deadline {
        for event in bridge.drain_events()? {
            match event {
                Event::Message(message) => receive(&bridge, message, &directory)?,
                Event::Disconnected(reason) => {
                    return Err(format!("Peer disconnected: {reason:?}"));
                }
                _ => {}
            }
        }
        let command = directory.join("peer-command");
        if command.exists() {
            let command = fs::read_to_string(command).map_err(|e| e.to_string())?;
            if command != last {
                last = command.clone();
                match command.trim() {
                    "invite" => bridge.send::<_, GroupChannel>(GroupInviteIntent {
                        name: "Mentorone".into(),
                    })?,
                    "leave" => bridge.send::<_, GroupChannel>(LeaveGroup)?,
                    "raid" => bridge.send::<_, GroupChannel>(ConvertGroupToRaid)?,
                    "party" => bridge.send::<_, GroupChannel>(ConvertGroupToParty)?,
                    "quit" => return Ok(()),
                    other => return Err(format!("Unknown peer command {other}")),
                }
            }
        }
        thread::sleep(Duration::from_millis(25));
    }
    Err("Peer command deadline".into())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("MENTOR_PEER_ERROR {error}");
        std::process::exit(1);
    }
}
