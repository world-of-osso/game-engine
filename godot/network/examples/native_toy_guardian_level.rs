//! Private UDP5911 Scarab Storm proof: real bag learning/use and replicated guardian levels.
use game_engine_network::{BridgeConfig, Event, NetworkBridge, ProtocolMessage, replica::Replica};
use shared::components::{Health, Npc, UnitLevel, UnitSummonedBy};
use shared::protocol::*;
use std::{
    fs, thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const TOY: u32 = 64881;
const SCARAB: u32 = 51113;
const DEADLINE: Duration = Duration::from_secs(120);

struct Peer {
    bridge: NetworkBridge,
    replica: Replica,
    player: Option<u64>,
    inventory: Option<InventorySnapshot>,
    catalog: Option<ToyCollectionUpdate>,
}

impl Peer {
    fn receive_events(&mut self) -> Result<(), String> {
        for event in self.bridge.drain_events()? {
            match event {
                Event::Message(message) => self.receive_message(message)?,
                Event::ReplicationStarted(schema) => self.replica = Replica::new(schema),
                Event::Replication(batch) => {
                    self.replica.apply(batch).map_err(|e| e.to_string())?
                }
                Event::Disconnected(reason) => return Err(format!("disconnected: {reason:?}")),
                Event::ProtocolRejected(reason) => return Err(reason),
                _ => {}
            }
        }
        Ok(())
    }

    fn receive_message(&mut self, message: ProtocolMessage) -> Result<(), String> {
        if message.is::<LoginResponse>() {
            let reply = message
                .downcast::<LoginResponse>()
                .map_err(|_| "login type")?;
            if !reply.success {
                return Err(format!("login refused: {:?}", reply.error));
            }
            let character = reply.characters.first().ok_or("empty roster")?;
            self.bridge.send::<_, AuthChannel>(SelectCharacter {
                character_id: character.character_id,
            })?;
        } else if message.is::<EnterWorldResponse>() {
            let reply = message
                .downcast::<EnterWorldResponse>()
                .map_err(|_| "entry type")?;
            if !reply.success {
                return Err(format!("entry refused: {:?}", reply.error));
            }
            self.player = reply.player_entity;
        } else if message.is::<NewWorld>() {
            self.bridge.send::<_, TransferChannel>(WorldPortAck)?;
        } else {
            self.receive_collection(message)?;
        }
        Ok(())
    }

    fn receive_collection(&mut self, message: ProtocolMessage) -> Result<(), String> {
        if message.is::<InventorySnapshot>() {
            self.inventory = Some(
                message
                    .downcast::<InventorySnapshot>()
                    .map_err(|_| "inventory type")?,
            );
        } else if message.is::<ToyCollectionUpdate>() {
            self.catalog = Some(
                message
                    .downcast::<ToyCollectionUpdate>()
                    .map_err(|_| "catalog type")?,
            );
        } else if message.is::<ToyResult>() {
            let reply = message
                .downcast::<ToyResult>()
                .map_err(|_| "toy result type")?;
            if reply.error.is_some() {
                return Err(format!("toy refused: {reply:?}"));
            }
        } else if message.is::<CastFailed>() {
            return Err(format!(
                "cast failed: {:?}",
                message
                    .downcast::<CastFailed>()
                    .map_err(|_| "failure type")?
            ));
        }
        Ok(())
    }

    fn learned(&self) -> bool {
        self.catalog.as_ref().is_some_and(|catalog| {
            catalog
                .toys
                .iter()
                .any(|toy| toy.item_id == TOY && toy.learned)
        })
    }

    fn send_learning(&self) -> Result<(), String> {
        if self.learned() {
            return Err("fixture requires a fresh unlearned account".into());
        }
        let inventory = self.inventory.as_ref().ok_or("missing inventory")?;
        let location = inventory
            .bags
            .iter()
            .find_map(|bag| {
                bag.items
                    .iter()
                    .find(|slot| slot.item.item_id == TOY)
                    .map(|slot| ItemLocation::Bag {
                        bag: bag.bag,
                        slot: slot.slot,
                    })
            })
            .ok_or("source physical toy absent")?;
        self.bridge.send::<_, InventoryChannel>(UseItem {
            location,
            target: None,
        })
    }

    fn scarabs(&self) -> Vec<(u8, u64, f32)> {
        self.replica
            .units()
            .filter_map(|unit| {
                if unit.get::<Npc>()?.template_id != SCARAB {
                    return None;
                }
                Some((
                    unit.get::<UnitLevel>()?.0,
                    unit.get::<UnitSummonedBy>()?.0,
                    unit.get::<Health>()?.max,
                ))
            })
            .collect()
    }
}

fn connect(account: String) -> Result<Peer, String> {
    let address = std::env::var("GODOT_TEST_SERVER").map_err(|e| e.to_string())?;
    if address != "127.0.0.1:5911" || !account.starts_with("fb_") {
        return Err("private5911/fb_* required".into());
    }
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos() as u64;
    let bridge = BridgeConfig::new()
        .receive::<LoginResponse>()
        .receive::<EnterWorldResponse>()
        .receive::<NewWorld>()
        .receive::<InventorySnapshot>()
        .receive::<ToyCollectionUpdate>()
        .receive::<ToyResult>()
        .receive::<CastFailed>()
        .connect(
            address
                .parse()
                .map_err(|e: std::net::AddrParseError| e.to_string())?,
            id,
        )?;
    bridge.send::<_, AuthChannel>(LoginRequest {
        token: None,
        username: account,
        password: "fbtest".into(),
    })?;
    Ok(Peer {
        bridge,
        replica: Replica::default(),
        player: None,
        inventory: None,
        catalog: None,
    })
}

fn wait(peer: &mut Peer, label: &str, predicate: impl Fn(&Peer) -> bool) -> Result<(), String> {
    let deadline = Instant::now() + DEADLINE;
    while Instant::now() < deadline {
        peer.receive_events()?;
        if predicate(peer) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(25));
    }
    Err(format!("{label}: actual UDP deadline"))
}

fn prove_account(
    account: String,
    owner_level: u8,
    expected_level: u8,
) -> Result<serde_json::Value, String> {
    let mut peer = connect(account.clone())?;
    wait(&mut peer, "selected owner and collections", |peer| {
        peer.player.and_then(|id| peer.replica.unit(id)).is_some()
            && peer.inventory.is_some()
            && peer.catalog.is_some()
    })?;
    let player = peer.player.ok_or("missing owner")?;
    let actual_owner_level = peer
        .replica
        .unit(player)
        .and_then(|unit| unit.get::<UnitLevel>())
        .ok_or("owner level absent")?
        .0;
    if actual_owner_level != owner_level {
        return Err(format!(
            "owner level {actual_owner_level}, expected {owner_level}"
        ));
    }
    peer.send_learning()?;
    wait(&mut peer, "actual bag learning", Peer::learned)?;
    peer.bridge.send::<_, CollectionChannel>(UseToy {
        item_id: TOY,
        destination: None,
        witness: None,
    })?;
    wait(&mut peer, "replicated source Scarabs", |peer| {
        peer.scarabs().len() >= 20
    })?;
    let actors = peer.scarabs();
    if !(20..=30).contains(&actors.len())
        || actors.iter().any(|&(level, owner, health)| {
            level != expected_level || owner != player || health <= 0.0
        })
    {
        return Err(format!(
            "source level/ownership/health mismatch: {actors:?}"
        ));
    }
    Ok(
        serde_json::json!({"account": account, "player": player, "owner_level": actual_owner_level,
        "expected_scarab_level": expected_level, "actors": actors, "learning": "actual UseItem then UseToy"}),
    )
}

fn main() -> Result<(), String> {
    let accounts = [
        std::env::var("TOYFX_LEVEL20_ACCOUNT").map_err(|e| e.to_string())?,
        std::env::var("TOYFX_LEVEL80_ACCOUNT").map_err(|e| e.to_string())?,
    ];
    let output = std::env::var("TOYFX_LEVEL_OUTPUT").map_err(|e| e.to_string())?;
    let proofs = vec![
        prove_account(accounts[0].clone(), 20, 20)?,
        prove_account(accounts[1].clone(), 80, 30)?,
    ];
    fs::write(
        output,
        serde_json::to_vec_pretty(&proofs).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    println!("PASS actual bag learning/UseToy: Northshire Scarab guardian20/30 at owner20/80");
    Ok(())
}
