//! Private UDP5911 summoned-level proof: real bag learning/use and replicated source fields.
//! Default Scarab control; explicit `podium`/`shrine` arguments probe wild initial stats.
use game_engine_network::{BridgeConfig, Event, NetworkBridge, ProtocolMessage, replica::Replica};
use shared::components::{Health, Npc, Position, UnitLevel, UnitSummonedBy};
use shared::protocol::*;
use std::{
    fs, thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const DEADLINE: Duration = Duration::from_secs(120);

#[derive(Clone, Copy, Debug)]
enum SourceSummon {
    Scarab,
    Podium,
    Shrine,
}

impl SourceSummon {
    fn item(self) -> u32 {
        match self {
            Self::Scarab => 64881,
            Self::Podium => 239018,
            Self::Shrine => 245942,
        }
    }

    fn entry(self) -> u32 {
        match self {
            Self::Scarab => 51113,
            Self::Podium => 240220,
            Self::Shrine => 246037,
        }
    }

    fn count_range(self) -> std::ops::RangeInclusive<usize> {
        match self {
            Self::Scarab => 20..=30,
            Self::Podium | Self::Shrine => 1..=1,
        }
    }

    fn expected_health(self, level: u8) -> Result<Option<f32>, String> {
        match (self, level) {
            // Scarab full-stat parity is not asserted by this fixture.
            (Self::Scarab, _) => Ok(None),
            (Self::Podium, 20) => Ok(Some(48066.0)),
            (Self::Podium, 30) => Ok(Some(87576.0)),
            (Self::Shrine, 30) => Ok(Some(4379.0)),
            _ => Err(format!("unsupported source level {level} for {self:?}")),
        }
    }
}

struct Peer {
    source: SourceSummon,
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
                .any(|toy| toy.item_id == self.source.item() && toy.learned)
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
                    .find(|slot| slot.item.item_id == self.source.item())
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

    fn owned_summons(&self) -> Vec<(u8, u64, f32)> {
        self.replica
            .units()
            .filter_map(|unit| {
                if unit.get::<Npc>()?.template_id != self.source.entry()
                    || unit.get::<UnitSummonedBy>()?.0 != self.player?
                {
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

fn connect(account: String, source: SourceSummon) -> Result<Peer, String> {
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
        source,
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

fn wait_for_owner(peer: &mut Peer, expected_level: u8) -> Result<u64, String> {
    wait(peer, "selected owner and collections", |peer| {
        peer.player.and_then(|id| peer.replica.unit(id)).is_some()
            && peer.inventory.is_some()
            && peer.catalog.is_some()
    })?;
    let player = peer.player.ok_or("missing owner")?;
    let level = peer
        .replica
        .unit(player)
        .and_then(|unit| unit.get::<UnitLevel>())
        .ok_or("owner level absent")?
        .0;
    if level != expected_level {
        return Err(format!("owner level {level}, expected {expected_level}"));
    }
    Ok(player)
}

fn learn_and_activate(peer: &mut Peer) -> Result<(), String> {
    peer.send_learning()?;
    wait(peer, "actual bag learning", Peer::learned)?;
    let destination = source_destination(peer)?;
    peer.bridge.send::<_, CollectionChannel>(UseToy {
        item_id: peer.source.item(),
        destination,
        witness: None,
    })?;
    wait(peer, "replicated source summons", |peer| {
        peer.owned_summons().len() >= *peer.source.count_range().start()
    })
}

fn source_destination(peer: &Peer) -> Result<Option<[f32; 3]>, String> {
    if !matches!(peer.source, SourceSummon::Shrine) {
        return Ok(None);
    }
    let player = peer.player.ok_or("missing cursor caster")?;
    let position = peer
        .replica
        .unit(player)
        .and_then(|unit| unit.get::<Position>())
        .ok_or("missing cursor caster position")?;
    Ok(Some([position.x + 3.0, position.y, position.z]))
}

fn prove_account(
    account: String,
    owner_level: u8,
    expected_level: u8,
    source: SourceSummon,
) -> Result<serde_json::Value, String> {
    let mut peer = connect(account.clone(), source)?;
    let player = wait_for_owner(&mut peer, owner_level)?;
    learn_and_activate(&mut peer)?;
    let actors = peer.owned_summons();
    let expected_health = verify_source_summons(source, &actors, player, expected_level)?;
    let viewer_levels = verify_viewer_levels(&peer, expected_level)?;
    Ok(
        serde_json::json!({"account": account, "player": player, "owner_level": owner_level,
        "expected_summon_level": expected_level, "source": format!("{source:?}"),
        "expected_health": expected_health, "actors": actors, "viewer_levels": viewer_levels,
        "learning": "actual UseItem then UseToy"}),
    )
}

fn verify_viewer_levels(peer: &Peer, native: u8) -> Result<Vec<serde_json::Value>, String> {
    use shared::level_scaling::{LevelScaling, level_for_viewer};
    let mut observations = Vec::new();
    for unit in peer.replica.units().filter(|unit| {
        unit.get::<Npc>()
            .is_some_and(|npc| npc.template_id == peer.source.entry())
            && unit.get::<UnitSummonedBy>().map(|owner| owner.0) == peer.player
    }) {
        let level = *unit.get::<UnitLevel>().ok_or("missing native level")?;
        let scaling = unit.get::<LevelScaling>();
        for viewer in [1, 20, 80] {
            let expected = if matches!(peer.source, SourceSummon::Shrine) {
                viewer.clamp(1, 30)
            } else {
                native
            };
            let actual = level_for_viewer(level, scaling, viewer);
            if actual != expected {
                return Err(format!(
                    "viewer{viewer}: level{actual}, expected{expected}; scaling{scaling:?}"
                ));
            }
            observations
                .push(serde_json::json!({"viewer": viewer, "level": actual, "scaling": scaling}));
        }
    }
    Ok(observations)
}

fn read_source_scenario() -> Result<SourceSummon, String> {
    let source = match std::env::args().nth(1).as_deref() {
        None | Some("scarab") => SourceSummon::Scarab,
        Some("podium") => SourceSummon::Podium,
        Some("shrine") => SourceSummon::Shrine,
        Some(other) => return Err(format!("unknown source scenario {other}")),
    };
    Ok(source)
}

fn verify_source_summons(
    source: SourceSummon,
    actors: &[(u8, u64, f32)],
    player: u64,
    expected_level: u8,
) -> Result<Option<f32>, String> {
    if !source.count_range().contains(&actors.len()) {
        return Err(format!("source count mismatch: {actors:?}"));
    }
    let expected_health = source.expected_health(expected_level)?;
    let wrong_identity = actors
        .iter()
        .any(|&(level, owner, health)| level != expected_level || owner != player || health <= 0.0);
    let wrong_health =
        expected_health.is_some_and(|expected| actors.iter().any(|actor| actor.2 != expected));
    if wrong_identity || wrong_health {
        return Err(format!(
            "source level/ownership/health mismatch: {actors:?}"
        ));
    }
    Ok(expected_health)
}

fn main() -> Result<(), String> {
    let source = read_source_scenario()?;
    let expected_at20 = if matches!(source, SourceSummon::Shrine) {
        30
    } else {
        20
    };
    let accounts = [
        std::env::var("TOYFX_LEVEL20_ACCOUNT").map_err(|e| e.to_string())?,
        std::env::var("TOYFX_LEVEL80_ACCOUNT").map_err(|e| e.to_string())?,
    ];
    let output = std::env::var("TOYFX_LEVEL_OUTPUT").map_err(|e| e.to_string())?;
    let proofs = vec![
        prove_account(accounts[0].clone(), 20, expected_at20, source)?,
        prove_account(accounts[1].clone(), 80, 30, source)?,
    ];
    fs::write(
        output,
        serde_json::to_vec_pretty(&proofs).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    println!(
        "PASS actual bag learning/UseToy: Northshire {source:?} initialized fields at owner20/80"
    );
    Ok(())
}
