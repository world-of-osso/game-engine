//! Actual private5911 UDP group commands and learned source toy162979/80682.
//! TC a352b1fa captures a group GUID; joining/leaving must alter actual replicas.
use game_engine_network::{BridgeConfig, Event, NetworkBridge, ProtocolMessage, replica::Replica};
use shared::components::{Health, Npc, Position};
use shared::protocol::*;
use std::{
    fs,
    path::PathBuf,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const TOY: u32 = 112324;
const ENTRY: u32 = 80682;

struct Peer {
    bridge: NetworkBridge,
    replica: Replica,
    player: Option<u64>,
    name: String,
    map: Option<String>,
    catalog: Option<ToyCollectionUpdate>,
    inventory: Option<InventorySnapshot>,
    roster: Vec<String>,
    invited: bool,
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
                    return Err(format!("{} disconnected: {reason:?}", self.name));
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
            let character = reply.characters.first().ok_or("empty roster")?;
            self.name = character.name.clone();
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
            message
                .downcast::<NewWorld>()
                .map_err(|_| "transfer type")?;
            self.bridge.send::<_, TransferChannel>(WorldPortAck)?;
        } else {
            self.receive_state(message)?;
        }
        Ok(())
    }

    fn receive_state(&mut self, message: ProtocolMessage) -> Result<(), String> {
        if message.is::<LoadTerrain>() {
            self.map = Some(
                message
                    .downcast::<LoadTerrain>()
                    .map_err(|_| "terrain type")?
                    .map_name,
            );
        } else if message.is::<ToyCollectionUpdate>() {
            self.catalog = Some(
                message
                    .downcast::<ToyCollectionUpdate>()
                    .map_err(|_| "catalog type")?,
            );
        } else if message.is::<InventorySnapshot>() {
            self.inventory = Some(
                message
                    .downcast::<InventorySnapshot>()
                    .map_err(|_| "inventory type")?,
            );
        } else if message.is::<GroupRosterSnapshot>() {
            self.roster = message
                .downcast::<GroupRosterSnapshot>()
                .map_err(|_| "roster type")?
                .members
                .into_iter()
                .map(|member| member.name)
                .collect();
        } else if message.is::<GroupInvitePrompt>() {
            message
                .downcast::<GroupInvitePrompt>()
                .map_err(|_| "invite type")?;
            self.invited = true;
        } else if message.is::<ToyResult>() {
            let reply = message
                .downcast::<ToyResult>()
                .map_err(|_| "toy result type")?;
            if reply.error.is_some() {
                return Err(format!("actual toy refused: {reply:?}"));
            }
        } else if message.is::<CastFailed>() {
            return Err(format!(
                "actual cast failed: {:?}",
                message
                    .downcast::<CastFailed>()
                    .map_err(|_| "failure type")?
            ));
        }
        Ok(())
    }

    fn position(&self) -> Option<Position> {
        let unit = self.replica.unit(self.player?)?;
        if unit.get::<Health>()?.current <= 0.0 {
            return None;
        }
        unit.get::<Position>().copied()
    }

    fn sees_toy(&self) -> bool {
        self.replica.units().any(|unit| {
            unit.get::<Npc>()
                .is_some_and(|npc| npc.template_id == ENTRY)
        })
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
            return Ok(());
        }
        let bags = &self.inventory.as_ref().ok_or("inventory not ready")?.bags;
        let location = bags
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
        .receive::<GroupRosterSnapshot>()
        .receive::<GroupInvitePrompt>()
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
        name: String::new(),
        map: None,
        catalog: None,
        inventory: None,
        roster: Vec::new(),
        invited: false,
    })
}

fn wait_peers(
    peers: &mut [Peer],
    label: &str,
    predicate: impl Fn(&[Peer]) -> bool,
) -> Result<(), String> {
    wait_peers_for(peers, label, Duration::from_secs(120), predicate)
}

fn wait_peers_for(
    peers: &mut [Peer],
    label: &str,
    timeout: Duration,
    predicate: impl Fn(&[Peer]) -> bool,
) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        for peer in peers.iter_mut() {
            peer.receive_events()?;
        }
        if predicate(peers) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(25));
    }
    Err(format!("{label}: actual UDP deadline"))
}

fn join_group(peers: &mut [Peer], member: usize) -> Result<(), String> {
    peers[member].invited = false;
    peers[0].bridge.send::<_, GroupChannel>(GroupInviteIntent {
        name: peers[member].name.clone(),
    })?;
    wait_peers(peers, "invite", |peers| peers[member].invited)?;
    peers[member]
        .bridge
        .send::<_, GroupChannel>(RespondGroupInvite { accept: true })?;
    wait_peers(peers, "accepted roster", |peers| {
        peers[member].roster.contains(&peers[0].name)
    })
}

fn require_same_map_and_reach(peers: &[Peer]) -> Result<(), String> {
    let owner = peers[0].position().ok_or("owner position")?;
    for peer in peers.iter().skip(1) {
        if peer.map.is_none() || peer.map != peers[0].map {
            return Err("same map required".into());
        }
        let p = peer.position().ok_or("peer position")?;
        let distance =
            ((owner.x - p.x).powi(2) + (owner.y - p.y).powi(2) + (owner.z - p.z).powi(2)).sqrt();
        if distance > 10.0 {
            return Err(format!("peer {} not nearby: {distance}yd", peer.name));
        }
        if peer
            .replica
            .unit(peers[0].player.ok_or("owner id")?)
            .is_none()
        {
            return Err("public owner replica missing".into());
        }
    }
    Ok(())
}

fn write_phase(
    path: &PathBuf,
    phases: &mut Vec<serde_json::Value>,
    label: &str,
    peers: &[Peer],
    start: Instant,
) -> Result<(), String> {
    phases.push(serde_json::json!({"phase":label,"elapsed_ms":start.elapsed().as_millis(),
        "peers":peers.iter().map(|peer| serde_json::json!({"name":peer.name,"player":peer.player,
            "roster":peer.roster,"sees_source_entry":peer.sees_toy(),"position":peer.position()})).collect::<Vec<_>>()}));
    fs::write(
        path,
        serde_json::to_vec_pretty(phases).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

fn observe_group_visibility(peers: &mut [Peer], expected: [bool; 4]) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        for peer in peers.iter_mut() {
            peer.receive_events()?;
        }
        for (peer, visible) in peers.iter().zip(expected) {
            if peer.sees_toy() != visible {
                return Err(format!(
                    "{}: expected source visibility {visible}",
                    peer.name
                ));
            }
        }
        thread::sleep(Duration::from_millis(25));
    }
    Ok(())
}

fn spawn_source_group_summon(peers: &mut [Peer]) -> Result<(), String> {
    wait_peers(peers, "selected living replicas/catalogs", |peers| {
        peers.iter().all(|peer| {
            peer.position().is_some() && peer.catalog.is_some() && peer.inventory.is_some()
        })
    })?;
    require_same_map_and_reach(peers)?;
    if peers.iter().any(Peer::sees_toy) {
        return Err("pre-existing source summon".into());
    }
    join_group(peers, 1)?;
    join_group(peers, 2)?;
    peers[0].send_learning()?;
    wait_peers(peers, "real bag learning", |peers| peers[0].learned())?;
    peers[0].bridge.send::<_, CollectionChannel>(UseToy {
        item_id: TOY,
        destination: None,
        witness: None,
    })?;
    wait_peers(peers, "source group replica", |peers| {
        peers[..3].iter().all(Peer::sees_toy)
    })?;
    observe_group_visibility(peers, [true, true, true, false])
}

fn prove_roster_visibility(
    peers: &mut [Peer],
    output: &PathBuf,
    phases: &mut Vec<serde_json::Value>,
    start: Instant,
) -> Result<(), String> {
    peers[1].bridge.send::<_, GroupChannel>(LeaveGroup)?;
    wait_peers(peers, "stationary member removal", |peers| {
        !peers[1].sees_toy() && peers[1].roster.is_empty()
    })?;
    observe_group_visibility(peers, [true, false, true, false])?;
    write_phase(output, phases, "member-left", peers, start)?;
    join_group(peers, 3)?;
    wait_peers(peers, "stationary member addition", |peers| {
        peers[3].sees_toy()
    })?;
    observe_group_visibility(peers, [true, false, true, true])?;
    write_phase(output, phases, "stranger-joined", peers, start)
}

fn prove_captured_group(peers: &mut [Peer]) -> Result<(), String> {
    peers[0].bridge.send::<_, GroupChannel>(LeaveGroup)?;
    wait_peers(peers, "creator removal", |peers| {
        !peers[0].sees_toy() && peers[0].roster.is_empty()
    })?;
    join_group(peers, 1)?;
    observe_group_visibility(peers, [false, false, true, true])
}

fn run() -> Result<(), String> {
    let output = PathBuf::from(std::env::var("TOY_GROUP_RECEIPT").map_err(|e| e.to_string())?);
    if output.exists() {
        return Err("fresh receipt required".into());
    }
    let start = Instant::now();
    let mut phases = Vec::new();
    let mut peers = [
        connect_peer("TOY_GROUP_OWNER", 0)?,
        connect_peer("TOY_GROUP_MEMBER", 1)?,
        connect_peer("TOY_GROUP_ANCHOR", 2)?,
        connect_peer("TOY_GROUP_STRANGER", 3)?,
    ];
    spawn_source_group_summon(&mut peers)?;
    write_phase(&output, &mut phases, "initial-group", &peers, start)?;
    prove_roster_visibility(&mut peers, &output, &mut phases, start)?;
    prove_captured_group(&mut peers)?;
    write_phase(&output, &mut phases, "creator-new-group", &peers, start)?;
    wait_peers_for(
        &mut peers,
        "natural source180s expiry",
        Duration::from_secs(240),
        |peers| peers.iter().all(|peer| !peer.sees_toy()),
    )?;
    write_phase(&output, &mut phases, "natural-expiry", &peers, start)?;
    println!("PASS actual UDP source group-private join/leave/captured-group/natural expiry");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("FAIL: {error}");
        std::process::exit(1);
    }
}
