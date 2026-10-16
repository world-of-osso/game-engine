//! One real authenticated stationary enemy for private UDP5941 honor proofs.
//! No balance/level injection: damage is driven through the private realm's normal admin path.
use game_engine_network::{BridgeConfig, Event, ProtocolMessage};
use shared::protocol::*;
use std::{
    fs,
    path::PathBuf,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

fn main() -> Result<(), String> {
    let directory = PathBuf::from(std::env::var("HONOR_DIRECTORY").map_err(|e| e.to_string())?);
    let bridge = BridgeConfig::new()
        .receive::<LoginResponse>()
        .receive::<EnterWorldResponse>()
        .receive::<NewWorld>()
        .receive::<PvpStateUpdate>()
        .connect(
            "127.0.0.1:5941"
                .parse()
                .map_err(|e| format!("private endpoint: {e}"))?,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos() as u64,
        )?;
    bridge.send::<_, AuthChannel>(LoginRequest {
        token: None,
        username: "fb_pvpprog_enemy".into(),
        password: "fbtest".into(),
    })?;
    let deadline = Instant::now() + Duration::from_secs(1800);
    while Instant::now() < deadline {
        if directory.join("peer-stop").exists() {
            return bridge.stop();
        }
        for event in bridge.drain_events()? {
            match event {
                Event::Message(message) => receive(&bridge, &directory, message)?,
                Event::Disconnected(reason) => {
                    return Err(format!("enemy disconnected: {reason:?}"));
                }
                Event::ProtocolRejected(reason) => return Err(reason),
                _ => {}
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
    Err("private honor peer exceeded30minutes".into())
}

fn receive(
    bridge: &game_engine_network::NetworkBridge,
    directory: &std::path::Path,
    message: ProtocolMessage,
) -> Result<(), String> {
    if message.is::<LoginResponse>() {
        let reply = message
            .downcast::<LoginResponse>()
            .map_err(|_| "login type")?;
        if !reply.success {
            return Err(format!("enemy login: {:?}", reply.error));
        }
        bridge.send::<_, AuthChannel>(SelectCharacter {
            character_id: reply.characters.first().ok_or("empty roster")?.character_id,
        })?;
    } else if message.is::<EnterWorldResponse>() {
        let reply = message
            .downcast::<EnterWorldResponse>()
            .map_err(|_| "entry type")?;
        if !reply.success {
            return Err(format!("enemy entry: {:?}", reply.error));
        }
        fs::write(
            directory.join("enemy.json"),
            serde_json::to_vec_pretty(&reply).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        bridge.send::<_, PvpChannel>(QueryPvpStatus)?;
    } else if message.is::<NewWorld>() {
        bridge.send::<_, TransferChannel>(WorldPortAck)?;
    } else if message.is::<PvpStateUpdate>() {
        let reply = message
            .downcast::<PvpStateUpdate>()
            .map_err(|_| "pvp type")?;
        fs::write(
            directory.join("enemy-pvp.json"),
            serde_json::to_vec_pretty(&reply).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
