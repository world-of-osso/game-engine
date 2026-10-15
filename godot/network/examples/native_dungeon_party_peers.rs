//! Four real-UDP party peers beside the single native proof camera.
//! Setup and combat commands use ordinary client messages; no server/admin API.
#[path = "native_dungeon_party_peers/sight.rs"]
mod sight;

use game_engine_network::{BridgeConfig, Event, NetworkBridge, ProtocolMessage, replica::Replica};
use serde::{Deserialize, Serialize};
use shared::{
    casting::CastState,
    components::{
        EquipmentAppearance, Health, Mana, MovementControl, Npc, Player, Position, UnitLevel,
        UnitPowers, UnitStats,
    },
    protocol::*,
};
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::Write,
    net::SocketAddr,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Deserialize, Serialize)]
struct Profile {
    account: String,
    name: String,
    class: u8,
}

#[derive(Deserialize)]
struct Command {
    sequence: u64,
    action: Action,
}

#[derive(Deserialize, Serialize)]
enum Action {
    Register,
    Login,
    Create,
    Enter,
    Specialize(u32),
    Invite(String),
    Target(Option<u64>),
    Attack(u64),
    StopAttack,
    Cast(SpellCastIntent),
    StopCast,
    Move(PlayerInput),
    Equip(EquipItem),
    Swap(SwapItem),
    Use(UseItem),
    Loot(LootUnit),
    LootChoice(GroupLootChoice),
}

struct Peer {
    index: usize,
    profile: Profile,
    bridge: NetworkBridge,
    replica: Replica,
    log: File,
    character: Option<CharacterListEntry>,
    registered: bool,
    pending_approval: bool,
    logged_in: bool,
    entered: bool,
    map: Option<u32>,
    specialization: Option<u32>,
    group: Option<GroupRosterSnapshot>,
    last_sequence: Option<u64>,
    inventory: Option<InventorySnapshot>,
    equipment: Option<EquipmentSnapshot>,
    inventory_delta: Option<InventoryDelta>,
    known_spells: Vec<u32>,
    sight: sight::CastSight,
}

fn decode<M: lightyear::prelude::Message>(message: ProtocolMessage) -> Result<M, String> {
    message.downcast::<M>().map_err(|_| {
        format!(
            "Decode expected protocol message {}",
            std::any::type_name::<M>()
        )
    })
}

fn connect_bridge(index: usize, endpoint: SocketAddr) -> Result<NetworkBridge, String> {
    BridgeConfig::new()
        .receive::<RegisterResponse>()
        .receive::<LoginResponse>()
        .receive::<CreateCharacterResponse>()
        .receive::<EnterWorldResponse>()
        .receive::<NewWorld>()
        .receive::<GroupInvitePrompt>()
        .receive::<GroupRosterSnapshot>()
        .receive::<SpecializationChanged>()
        .receive::<KnownSpellsSnapshot>()
        .receive::<CombatLogEvent>()
        .receive::<CastFailed>()
        .receive::<EncounterStart>()
        .receive::<EncounterEnd>()
        .receive::<InventorySnapshot>()
        .receive::<EquipmentSnapshot>()
        .receive::<InventoryDelta>()
        .receive::<InventoryError>()
        .receive::<LootResponse>()
        .receive::<LootFailed>()
        .receive::<GroupLootRoll>()
        .connect(endpoint, connection_nonce()? + index as u64)
}

impl Peer {
    fn connect(
        index: usize,
        profile: Profile,
        endpoint: SocketAddr,
        directory: &Path,
    ) -> Result<Self, String> {
        let bridge = connect_bridge(index, endpoint)?;
        let log = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(directory.join(format!("peer{index}-events.jsonl")))
            .map_err(|error| format!("Create peer{index} event log: {error}"))?;
        Ok(Self {
            index,
            profile,
            bridge,
            replica: Replica::default(),
            log,
            character: None,
            registered: false,
            pending_approval: false,
            logged_in: false,
            entered: false,
            map: None,
            specialization: None,
            group: None,
            last_sequence: None,
            inventory: None,
            equipment: None,
            inventory_delta: None,
            known_spells: Vec::new(),
            sight: sight::CastSight::load()?,
        })
    }

    fn record<T: Serialize>(&mut self, kind: &str, value: &T) -> Result<(), String> {
        let entry = serde_json::json!({"kind":kind,"data":value});
        writeln!(self.log, "{entry}")
            .and_then(|()| self.log.flush())
            .map_err(|error| format!("Write {} {kind} receipt: {error}", self.profile.name))
    }

    fn poll(&mut self) -> Result<(), String> {
        for event in self.bridge.drain_events()? {
            match event {
                Event::Message(message) => self.receive(message)?,
                Event::ReplicationStarted(schema) => self.replica = Replica::new(schema),
                Event::Replication(batch) => self
                    .replica
                    .apply(batch)
                    .map_err(|error| error.to_string())?,
                Event::Disconnected(reason) => {
                    return Err(format!("{} disconnected: {reason:?}", self.profile.name));
                }
                Event::ProtocolRejected(reason) => return Err(reason),
                _ => {}
            }
        }
        Ok(())
    }

    fn receive(&mut self, message: ProtocolMessage) -> Result<(), String> {
        if message.is::<RegisterResponse>() {
            return self.registered_reply(decode(message)?);
        }
        if message.is::<LoginResponse>() {
            return self.login_reply(decode(message)?);
        }
        if message.is::<CreateCharacterResponse>() {
            return self.creation_reply(decode(message)?);
        }
        self.receive_world(message)
    }

    fn registered_reply(&mut self, reply: RegisterResponse) -> Result<(), String> {
        self.record("registration", &serde_json::json!({
            "success":reply.success,"pending_approval":reply.pending_approval,"error":reply.error
        }))?;
        if !reply.success && !reply.pending_approval {
            return Err(format!(
                "{} registration refused: {:?}",
                self.profile.name, reply.error
            ));
        }
        self.registered = true;
        self.pending_approval = reply.pending_approval;
        Ok(())
    }

    fn login_reply(&mut self, reply: LoginResponse) -> Result<(), String> {
        if !reply.success {
            return Err(format!(
                "{} login refused: {:?}",
                self.profile.name, reply.error
            ));
        }
        self.character = reply
            .characters
            .into_iter()
            .find(|character| character.name == self.profile.name);
        if let Some(character) = &self.character {
            validate_character(&self.profile, character)?;
        }
        self.logged_in = true;
        self.pending_approval = false;
        self.record(
            "login",
            &serde_json::json!({"success":true,"character":self.character}),
        )
    }

    fn creation_reply(&mut self, reply: CreateCharacterResponse) -> Result<(), String> {
        if !reply.success {
            return Err(format!(
                "{} creation refused: {:?}",
                self.profile.name, reply.error
            ));
        }
        let character = reply
            .character
            .ok_or("Successful creation omitted character")?;
        validate_character(&self.profile, &character)?;
        self.record("character_created", &character)?;
        self.character = Some(character);
        Ok(())
    }

    fn receive_world(&mut self, message: ProtocolMessage) -> Result<(), String> {
        if message.is::<EnterWorldResponse>() {
            let reply: EnterWorldResponse = decode(message)?;
            if !reply.success {
                return Err(format!(
                    "{} world entry: {:?}",
                    self.profile.name, reply.error
                ));
            }
            self.entered = true;
            return self.record("world_entry", &reply);
        }
        if message.is::<NewWorld>() {
            let reply: NewWorld = decode(message)?;
            self.map = Some(reply.map_id);
            self.record("world_port", &reply)?;
            return self.bridge.send::<_, TransferChannel>(WorldPortAck);
        }
        if message.is::<GroupInvitePrompt>() {
            self.record("group_invite", &decode::<GroupInvitePrompt>(message)?)?;
            return self
                .bridge
                .send::<_, GroupChannel>(RespondGroupInvite { accept: true });
        }
        self.receive_party_and_combat(message)
    }

    fn receive_party_and_combat(&mut self, message: ProtocolMessage) -> Result<(), String> {
        if message.is::<KnownSpellsSnapshot>() {
            let reply: KnownSpellsSnapshot = decode(message)?;
            self.record("known_spells", &reply)?;
            self.known_spells = reply.spells;
        } else if message.is::<GroupRosterSnapshot>() {
            let reply: GroupRosterSnapshot = decode(message)?;
            self.record("group", &reply)?;
            self.group = Some(reply);
        } else if message.is::<SpecializationChanged>() {
            let reply: SpecializationChanged = decode(message)?;
            self.specialization = Some(reply.spec_id);
            self.record("specialization", &reply)?;
        } else if message.is::<CombatLogEvent>() {
            self.record("combat", &decode::<CombatLogEvent>(message)?)?;
        } else if message.is::<CastFailed>() {
            self.record("cast_failed", &decode::<CastFailed>(message)?)?;
        } else if message.is::<EncounterStart>() {
            self.record("encounter_start", &decode::<EncounterStart>(message)?)?;
        } else if message.is::<EncounterEnd>() {
            self.record("encounter_end", &decode::<EncounterEnd>(message)?)?;
        } else {
            return self.receive_inventory(message);
        }
        Ok(())
    }

    fn receive_inventory(&mut self, message: ProtocolMessage) -> Result<(), String> {
        if message.is::<InventorySnapshot>() {
            let reply: InventorySnapshot = decode(message)?;
            self.record("inventory", &reply)?;
            self.inventory = Some(reply);
        } else if message.is::<EquipmentSnapshot>() {
            let reply: EquipmentSnapshot = decode(message)?;
            self.record("equipment", &reply)?;
            self.equipment = Some(reply);
        } else if message.is::<InventoryDelta>() {
            let reply: InventoryDelta = decode(message)?;
            self.record("inventory_delta", &reply)?;
            self.inventory_delta = Some(reply);
        } else if message.is::<InventoryError>() {
            self.record("inventory_error", &decode::<InventoryError>(message)?)?;
        } else {
            return self.receive_loot(message);
        }
        Ok(())
    }

    fn receive_loot(&mut self, message: ProtocolMessage) -> Result<(), String> {
        if message.is::<LootResponse>() {
            self.record("loot", &decode::<LootResponse>(message)?)?;
        } else if message.is::<LootFailed>() {
            self.record("loot_failed", &decode::<LootFailed>(message)?)?;
        } else if message.is::<GroupLootRoll>() {
            self.record("loot_roll", &decode::<GroupLootRoll>(message)?)?;
        }
        Ok(())
    }

    fn read_command(&mut self, directory: &Path) -> Result<(), String> {
        let path = directory.join(format!("peer{}-command.json", self.index));
        if !path.exists() {
            return Ok(());
        }
        let bytes = fs::read(&path).map_err(|error| format!("Read {}: {error}", path.display()))?;
        let command: Command = serde_json::from_slice(&bytes)
            .map_err(|error| format!("Decode {}: {error}", path.display()))?;
        if self.last_sequence == Some(command.sequence) {
            return Ok(());
        }
        self.send_action(&command.action)?;
        self.record(
            "intent",
            &serde_json::json!({"sequence":command.sequence,"action":command.action}),
        )?;
        self.last_sequence = Some(command.sequence);
        Ok(())
    }

    fn send_action(&mut self, action: &Action) -> Result<(), String> {
        match action {
            Action::Register => self.bridge.send::<_, AuthChannel>(RegisterRequest {
                username: self.profile.account.clone(),
                password: "fbtest".into(),
            }),
            Action::Login => self.bridge.send::<_, AuthChannel>(LoginRequest {
                token: None,
                username: self.profile.account.clone(),
                password: "fbtest".into(),
            }),
            Action::Create => self.bridge.send::<_, AuthChannel>(CreateCharacter {
                name: self.profile.name.clone(),
                race: 1,
                class: self.profile.class,
                appearance: Default::default(),
            }),
            Action::Enter => self.bridge.send::<_, AuthChannel>(SelectCharacter {
                character_id: self
                    .character
                    .as_ref()
                    .ok_or("Character not created/logged in")?
                    .character_id,
            }),
            _ => self.send_world_action(action),
        }
    }

    fn send_world_action(&mut self, action: &Action) -> Result<(), String> {
        if !self.entered {
            return Err(format!("{} has not entered the world", self.profile.name));
        }
        match action {
            Action::Specialize(spec_id) => self
                .bridge
                .send::<_, TalentChannel>(SetSpecialization { spec_id: *spec_id }),
            Action::Invite(name) => self
                .bridge
                .send::<_, GroupChannel>(GroupInviteIntent { name: name.clone() }),
            Action::Target(target) => self.bridge.send::<_, CombatChannel>(SetTarget {
                target_entity: *target,
            }),
            Action::Attack(target) => self
                .bridge
                .send::<_, CombatChannel>(AttackSwing { target: *target }),
            Action::StopAttack => self.bridge.send::<_, CombatChannel>(AttackStop),
            Action::Cast(intent) => self.send_cast(intent.clone()),
            Action::StopCast => self.bridge.send::<_, CombatChannel>(StopSpellCast),
            Action::Move(input) => self.bridge.send::<_, InputChannel>(input.clone()),
            _ => self.send_inventory_action(action),
        }
    }

    fn send_cast(&mut self, mut intent: SpellCastIntent) -> Result<(), String> {
        intent.witness = self.sight.witness(
            &self.replica,
            &self.profile.name,
            self.map,
            intent.target_entity,
        )?;
        self.record("wire_cast", &intent)?;
        self.bridge.send::<_, CombatChannel>(intent)
    }

    fn send_inventory_action(&self, action: &Action) -> Result<(), String> {
        match action {
            Action::Equip(intent) => self.bridge.send::<_, InventoryChannel>(intent.clone()),
            Action::Swap(intent) => self.bridge.send::<_, InventoryChannel>(intent.clone()),
            Action::Use(intent) => self.bridge.send::<_, InventoryChannel>(intent.clone()),
            Action::Loot(intent) => self.bridge.send::<_, LootChannel>(*intent),
            Action::LootChoice(intent) => self.bridge.send::<_, LootChannel>(*intent),
            _ => Err("Auth action routed as world action".into()),
        }
    }

    fn write_state(&self, directory: &Path) -> Result<(), String> {
        let players = observe_players(&self.replica);
        let bosses = observe_bosses(&self.replica);
        let state = serde_json::json!({"profile":self.profile,"registered":self.registered,
            "pending_approval":self.pending_approval,"logged_in":self.logged_in,"character":self.character,
            "entered":self.entered,"map":self.map,"spec_id":self.specialization,"group":self.group,
            "last_sequence":self.last_sequence,"players":players,"bosses":bosses,
            "inventory_snapshot":self.inventory,"equipment_snapshot":self.equipment,"last_inventory_delta":self.inventory_delta,
            "known_spells":self.known_spells});
        let path = directory.join(format!("peer{}-state.json", self.index));
        let temporary = path.with_extension("json.tmp");
        fs::write(
            &temporary,
            serde_json::to_vec_pretty(&state).map_err(|error| error.to_string())?,
        )
        .and_then(|()| fs::rename(&temporary, &path))
        .map_err(|error| format!("Write {}: {error}", path.display()))
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        if let Err(error) = self.bridge.disconnect() {
            eprintln!("Disconnect {}: {error}", self.profile.name);
        }
        if let Err(error) = self.bridge.stop() {
            eprintln!("Stop {}: {error}", self.profile.name);
        }
    }
}

fn validate_character(profile: &Profile, character: &CharacterListEntry) -> Result<(), String> {
    if character.name != profile.name || character.race != 1 || character.class != profile.class {
        return Err(format!(
            "{} roster identity differs from its explicit profile",
            profile.name
        ));
    }
    Ok(())
}

fn observe_players(replica: &Replica) -> Vec<serde_json::Value> {
    replica.units().filter_map(|unit| {
        let player = unit.get::<Player>()?;
        Some(serde_json::json!({"entity":unit.server_id,"name":player.name,
            "race":player.race,"class":player.class,"level":unit.get::<UnitLevel>(),
            "health":unit.get::<Health>(),"mana":unit.get::<Mana>(),"position":unit.get::<Position>(),
            "movement":unit.get::<MovementControl>(),"cast":unit.get::<CastState>(),
            "stats":unit.get::<UnitStats>(),"powers":unit.get::<UnitPowers>(),
            "equipment_appearance":unit.get::<EquipmentAppearance>()}))
    }).collect()
}

fn observe_bosses(replica: &Replica) -> Vec<serde_json::Value> {
    replica.units().filter_map(|unit| {
        let npc = unit.get::<Npc>()?;
        (npc.template_id == 3887).then(|| serde_json::json!({"entity":unit.server_id,
            "entry":npc.template_id,"health":unit.get::<Health>(),"position":unit.get::<Position>()}))
    }).collect()
}

fn read_profiles(directory: &Path) -> Result<Vec<Profile>, String> {
    let bytes = fs::read(directory.join("peers.json"))
        .map_err(|error| format!("Read party profiles: {error}"))?;
    let profiles: Vec<Profile> = serde_json::from_slice(&bytes)
        .map_err(|error| format!("Decode party profiles: {error}"))?;
    if profiles.len() != 4 {
        return Err("Exactly four headless profiles are required".into());
    }
    if profiles
        .iter()
        .any(|profile| !profile.account.starts_with("fb_encounter_"))
    {
        return Err("Only disposable fb_encounter_* accounts are permitted".into());
    }
    let accounts: HashSet<_> = profiles.iter().map(|profile| &profile.account).collect();
    let names: HashSet<_> = profiles.iter().map(|profile| &profile.name).collect();
    if accounts.len() != 4 || names.len() != 4 {
        return Err("Party accounts and character names must be distinct".into());
    }
    Ok(profiles)
}

fn connection_nonce() -> Result<u64, String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("Connection clock: {error}"))?
        .as_nanos() as u64)
}

fn run() -> Result<(), String> {
    let directory = PathBuf::from(
        std::env::var("ENCOUNTER_PARTY_DIRECTORY").map_err(|error| error.to_string())?,
    );
    let endpoint: SocketAddr = std::env::var("GODOT_TEST_SERVER")
        .map_err(|error| error.to_string())?
        .parse()
        .map_err(|error| format!("Private party endpoint: {error}"))?;
    if !endpoint.ip().is_loopback() || endpoint.port() != 5798 {
        return Err("Only owned loopback UDP5798 is permitted".into());
    }
    let profiles = read_profiles(&directory)?;
    let mut peers: Vec<_> = profiles
        .into_iter()
        .enumerate()
        .map(|(index, profile)| Peer::connect(index, profile, endpoint, &directory))
        .collect::<Result<_, _>>()?;
    let deadline = Instant::now() + Duration::from_secs(3600);
    while Instant::now() < deadline {
        if directory.join("stop").exists() {
            return Ok(());
        }
        for peer in &mut peers {
            peer.poll()?;
            peer.read_command(&directory)?;
            peer.write_state(&directory)?;
        }
        thread::sleep(Duration::from_millis(50));
    }
    Err("Owned party control deadline expired".into())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("DUNGEON_PARTY_PEER_ERROR {error}");
        std::process::exit(1);
    }
}
