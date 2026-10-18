//! Source-minimum real-UDP account peers alongside one native Godot client.
//! No simulated match state, synthetic roster or server shortcuts.
use game_engine_network::{BridgeConfig, Event, NetworkBridge, ProtocolMessage, replica::Replica};
#[path = "native_battleground_peers/sight.rs"]
mod sight;
use shared::battleground::BgTeam;
use shared::protocol::*;
use std::{
    cell::RefCell,
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
const PRIVATE_PROOF_SUPPORT_COUNT: usize = 19;

struct Peer {
    index: usize,
    username: String,
    character_name: String,
    sight: RefCell<Option<sight::FixtureSight>>,
    battleground_id: u32,
    bridge: NetworkBridge,
    entered: bool,
    map: Option<u32>,
    snapshot: Option<PvpSnapshot>,
    group_roster: Option<GroupRosterSnapshot>,
    group_invite: Option<GroupInvitePrompt>,
    group_notice: Option<GroupCommandResponse>,
    replica: Replica,
    inventory: Option<InventorySnapshot>,
    equipment: Option<EquipmentSnapshot>,
    inventory_delta: Option<InventoryDelta>,
    traits: Option<TraitConfigSnapshot>,
    trait_result: Option<TraitCommitResult>,
    known_spells: Vec<u32>,
    combat_log: Vec<CombatLogEvent>,
}
impl Peer {
    fn receive(&mut self, message: ProtocolMessage) -> Result<(), String> {
        if message.is::<GroupRosterSnapshot>()
            || message.is::<GroupInvitePrompt>()
            || message.is::<GroupCommandResponse>()
            || message.is::<GroupInviteCancelled>()
        {
            return self.receive_group(message);
        }
        let inventory_receipt = message.is::<InventorySnapshot>()
            || message.is::<EquipmentSnapshot>()
            || message.is::<InventoryDelta>()
            || message.is::<InventoryError>();
        if inventory_receipt {
            return self.receive_inventory(message);
        }
        let spellbook_receipt = message.is::<KnownSpellsSnapshot>()
            || message.is::<SpecializationChanged>()
            || message.is::<SpellsLearned>()
            || message.is::<SpellsUnlearned>();
        if spellbook_receipt {
            return self.receive_spellbook(message);
        }
        if message.is::<CombatLogEvent>() {
            let event = message
                .downcast::<CombatLogEvent>()
                .map_err(|_| "combat log type")?;
            println!("{} combat log {event:?}", self.username);
            self.combat_log.push(event);
        } else if message.is::<TraitConfigSnapshot>() {
            let snapshot = message
                .downcast::<TraitConfigSnapshot>()
                .map_err(|_| "trait config type")?;
            println!("{} trait config {snapshot:?}", self.username);
            self.traits = Some(snapshot);
        } else if message.is::<TraitCommitResult>() {
            let result = message
                .downcast::<TraitCommitResult>()
                .map_err(|_| "trait commit type")?;
            println!("{} trait result {result:?}", self.username);
            self.trait_result = Some(result);
        } else if message.is::<TraitLoadoutsSnapshot>() {
            let snapshot = message
                .downcast::<TraitLoadoutsSnapshot>()
                .map_err(|_| "trait loadouts type")?;
            println!("{} trait loadouts {snapshot:?}", self.username);
        } else if message.is::<AttackStart>() {
            let receipt = message
                .downcast::<AttackStart>()
                .map_err(|_| "attack start type")?;
            println!("{} attack start {receipt:?}", self.username);
        } else if message.is::<AttackStopped>() {
            let receipt = message
                .downcast::<AttackStopped>()
                .map_err(|_| "attack stop type")?;
            println!("{} attack stopped {receipt:?}", self.username);
        } else if message.is::<CastFailed>() {
            let receipt = message
                .downcast::<CastFailed>()
                .map_err(|_| "cast failure type")?;
            println!("{} cast failed {receipt:?}", self.username);
        } else if message.is::<LoginResponse>() {
            let reply = message
                .downcast::<LoginResponse>()
                .map_err(|_| "login type")?;
            if !reply.success {
                return Err(format!("{} login: {:?}", self.username, reply.error));
            }
            let character = reply.characters.first().ok_or("Empty roster")?;
            self.character_name = character.name.clone();
            self.bridge.send::<_, AuthChannel>(SelectCharacter {
                character_id: character.character_id,
            })?;
        } else if message.is::<EnterWorldResponse>() {
            let reply = message
                .downcast::<EnterWorldResponse>()
                .map_err(|_| "entry type")?;
            if !reply.success {
                return Err(format!("world entry: {:?}", reply.error));
            }
            self.entered = true;
            self.bridge.send::<_, PvpChannel>(QueryPvpStatus)?;
        } else if message.is::<NewWorld>() {
            let destination = message
                .downcast::<NewWorld>()
                .map_err(|_| "transfer type")?;
            self.map = Some(destination.map_id);
            self.bridge.send::<_, TransferChannel>(WorldPortAck)?;
            println!("{} WorldPortAck map{}", self.username, destination.map_id);
        } else if message.is::<PvpStateUpdate>() {
            let update = message
                .downcast::<PvpStateUpdate>()
                .map_err(|_| "pvp type")?;
            if let Some(error) = update.error {
                return Err(format!("{} PvP: {error}", self.username));
            }
            if let Some(snapshot) = update.snapshot {
                self.snapshot = Some(snapshot);
            }
        }
        Ok(())
    }
    fn receive_events(&mut self) -> Result<(), String> {
        for event in self.bridge.drain_events()? {
            match event {
                Event::Message(message) => self.receive(message)?,
                Event::ReplicationStarted(schema) => self.replica = Replica::new(schema),
                Event::Replication(batch) => self
                    .replica
                    .apply(batch)
                    .map_err(|error| error.to_string())?,
                Event::Disconnected(reason) => {
                    return Err(format!("{} disconnected: {reason:?}", self.username));
                }
                Event::ProtocolRejected(reason) => return Err(reason),
                _ => {}
            }
        }
        Ok(())
    }

    fn receive_spellbook(&mut self, message: ProtocolMessage) -> Result<(), String> {
        if message.is::<KnownSpellsSnapshot>() {
            let spells = message
                .downcast::<KnownSpellsSnapshot>()
                .map_err(|_| "known spells type")?;
            println!("{} known spells {spells:?}", self.username);
            self.known_spells = spells.spells;
        } else if message.is::<SpecializationChanged>() {
            let spec = message
                .downcast::<SpecializationChanged>()
                .map_err(|_| "specialization type")?;
            println!("{} specialization {spec:?}", self.username);
        } else if message.is::<SpellsLearned>() {
            let spells = message
                .downcast::<SpellsLearned>()
                .map_err(|_| "learned spells type")?;
            println!("{} learned spells {spells:?}", self.username);
            self.known_spells.extend(spells.spells);
            self.known_spells.sort_unstable();
            self.known_spells.dedup();
        } else {
            let spells = message
                .downcast::<SpellsUnlearned>()
                .map_err(|_| "unlearned spells type")?;
            println!("{} unlearned spells {spells:?}", self.username);
            self.known_spells
                .retain(|spell| !spells.spells.contains(spell));
        }
        Ok(())
    }

    fn receive_group(&mut self, message: ProtocolMessage) -> Result<(), String> {
        if message.is::<GroupRosterSnapshot>() {
            self.group_roster = Some(
                message
                    .downcast::<GroupRosterSnapshot>()
                    .map_err(|_| "group roster type")?,
            );
        } else if message.is::<GroupInvitePrompt>() {
            self.group_invite = Some(
                message
                    .downcast::<GroupInvitePrompt>()
                    .map_err(|_| "group invitation type")?,
            );
        } else if message.is::<GroupCommandResponse>() {
            self.group_notice = Some(
                message
                    .downcast::<GroupCommandResponse>()
                    .map_err(|_| "group notice type")?,
            );
        } else {
            message
                .downcast::<GroupInviteCancelled>()
                .map_err(|_| "group cancellation type")?;
            self.group_invite = None;
        }
        Ok(())
    }

    fn receive_inventory(&mut self, message: ProtocolMessage) -> Result<(), String> {
        if message.is::<InventorySnapshot>() {
            self.inventory = Some(
                message
                    .downcast::<InventorySnapshot>()
                    .map_err(|_| "inventory snapshot type")?,
            );
        } else if message.is::<EquipmentSnapshot>() {
            self.equipment = Some(
                message
                    .downcast::<EquipmentSnapshot>()
                    .map_err(|_| "equipment snapshot type")?,
            );
        } else if message.is::<InventoryDelta>() {
            self.inventory_delta = Some(
                message
                    .downcast::<InventoryDelta>()
                    .map_err(|_| "inventory delta type")?,
            );
        } else {
            let error = message
                .downcast::<InventoryError>()
                .map_err(|_| "unexpected inventory receipt type")?;
            return Err(format!("{} inventory refusal: {error:?}", self.username));
        }
        Ok(())
    }

    fn action(&self, action: &str) -> Result<(), String> {
        let Some(action) = select_peer_action(self.index, action)? else {
            return Ok(());
        };
        if !self.entered {
            return Err(format!("{} has not entered", self.username));
        }
        self.send_action(action)
    }

    fn send_action(&self, action: &str) -> Result<(), String> {
        match action {
            "group-accept" => {
                self.group_invite
                    .as_ref()
                    .ok_or("no received group invitation")?;
                self.bridge
                    .send::<_, GroupChannel>(RespondGroupInvite { accept: true })
            }
            _ if action.starts_with("group-invite:") => {
                let name = action
                    .strip_prefix("group-invite:")
                    .expect("matched command");
                if name.is_empty() {
                    return Err("group invite requires a character name".into());
                }
                self.bridge
                    .send::<_, GroupChannel>(GroupInviteIntent { name: name.into() })
            }
            "queue" => self.bridge.send::<_, PvpChannel>(QueueForBattleground {
                battleground_id: self.battleground_id,
            }),
            "accept" | "decline" => {
                self.bridge
                    .send::<_, PvpChannel>(RespondToBattlegroundInvite {
                        match_id: self.battleground()?.match_id,
                        accepted: action == "accept",
                    })
            }
            "leave" => self.bridge.send::<_, PvpChannel>(LeaveBattleground {
                match_id: self.battleground()?.match_id,
            }),
            _ => self.send_gameplay_action(action),
        }
    }

    fn send_gameplay_action(&self, action: &str) -> Result<(), String> {
        match action {
            "attack-stop" => self.bridge.send::<_, CombatChannel>(AttackStop),
            _ if action.starts_with("attack:") => self.attack_source(action),
            _ if action.starts_with("equip:") => {
                self.bridge.send::<_, InventoryChannel>(EquipItem {
                    from: parse_bag_location(action, "equip")?,
                })
            }
            _ if action.starts_with("unequip-offhand:") => {
                self.bridge.send::<_, InventoryChannel>(SwapItem {
                    from: ItemLocation::Equipment(EquipmentSlot::OffHand),
                    to: parse_bag_location(action, "unequip-offhand")?,
                })
            }
            _ if action.starts_with("cast:") => self.send_spell_cast(action),
            _ if action.starts_with("traits:") => self.send_trait_commit(action),
            _ if action.starts_with("spec:") => self
                .bridge
                .send::<_, TalentChannel>(parse_specialization(action)?),
            _ => Err(format!("Unknown command {action}")),
        }
    }

    fn send_trait_commit(&self, action: &str) -> Result<(), String> {
        let path = action
            .strip_prefix("traits:")
            .ok_or("expected traits:<JSONfile>")?;
        let json = fs::read_to_string(path)
            .map_err(|error| format!("Read trait request {path}: {error}"))?;
        let request = parse_trait_commit(&json)?;
        self.bridge.send::<_, TalentChannel>(request)
    }

    fn send_spell_cast(&self, action: &str) -> Result<(), String> {
        let mut request = parse_spell_cast(action)?;
        let map = self.map.ok_or("peer has not received its current map")?;
        let mut sight = self.sight.borrow_mut();
        if sight.is_none() {
            let dir =
                PathBuf::from(std::env::var("BG_LOS_DIR").map_err(|_| "casts require BG_LOS_DIR")?);
            *sight = Some(sight::FixtureSight::load(&dir)?);
        }
        request.witness = Some(sight.as_mut().expect("loaded source sight").find_witness(
            &self.replica,
            &self.character_name,
            request.target_entity.expect("parsed target"),
            map,
        )?);
        self.bridge.send::<_, CombatChannel>(request)
    }

    fn battleground(&self) -> Result<&BattlegroundSnapshot, String> {
        self.snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.battleground.as_ref())
            .ok_or_else(|| "No authoritative battleground".into())
    }

    fn attack_source(&self, action: &str) -> Result<(), String> {
        let (entry, team) = parse_source_attack(action)?;
        let bg = self.battleground()?;
        if bg.team != team {
            return Ok(());
        }
        if bg.phase != shared::battleground::BgPhase::InProgress || !bg.accepted {
            return Err(format!("{} has no active accepted match", self.username));
        }
        let target = self
            .replica
            .units()
            .find_map(|unit| {
                let npc = unit.get::<shared::components::Npc>()?;
                let health = unit.get::<shared::components::Health>()?;
                (npc.template_id == entry && health.current > 0.0).then_some(unit.server_id)
            })
            .ok_or_else(|| format!("{} cannot see living source NPC{entry}", self.username))?;
        self.bridge.send::<_, CombatChannel>(SetTarget {
            target_entity: Some(target),
        })?;
        self.bridge
            .send::<_, CombatChannel>(AttackSwing { target })?;
        println!(
            "{} submitted normal attack on NPC{entry} entity{target}",
            self.username
        );
        Ok(())
    }

    fn observed_npcs(&self) -> Vec<serde_json::Value> {
        self.replica
            .units()
            .filter_map(|unit| {
                let npc = unit.get::<shared::components::Npc>()?;
                let position = unit.get::<shared::components::Position>()?;
                let health = unit
                    .get::<shared::components::Health>()
                    .map(|health| serde_json::json!({"current":health.current,"max":health.max}));
                Some(
                    serde_json::json!({"server_id":unit.server_id,"entry":npc.template_id,
                "name":npc.name,"position":[position.x,position.y,position.z],"health":health}),
                )
            })
            .collect()
    }

    fn observed_players(&self) -> Vec<serde_json::Value> {
        self.replica
            .units()
            .filter_map(|unit| {
                let player = unit.get::<shared::components::Player>()?;
                let position = unit.get::<shared::components::Position>()?;
                let health = unit.get::<shared::components::Health>()?;
                Some(
                    serde_json::json!({"server_id":unit.server_id,"name":player.name,
                "position":[position.x,position.y,position.z],
                "health":{"current":health.current,"max":health.max}}),
                )
            })
            .collect()
    }

    fn write_state(&self, directory: &Path) -> Result<(), String> {
        let value = serde_json::json!({"entered": self.entered,"map":self.map,
            "snapshot":self.snapshot,"npcs":self.observed_npcs(),
            "character_name":self.character_name,"group_roster":self.group_roster,
            "group_invite":self.group_invite,"group_notice":self.group_notice,
            "players":self.observed_players(),"inventory":self.inventory,
            "equipment":self.equipment,"inventory_delta":self.inventory_delta,
            "traits":self.traits,"trait_result":self.trait_result,"known_spells":self.known_spells,
            "combat_log":self.combat_log});
        write_peer_snapshot(&directory.join(format!("peer{}.json", self.index)), &value)
    }
}

fn write_peer_snapshot(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    let pending = path.with_extension("json.pending");
    fs::write(&pending, bytes)
        .map_err(|error| format!("Write peer snapshot {}: {error}", pending.display()))?;
    fs::rename(&pending, path)
        .map_err(|error| format!("Publish peer snapshot {}: {error}", path.display()))
}

fn private_support_count(address: std::net::SocketAddr, minimum: u8) -> Result<usize, String> {
    if !address.ip().is_loopback() {
        return Err("Private loopback battleground endpoints only".into());
    }
    match address.port() {
        5941 => Ok(PRIVATE_PROOF_SUPPORT_COUNT),
        5710..=5712 => Ok(usize::from(minimum) * 2 - 1),
        _ => Err("Private UDP5710–5712 or5941 only".into()),
    }
}

fn support_count_for_queue(
    address: std::net::SocketAddr,
    battleground_id: u32,
) -> Result<usize, String> {
    if battleground_id == shared::battleground::RANDOM_BATTLEGROUND_ID {
        return if address.ip().is_loopback() && address.port() == 5941 {
            Ok(PRIVATE_PROOF_SUPPORT_COUNT)
        } else {
            Err("Random queue fixture requires private loopback UDP5941".into())
        };
    }
    let template = shared::battleground::template_by_id(battleground_id)
        .ok_or("Unknown source battleground")?;
    private_support_count(address, template.min_players_per_team)
}

fn run() -> Result<(), String> {
    let directory = PathBuf::from(std::env::var("BG_DIRECTORY").map_err(|e| e.to_string())?);
    let address = std::env::var("BG_ENDPOINT")
        .map_err(|e| e.to_string())?
        .parse::<std::net::SocketAddr>()
        .map_err(|e| e.to_string())?;
    let battleground_id = std::env::var("BG_BATTLEGROUND_ID")
        .map_err(|e| e.to_string())?
        .parse()
        .map_err(|e: std::num::ParseIntError| e.to_string())?;
    let count = support_count_for_queue(address, battleground_id)?;
    let prefix = std::env::var("BG_ACCOUNT_PREFIX").map_err(|e| e.to_string())?;
    if !prefix.starts_with("fb_") {
        return Err("Only private fb_* accounts are allowed".into());
    }
    let mut peers = Vec::new();
    let clock = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos() as u64;
    for index in 1..=count {
        let bridge = BridgeConfig::new()
            .receive::<LoginResponse>()
            .receive::<EnterWorldResponse>()
            .receive::<NewWorld>()
            .receive::<PvpStateUpdate>()
            .receive::<GroupRosterSnapshot>()
            .receive::<GroupInvitePrompt>()
            .receive::<GroupCommandResponse>()
            .receive::<GroupInviteCancelled>()
            .receive::<AttackStart>()
            .receive::<AttackStopped>()
            .receive::<CastFailed>()
            .receive::<CombatLogEvent>()
            .receive::<InventorySnapshot>()
            .receive::<EquipmentSnapshot>()
            .receive::<InventoryDelta>()
            .receive::<InventoryError>()
            .receive::<KnownSpellsSnapshot>()
            .receive_talents()
            .receive::<SpellsLearned>()
            .receive::<SpellsUnlearned>()
            .connect(address, clock + index as u64)?;
        let username = format!("{prefix}{index}");
        bridge.send::<_, AuthChannel>(LoginRequest {
            token: None,
            username: username.clone(),
            password: "fbtest".into(),
        })?;
        peers.push(Peer {
            index,
            username,
            character_name: String::new(),
            sight: RefCell::new(None),
            battleground_id,
            bridge,
            entered: false,
            map: None,
            snapshot: None,
            group_roster: None,
            group_invite: None,
            group_notice: None,
            replica: Replica::default(),
            inventory: None,
            equipment: None,
            inventory_delta: None,
            traits: None,
            trait_result: None,
            known_spells: Vec::new(),
            combat_log: Vec::new(),
        });
        // Each real account completes entry before another worker starts. Large
        // rosters must not starve earlier handshakes during parallel app setup.
        let entry_deadline = Instant::now() + Duration::from_secs(30);
        while !peers.last().unwrap().entered {
            if Instant::now() >= entry_deadline {
                return Err(format!("{prefix}{index} did not enter within 30 seconds"));
            }
            for peer in &mut peers {
                peer.receive_events()?;
                peer.write_state(&directory)?;
            }
            thread::sleep(Duration::from_millis(50));
        }
        if index < count {
            settle_peer_login(&mut peers, &directory)?;
        }
    }
    let mut last = String::new();
    // Full source-timed matches include admission, terrain travel, objective clocks,
    // and saved return; the observed 12 gathers in 15 minutes need a longer fixture.
    // This is only the peer process lifetime, not a protocol/gameplay timeout.
    let deadline = Instant::now() + Duration::from_secs(3600);
    while Instant::now() < deadline {
        for peer in &mut peers {
            peer.receive_events()?;
            peer.write_state(&directory)?;
        }
        match fs::read_to_string(directory.join("peers-command.txt")) {
            Ok(command) if command != last => {
                last = command;
                let action = last.split_whitespace().next().ok_or("Empty command")?;
                if action == "quit" {
                    return Ok(());
                }
                for peer in &peers {
                    peer.action(action)?;
                }
                println!("submitted {last}");
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
        thread::sleep(Duration::from_millis(50));
    }
    Err("Live peers exceeded 30-minute deadline".into())
}
fn select_peer_action(index: usize, action: &str) -> Result<Option<&str>, String> {
    let Some(scoped) = action.strip_prefix("peer:") else {
        return Ok(Some(action));
    };
    let (requested, action) = scoped
        .split_once(':')
        .ok_or("expected peer:<index>:<action>")?;
    let requested: usize = requested
        .parse()
        .map_err(|_| "peer index must be an integer")?;
    if requested == 0 || action.is_empty() {
        return Err("peer index must be nonzero and action nonempty".into());
    }
    Ok((requested == index).then_some(action))
}

fn parse_bag_location(action: &str, verb: &str) -> Result<ItemLocation, String> {
    let fields: Vec<_> = action.split(':').collect();
    let [actual, bag, slot] = fields.as_slice() else {
        return Err(format!("expected {verb}:<bag>:<slot>"));
    };
    if *actual != verb {
        return Err(format!("expected {verb}"));
    }
    let bag: u8 = bag.parse().map_err(|_| "bag must be an integer")?;
    let slot: u8 = slot.parse().map_err(|_| "slot must be an integer")?;
    if bag >= BAG_COUNT {
        return Err("bag must be in the owner inventory".into());
    }
    Ok(ItemLocation::Bag { bag, slot })
}

fn parse_trait_commit(json: &str) -> Result<CommitTraitConfig, String> {
    serde_json::from_str(json).map_err(|error| format!("Decode trait request: {error}"))
}

fn parse_specialization(action: &str) -> Result<SetSpecialization, String> {
    let id = action
        .strip_prefix("spec:")
        .ok_or("expected spec:<specialization-ID>")?;
    let spec_id = id
        .parse()
        .map_err(|_| "specialization ID must be an integer")?;
    if spec_id == 0 {
        return Err("specialization ID must be nonzero".into());
    }
    Ok(SetSpecialization { spec_id })
}

fn parse_spell_cast(action: &str) -> Result<SpellCastIntent, String> {
    let fields: Vec<_> = action.split(':').collect();
    let ["cast", spell, target] = fields.as_slice() else {
        return Err("expected cast:<spell-id>:<target-entity>".into());
    };
    let spell_id: u32 = spell.parse().map_err(|_| "spell ID must be an integer")?;
    let target: u64 = target
        .parse()
        .map_err(|_| "target entity must be an integer")?;
    if spell_id == 0 || target == 0 {
        return Err("spell ID and target entity must be nonzero".into());
    }
    Ok(SpellCastIntent {
        spell_id: Some(spell_id),
        spell: String::new(),
        target_entity: Some(target),
        destination: None,
        target_item_guid: None,
        witness: None,
    })
}

fn parse_source_attack(action: &str) -> Result<(u32, BgTeam), String> {
    let fields: Vec<_> = action.split(':').collect();
    let ["attack", entry, team] = fields.as_slice() else {
        return Err("expected attack:<source-NPC-entry>:<A|B>".into());
    };
    let entry: u32 = entry
        .parse()
        .map_err(|_| "source NPC entry must be an integer")?;
    if entry == 0 {
        return Err("source NPC entry must be nonzero".into());
    }
    let team = match *team {
        "A" => BgTeam::A,
        "B" => BgTeam::B,
        _ => return Err("attack requires authoritative team A or B".into()),
    };
    Ok((entry, team))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random32_fixture_bootstrap_accepts_only_owned5941_without_a_fake_template() {
        assert_eq!(
            support_count_for_queue("127.0.0.1:5941".parse().unwrap(), 32),
            Ok(19)
        );
    }

    #[test]
    fn random32_fixture_bootstrap_preserves_private_guards_and_concrete_counts() {
        for address in [
            "127.0.0.1:5710",
            "127.0.0.1:5711",
            "127.0.0.1:5712",
            "127.0.0.1:5000",
            "192.0.2.1:5941",
            "127.0.0.1:5942",
        ] {
            assert!(support_count_for_queue(address.parse().unwrap(), 32).is_err());
        }
        assert_eq!(
            support_count_for_queue("127.0.0.1:5941".parse().unwrap(), 1018),
            Ok(19)
        );
        assert_eq!(
            support_count_for_queue("127.0.0.1:5712".parse().unwrap(), 1018),
            Ok(15)
        );
        assert!(support_count_for_queue("127.0.0.1:5941".parse().unwrap(), u32::MAX).is_err());
    }

    #[test]
    fn release_twenty_proof_uses_nineteen_real_support_accounts_on_private5941() {
        assert_eq!(
            private_support_count("127.0.0.1:5941".parse().unwrap(), 8),
            Ok(19)
        );
        assert_eq!(
            private_support_count("127.0.0.1:5712".parse().unwrap(), 8),
            Ok(15)
        );
        for address in ["127.0.0.1:5000", "192.0.2.1:5941", "127.0.0.1:5942"] {
            assert!(private_support_count(address.parse().unwrap(), 8).is_err());
        }
    }

    #[test]
    fn peer_snapshot_readers_always_observe_complete_json_during_publication() {
        use std::sync::{
            Barrier,
            atomic::{AtomicBool, Ordering},
        };
        let directory = std::env::temp_dir().join(format!(
            "bg-peer-publication-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&directory).unwrap();
        let path = directory.join("peer1.json");
        let payload = "x".repeat(262144);
        write_peer_snapshot(&path, &serde_json::json!({"seq":0,"payload":payload})).unwrap();
        let started = Barrier::new(2);
        let done = AtomicBool::new(false);
        thread::scope(|scope| {
            scope.spawn(|| {
                started.wait();
                for seq in 1..=64 {
                    write_peer_snapshot(&path, &serde_json::json!({"seq":seq,"payload":payload}))
                        .unwrap();
                    thread::yield_now();
                }
                done.store(true, Ordering::Release);
            });
            started.wait();
            let mut reads = 0;
            while !done.load(Ordering::Acquire) || reads == 0 {
                let bytes = fs::read(&path).unwrap();
                let value: serde_json::Value = serde_json::from_slice(&bytes)
                    .expect("published peer state must be a complete JSON receipt");
                assert_eq!(value["payload"].as_str(), Some(payload.as_str()));
                assert!(value["seq"].as_u64().unwrap() <= 64);
                reads += 1;
            }
            assert!(reads > 0);
        });
        let value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(value["seq"], 64);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn scoped_command_selects_exact_real_peer_without_changing_the_request() {
        assert_eq!(
            select_peer_action(2, "peer:2:attack:11947:A"),
            Ok(Some("attack:11947:A"))
        );
        assert_eq!(select_peer_action(3, "peer:2:attack:11947:A"), Ok(None));
        assert_eq!(select_peer_action(3, "queue"), Ok(Some("queue")));
        for input in ["peer:0:queue", "peer:bad:queue", "peer:2:", "peer:2"] {
            assert!(select_peer_action(2, input).is_err(), "{input}");
        }
    }

    #[test]
    fn trait_commit_preserves_source_spec_entries_and_total_ranks() {
        let request = parse_trait_commit(r#"{"spec_id":73,"entries":[{"node_id":90302,"entry_id":112156,"rank":1},{"node_id":90295,"entry_id":112149,"rank":1}]}"#).unwrap();
        assert_eq!(request.spec_id, 73);
        assert_eq!(
            request.entries,
            vec![
                TraitEntrySelection {
                    node_id: 90302,
                    entry_id: 112156,
                    rank: 1
                },
                TraitEntrySelection {
                    node_id: 90295,
                    entry_id: 112149,
                    rank: 1
                },
            ]
        );
    }

    #[test]
    fn specialization_command_preserves_normal_server_authority() {
        assert_eq!(
            parse_specialization("spec:73"),
            Ok(SetSpecialization { spec_id: 73 })
        );
        assert_eq!(
            parse_specialization("spec:257"),
            Ok(SetSpecialization { spec_id: 257 })
        );
        for input in ["spec:0", "spec:bad", "spec:257:all"] {
            assert!(parse_specialization(input).is_err(), "{input}");
        }
    }

    #[test]
    fn equipment_command_retains_owned_bag_location() {
        assert_eq!(
            parse_bag_location("equip:0:9", "equip"),
            Ok(ItemLocation::Bag { bag: 0, slot: 9 })
        );
        assert_eq!(
            parse_bag_location("unequip-offhand:0:15", "unequip-offhand"),
            Ok(ItemLocation::Bag { bag: 0, slot: 15 })
        );
        for input in ["equip:6:0", "equip:0:256", "equip:0", "equip:0:0:all"] {
            assert!(parse_bag_location(input, "equip").is_err(), "{input}");
        }
    }

    #[test]
    fn cast_command_retains_spell_and_live_entity_without_synthetic_damage() {
        let cast = parse_spell_cast("cast:2061:4294739375").unwrap();
        assert_eq!(cast.spell_id, Some(2061));
        assert_eq!(cast.target_entity, Some(4294739375));
        assert!(cast.destination.is_none());
        for input in [
            "cast:0:42",
            "cast:2061:0",
            "cast:2061:bad",
            "cast:2061:42:all",
        ] {
            assert!(parse_spell_cast(input).is_err(), "{input}");
        }
    }

    #[test]
    fn source_attack_command_retains_npc_identity_and_authoritative_team() {
        assert_eq!(
            parse_source_attack("attack:11946:A"),
            Ok((11946, BgTeam::A))
        );
        assert_eq!(
            parse_source_attack("attack:11948:B"),
            Ok((11948, BgTeam::B))
        );
    }

    #[test]
    fn source_attack_command_rejects_malformed_and_unfiltered_attacks() {
        for input in [
            "attack",
            "attack:11946",
            "attack:0:A",
            "attack:bad:A",
            "attack:11946:Q",
            "attack:11946:A:all",
            "kill:11946:A",
        ] {
            assert!(parse_source_attack(input).is_err(), "{input}");
        }
    }
}

/// Fixture accommodation for server login/catalog frames measured at 2.6–3.8s.
/// Keep earlier clients polling while the server drains this admission; production
/// handshake deadlines and the source roster minimum remain unchanged.
fn settle_peer_login(peers: &mut [Peer], directory: &Path) -> Result<(), String> {
    const LOGIN_SETTLE_TIME: Duration = Duration::from_secs(10);
    let deadline = Instant::now() + LOGIN_SETTLE_TIME;
    while Instant::now() < deadline {
        for peer in peers.iter_mut() {
            peer.receive_events()?;
            peer.write_state(directory)?;
        }
        thread::sleep(Duration::from_millis(50));
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
