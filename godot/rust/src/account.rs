use std::{
    fs,
    net::ToSocketAddrs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use game_engine_network::{Event, NetworkBridge, ProtocolMessage, UnitSnapshot};
use game_engine_session::{
    AuthRequest, ReconnectPhase, Session, SessionEffect, SessionOptions, SessionScreen,
    normalize_auth_token, token_path,
};
use shared::protocol::{
    AuthChannel, CharacterListUpdate, CreateCharacter, CreateCharacterResponse, DeleteCharacter,
    DeleteCharacterResponse, EnterWorldResponse, ForcedDisconnect, InputChannel, LoadTerrain,
    LoginResponse, NewWorld, PlayerInput, RegisterResponse, TransferAborted, TransferChannel,
    WorldPortAck,
};

/// Godot host's account state. Only NetworkBridge owns the transport ECS world.
pub struct Account {
    pub session: Session,
    pub reply_received: bool,
    bridge: Option<NetworkBridge>,
    data_root: PathBuf,
    hostname: String,
}

pub enum AccountEvent {
    Screen(SessionScreen),
    WorldReset,
    LoadTerrain(LoadTerrain),
    NewWorld(NewWorld),
    TransferError(String),
    UnitUpdated(UnitSnapshot),
    UnitRemoved(u64),
    /// The character roster changed through a server update or response.
    RosterChanged,
    /// Server answer to the pending character-creation request.
    CharacterCreated {
        success: bool,
        error: Option<String>,
    },
}

impl Account {
    pub fn new(data_root: PathBuf) -> Self {
        Self {
            session: Session::default(),
            reply_received: false,
            bridge: None,
            data_root,
            hostname: String::new(),
        }
    }

    pub fn connect(
        &mut self,
        hostname: &str,
        username: &str,
        password: &str,
        register: bool,
    ) -> Result<(), String> {
        self.stop()?;
        self.reply_received = false;
        self.hostname = hostname.to_owned();
        self.session.token = self.read_token()?;
        let reconnect = !register && username.trim().is_empty() && password.trim().is_empty();
        if reconnect && self.session.token.is_none() {
            return Err("No saved session to reconnect".into());
        }
        self.start_transport(username, password, register)
    }

    fn start_transport(
        &mut self,
        username: &str,
        password: &str,
        register: bool,
    ) -> Result<(), String> {
        let address = self
            .hostname
            .to_socket_addrs()
            .map_err(|error| format!("Resolve realm {}: {error}", self.hostname))?
            .find(|address| address.is_ipv4())
            .ok_or_else(|| format!("Realm {} has no IPv4 address", self.hostname))?;
        self.reply_received = false;
        let client_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("Connection clock: {error}"))?
            .as_nanos() as u64;
        let bridge = NetworkBridge::connect(address, client_id)?;
        match self.session.auth_request(username, password, register) {
            AuthRequest::Login(request) => bridge.send::<_, AuthChannel>(request)?,
            AuthRequest::Register(request) => bridge.send::<_, AuthChannel>(request)?,
        }
        self.bridge = Some(bridge);
        Ok(())
    }

    pub fn send_enter_world(&self) -> Result<(), String> {
        let Some(request) = self.session.select_character() else {
            return Ok(());
        };
        self.connected_bridge()?.send::<_, AuthChannel>(request)
    }

    pub fn send_create_character(&self, request: CreateCharacter) -> Result<(), String> {
        self.connected_bridge()?.send::<_, AuthChannel>(request)
    }

    pub fn send_delete_character(&self, character_id: u64) -> Result<(), String> {
        self.connected_bridge()?
            .send::<_, AuthChannel>(DeleteCharacter { character_id })
    }

    pub fn send_player_input(&self, input: PlayerInput) -> Result<(), String> {
        if self.session.screen != SessionScreen::InWorld || !self.session.gameplay_input_allowed() {
            return Ok(());
        }
        self.connected_bridge()?.send::<_, InputChannel>(input)
    }

    /// Called by the host only after the destination is ready for world entry.
    pub fn finish_world_port(&mut self) -> Result<(), String> {
        self.session.finish_world_port();
        if self.session.loaded_world_port().is_some() {
            self.connected_bridge()?
                .send::<_, TransferChannel>(WorldPortAck)?;
            self.session.take_world_port_ack();
        }
        Ok(())
    }

    fn read_token(&self) -> Result<Option<String>, String> {
        let path = token_path(&self.data_root, Some(&self.hostname));
        match fs::read_to_string(&path) {
            Ok(token) => Ok(normalize_auth_token(&token)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(format!("Read saved session {}: {error}", path.display())),
        }
    }

    pub fn poll(&mut self) -> Result<Vec<AccountEvent>, String> {
        let events = match self.bridge.as_mut() {
            Some(bridge) => bridge.drain_events()?,
            None => return Ok(Vec::new()),
        };
        let mut output = Vec::new();
        for event in events {
            match event {
                Event::Connected => self.session.receive_connected(),
                Event::Disconnected(reason) => {
                    let effects = self
                        .session
                        .receive_disconnected_with_reason(reason.as_deref());
                    self.apply_effects(effects, &mut output)?;
                }
                Event::Message(message) => self.dispatch_message(message, &mut output)?,
                Event::UnitUpdated(unit) => output.push(AccountEvent::UnitUpdated(unit)),
                Event::UnitRemoved(id) => output.push(AccountEvent::UnitRemoved(id)),
            }
            if self.bridge.is_none() {
                break;
            }
        }
        // Finish the old event batch before creating a replacement worker. Reset effects stop
        // and join the old worker, and the loop above discards its remaining queued events.
        if self.bridge.is_none() && self.session.reconnect_phase == ReconnectPhase::PendingConnect {
            self.start_transport("", "", false)?;
        }
        Ok(output)
    }

    fn dispatch_message(
        &mut self,
        message: ProtocolMessage,
        output: &mut Vec<AccountEvent>,
    ) -> Result<(), String> {
        if message.is::<LoadTerrain>() {
            let request = decode(message)?;
            self.session.receive_terrain_refresh();
            output.push(AccountEvent::LoadTerrain(request));
            return Ok(());
        }
        if message.is::<NewWorld>() {
            let new_world: NewWorld = decode(message)?;
            self.session.begin_world_port(new_world.map_id);
            output.push(AccountEvent::NewWorld(new_world));
            output.push(AccountEvent::Screen(SessionScreen::Loading));
            return Ok(());
        }
        if message.is::<TransferAborted>() {
            let aborted: TransferAborted = decode(message)?;
            output.push(AccountEvent::TransferError(
                self.format_transfer_error(aborted)?,
            ));
            return Ok(());
        }
        if message.is::<CharacterListUpdate>() {
            self.session.receive_character_update(decode(message)?);
            output.push(AccountEvent::RosterChanged);
            return Ok(());
        }
        if message.is::<DeleteCharacterResponse>() {
            self.session.receive_character_deleted(decode(message)?);
            output.push(AccountEvent::RosterChanged);
            return Ok(());
        }
        if message.is::<CreateCharacterResponse>() {
            let result = self.session.receive_character_created(decode(message)?);
            output.push(AccountEvent::RosterChanged);
            output.push(AccountEvent::CharacterCreated {
                success: result.success,
                error: result.error,
            });
            return Ok(());
        }
        let effects = self.receive_message(message)?;
        self.apply_effects(effects, output)
    }

    fn receive_message(&mut self, message: ProtocolMessage) -> Result<Vec<SessionEffect>, String> {
        if message.is::<LoginResponse>() {
            self.reply_received = true;
            return Ok(self
                .session
                .receive_login(decode(message)?, SessionOptions::default()));
        }
        if message.is::<RegisterResponse>() {
            self.reply_received = true;
            return Ok(self.session.receive_registration(decode(message)?));
        }
        if message.is::<EnterWorldResponse>() {
            return Ok(self.session.receive_enter_world(decode(message)?));
        }
        if message.is::<ForcedDisconnect>() {
            return Ok(self.session.receive_forced_disconnect(decode(message)?));
        }
        Err("Unhandled account protocol message".into())
    }

    fn apply_effects(
        &mut self,
        effects: Vec<SessionEffect>,
        output: &mut Vec<AccountEvent>,
    ) -> Result<(), String> {
        for effect in effects {
            match effect {
                SessionEffect::PersistToken(token) => {
                    let path = token_path(&self.data_root, Some(&self.hostname));
                    fs::write(&path, token)
                        .map_err(|error| format!("Save session {}: {error}", path.display()))?;
                }
                SessionEffect::SelectCharacter(request) => {
                    self.connected_bridge()?.send::<_, AuthChannel>(request)?
                }
                SessionEffect::RequestDisconnect => self.connected_bridge()?.disconnect()?,
                SessionEffect::ResetNetworkWorld => {
                    self.stop()?;
                    output.push(AccountEvent::WorldReset);
                }
                SessionEffect::Transition(screen) => output.push(AccountEvent::Screen(screen)),
            }
        }
        Ok(())
    }

    fn format_transfer_error(&self, aborted: TransferAborted) -> Result<String, String> {
        use shared::protocol::TransferAbortReason;

        let map_name = match aborted.reason {
            TransferAbortReason::Difficulty(_) | TransferAbortReason::LockedToDifferentInstance => {
                read_transfer_map_name(&self.data_root, aborted.map_id)?
            }
            _ => String::new(),
        };
        Ok(self.session.receive_transfer_aborted(aborted, &map_name))
    }

    fn connected_bridge(&self) -> Result<&NetworkBridge, String> {
        self.bridge
            .as_ref()
            .ok_or_else(|| "No active account connection".into())
    }

    pub fn stop(&mut self) -> Result<(), String> {
        self.session.reset_world_port();
        if let Some(mut bridge) = self.bridge.take() {
            bridge.stop()?;
        }
        Ok(())
    }
}

fn read_transfer_map_name(data_root: &Path, map_id: u32) -> Result<String, String> {
    use game_engine_core::csv_util::header_index;

    let path = data_root.join("db2/12.1.0.69933/Map.csv");
    let file = fs::File::open(&path)
        .map_err(|error| format!("Read map names {}: {error}", path.display()))?;
    let mut reader = csv::Reader::from_reader(file);
    let headers = reader
        .headers()
        .map_err(|error| format!("Read map names {}: {error}", path.display()))?;
    if headers.is_empty() {
        return Err(format!("{} is empty", path.display()));
    }
    let headers: Vec<String> = headers.iter().map(str::to_owned).collect();
    let id_column = header_index(&headers, "ID", &path)?;
    let name_column = header_index(&headers, "MapName_lang", &path)?;
    for record in reader.records() {
        let fields =
            record.map_err(|error| format!("Read map names {}: {error}", path.display()))?;
        let id = fields
            .get(id_column)
            .ok_or_else(|| format!("{}: missing map ID in {fields:?}", path.display()))?
            .parse::<u32>()
            .map_err(|error| {
                format!("{}: invalid map ID in {fields:?}: {error}", path.display())
            })?;
        if id == map_id {
            return fields
                .get(name_column)
                .filter(|name| !name.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| format!("{}: missing name for map {map_id}", path.display()));
        }
    }
    Err(format!("{}: map {map_id} not found", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::protocol::TransferAbortReason;

    #[test]
    fn full_instance_error_does_not_require_map_catalog() {
        let account = Account::new(PathBuf::from("/nonexistent-map-catalog"));
        let error = account
            .format_transfer_error(TransferAborted {
                map_id: 34,
                reason: TransferAbortReason::MaxPlayers,
            })
            .unwrap();
        assert_eq!(error, "Transfer Aborted: instance is full");
    }

    #[test]
    fn heroic_transfer_error_uses_aborted_map_name() {
        let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let account = Account::new(data_root);
        let error = account
            .format_transfer_error(TransferAborted {
                map_id: 34,
                reason: TransferAbortReason::Difficulty(2),
            })
            .unwrap();
        assert_eq!(
            error,
            "Heroic difficulty mode is not available for Stormwind Stockade."
        );
    }

    #[test]
    fn transfer_abort_map_name_comes_from_map_csv_id() {
        let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        assert_eq!(
            read_transfer_map_name(&data_root, 34).unwrap(),
            "Stormwind Stockade"
        );
        assert_eq!(
            read_transfer_map_name(&data_root, 33).unwrap(),
            "Shadowfang Keep"
        );
        assert!(read_transfer_map_name(&data_root, u32::MAX).is_err());
    }
}

fn decode<M: game_engine_network::WireMessage>(message: ProtocolMessage) -> Result<M, String> {
    message.downcast::<M>().map_err(|_| {
        format!(
            "Invalid protocol message type {}",
            std::any::type_name::<M>()
        )
    })
}
