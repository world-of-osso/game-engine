//! A real UDP online comparison target for the private5898 criteria proof.
use game_engine_network::{BridgeConfig, Event, ProtocolMessage};
use shared::protocol::*;
use std::{
    fs,
    path::Path,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

fn receive(
    bridge: &game_engine_network::NetworkBridge,
    message: ProtocolMessage,
    directory: &Path,
) -> Result<(), String> {
    if message.is::<LoginResponse>() {
        let reply = message
            .downcast::<LoginResponse>()
            .map_err(|_| "login reply type")?;
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
            .map_err(|_| "world reply type")?;
        if !reply.success {
            return Err(format!("Entry refused: {:?}", reply.error));
        }
        fs::write(
            directory.join("peer-ready.json"),
            serde_json::to_vec_pretty(&reply).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    } else if message.is::<NewWorld>() {
        bridge.send::<_, TransferChannel>(WorldPortAck)?;
    } else if message.is::<AchievementStateUpdate>() {
        let reply = message
            .downcast::<AchievementStateUpdate>()
            .map_err(|_| "achievement state type")?;
        fs::write(
            directory.join("peer-achievements.json"),
            serde_json::to_vec_pretty(&reply).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn run() -> Result<(), String> {
    let directory =
        std::path::PathBuf::from(std::env::var("ACHIEVEMENTS_RUN").map_err(|e| e.to_string())?);
    let address = std::env::var("GODOT_TEST_SERVER").map_err(|e| e.to_string())?;
    if address != "127.0.0.1:5898" {
        return Err("Private5898 only".into());
    }
    let username = std::env::var("ACHIEVEMENTS_PEER_ACCOUNT").map_err(|e| e.to_string())?;
    if !username.starts_with("fb_achcrit_") {
        return Err("Criteria fixture account required".into());
    }
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos() as u64;
    let mut bridge = BridgeConfig::new()
        .receive::<LoginResponse>()
        .receive::<EnterWorldResponse>()
        .receive::<NewWorld>()
        .receive::<AchievementStateUpdate>()
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
        if directory.join("peer-quit").exists() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(25));
    }
    Err("Peer deadline expired".into())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("ACHIEVEMENTS_PEER_ERROR {error}");
        std::process::exit(1);
    }
}
