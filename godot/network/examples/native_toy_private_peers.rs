//! Real private5911 UDP UseToy: source-private73781 vs public66768, two accounts.
use game_engine_network::{BridgeConfig, Event, NetworkBridge, ProtocolMessage, replica::Replica};
use shared::components::{Health, Npc, Position};
use shared::protocol::*;
use std::{
    fs,
    path::PathBuf,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const PRIVATE_TOY: u32 = 104331;
const PRIVATE_ENTRY: u32 = 73781;
const PUBLIC_TOY: u32 = 89869;
const PUBLIC_ENTRY: u32 = 66768;

struct Peer {
    bridge: NetworkBridge,
    replica: Replica,
    player: Option<u64>,
    map: Option<String>,
    catalog: Option<ToyCollectionUpdate>,
    inventory: Option<InventorySnapshot>,
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
                Event::Disconnected(reason) => {
                    return Err(format!("private toy peer disconnected: {reason:?}"));
                }
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
            self.bridge.send::<_, AuthChannel>(SelectCharacter {
                character_id: reply.characters.first().ok_or("empty roster")?.character_id,
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
            message
                .downcast::<NewWorld>()
                .map_err(|_| "transfer type")?;
            self.bridge.send::<_, TransferChannel>(WorldPortAck)?;
        } else {
            self.receive_toy_message(message)?;
        }
        Ok(())
    }

    fn receive_toy_message(&mut self, message: ProtocolMessage) -> Result<(), String> {
        if message.is::<LoadTerrain>() {
            self.map = Some(
                message
                    .downcast::<LoadTerrain>()
                    .map_err(|_| "terrain type")?
                    .map_name,
            );
        } else if message.is::<InventorySnapshot>() {
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
                return Err(format!(
                    "UseToy{} refused: {:?}",
                    reply.item_id, reply.error
                ));
            }
        } else if message.is::<CastFailed>() {
            let reply = message
                .downcast::<CastFailed>()
                .map_err(|_| "cast failure type")?;
            return Err(format!("actual toy cast failed: {reply:?}"));
        }
        Ok(())
    }

    fn position(&self) -> Option<Position> {
        let id = self.player?;
        let unit = self.replica.unit(id)?;
        if unit.get::<Health>()?.current <= 0.0 {
            return None;
        }
        unit.get::<Position>().copied()
    }

    fn sees_entry(&self, entry: u32) -> bool {
        self.replica.units().any(|unit| {
            unit.get::<Npc>()
                .is_some_and(|npc| npc.template_id == entry)
        })
    }

    fn learned(&self, item: u32) -> bool {
        self.catalog.as_ref().is_some_and(|catalog| {
            catalog
                .toys
                .iter()
                .any(|toy| toy.item_id == item && toy.learned)
        })
    }

    fn learn_toy(&self, item: u32) -> Result<(), String> {
        if self.learned(item) {
            return Ok(());
        }
        let bags = &self.inventory.as_ref().ok_or("inventory not ready")?.bags;
        let location = bags
            .iter()
            .find_map(|bag| {
                bag.items
                    .iter()
                    .find(|slot| slot.item.item_id == item)
                    .map(|slot| ItemLocation::Bag {
                        bag: bag.bag,
                        slot: slot.slot,
                    })
            })
            .ok_or_else(|| format!("source physical toy{item} absent"))?;
        self.bridge.send::<_, InventoryChannel>(UseItem {
            location,
            target: None,
        })
    }

    fn use_toy(&self, item: u32) -> Result<(), String> {
        let toy = self
            .catalog
            .as_ref()
            .ok_or("catalog not ready")?
            .toys
            .iter()
            .find(|toy| toy.item_id == item)
            .ok_or("source toy absent")?;
        if !toy.learned || toy.unavailable_reason.is_some() {
            return Err(format!("owned source toy unavailable: {toy:?}"));
        }
        let position = self.position().ok_or("living player replica absent")?;
        let destination =
            (item == PUBLIC_TOY).then_some([position.x + 3.0, position.y, position.z]);
        self.bridge.send::<_, CollectionChannel>(UseToy {
            item_id: item,
            destination,
            witness: None,
        })
    }
}

fn connect_peer(variable: &str, offset: u64) -> Result<Peer, String> {
    let address = std::env::var("GODOT_TEST_SERVER").map_err(|e| e.to_string())?;
    let account = std::env::var(variable).map_err(|e| e.to_string())?;
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
        .receive::<LoadTerrain>()
        .receive::<ToyCollectionUpdate>()
        .receive::<InventorySnapshot>()
        .receive::<ToyResult>()
        .receive::<CastFailed>()
        .connect(
            address
                .parse()
                .map_err(|e: std::net::AddrParseError| e.to_string())?,
            id.wrapping_add(offset),
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
        map: None,
        catalog: None,
        inventory: None,
    })
}

fn wait_peers(
    owner: &mut Peer,
    observer: &mut Peer,
    predicate: impl Fn(&Peer, &Peer) -> bool,
) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(120);
    while Instant::now() < deadline {
        owner.receive_events()?;
        observer.receive_events()?;
        if predicate(owner, observer) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(25));
    }
    Err("private toy peer readiness deadline".into())
}

fn require_same_map_and_reach(owner: &Peer, observer: &Peer) -> Result<(), String> {
    if owner.map.is_none() || owner.map != observer.map {
        return Err("same map required".into());
    }
    let a = owner.position().ok_or("owner position")?;
    let b = observer.position().ok_or("observer position")?;
    let distance = ((a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2)).sqrt();
    if distance > 10.0 {
        return Err(format!("observer must be near actual caster: {distance}yd"));
    }
    Ok(())
}

fn write_receipt(path: &PathBuf, owner: &Peer, observer: &Peer) -> Result<(), String> {
    let json = serde_json::json!({"owner_player": owner.player, "observer_player": observer.player,
        "owner_map": owner.map, "observer_map": observer.map,
        "owner_private": owner.sees_entry(PRIVATE_ENTRY), "observer_private": observer.sees_entry(PRIVATE_ENTRY),
        "owner_public": owner.sees_entry(PUBLIC_ENTRY), "observer_public": observer.sees_entry(PUBLIC_ENTRY)});
    fs::write(
        path,
        serde_json::to_vec_pretty(&json).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

fn run() -> Result<(), String> {
    let output = PathBuf::from(std::env::var("TOY_PRIVATE_RECEIPT").map_err(|e| e.to_string())?);
    if output.exists() {
        return Err("fresh receipt path required".into());
    }
    let mut owner = connect_peer("TOY_PRIVATE_OWNER", 0)?;
    let mut observer = connect_peer("TOY_PRIVATE_OBSERVER", 1)?;
    wait_peers(&mut owner, &mut observer, |a, b| {
        a.position().is_some()
            && b.position().is_some()
            && a.catalog.is_some()
            && a.inventory.is_some()
    })?;
    require_same_map_and_reach(&owner, &observer)?;
    if owner.sees_entry(PRIVATE_ENTRY) || observer.sees_entry(PRIVATE_ENTRY) {
        return Err("pre-existing private sign; no duplicate source cast".into());
    }
    owner.learn_toy(PUBLIC_TOY)?;
    wait_peers(&mut owner, &mut observer, |a, _| a.learned(PUBLIC_TOY))?;
    owner.use_toy(PUBLIC_TOY)?;
    wait_peers(&mut owner, &mut observer, |a, b| {
        a.sees_entry(PUBLIC_ENTRY) && b.sees_entry(PUBLIC_ENTRY)
    })?;
    owner.use_toy(PRIVATE_TOY)?;
    wait_peers(&mut owner, &mut observer, |a, _| {
        a.sees_entry(PRIVATE_ENTRY)
    })?;
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        owner.receive_events()?;
        observer.receive_events()?;
        if observer.sees_entry(PRIVATE_ENTRY) {
            write_receipt(&output, &owner, &observer)?;
            return Err(
                "source private Warning Sign leaked into observer's actual UDP replica".into(),
            );
        }
        thread::sleep(Duration::from_millis(25));
    }
    write_receipt(&output, &owner, &observer)?;
    println!("PASS actual source-private sign owner-only/public scarecrow both peers");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("FAIL: {error}");
        std::process::exit(1)
    }
}
