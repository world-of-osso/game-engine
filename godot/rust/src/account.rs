use std::{
    fs,
    net::ToSocketAddrs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use game_engine_network::{Event, NetworkBridge, ProtocolMessage, UnitSnapshot};
use game_engine_session::{
    AuthRequest, Session, SessionEffect, SessionOptions, SessionScreen, normalize_auth_token,
    token_path,
};
use shared::protocol::{
    AuthChannel, CharacterListUpdate, CreateCharacterResponse, DeleteCharacterResponse,
    EnterWorldResponse, ForcedDisconnect, LoginResponse, RegisterResponse,
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
    UnitUpdated(UnitSnapshot),
    UnitRemoved(u64),
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
        let address = hostname
            .to_socket_addrs()
            .map_err(|error| format!("Resolve realm {hostname}: {error}"))?
            .find(|address| address.is_ipv4())
            .ok_or_else(|| format!("Realm {hostname} has no IPv4 address"))?;
        self.stop()?;
        self.reply_received = false;
        self.hostname = hostname.to_owned();
        self.session.token = self.read_token()?;
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
                Event::Connected => {}
                Event::Disconnected(reason) => {
                    if let Some(reason) = reason {
                        self.session.feedback = Some(reason);
                    }
                    let effects = self.session.receive_disconnected();
                    self.apply_effects(effects, &mut output)?;
                }
                Event::Message(message) => {
                    let effects = self.receive_message(message)?;
                    self.apply_effects(effects, &mut output)?;
                }
                Event::UnitUpdated(unit) => output.push(AccountEvent::UnitUpdated(unit)),
                Event::UnitRemoved(id) => output.push(AccountEvent::UnitRemoved(id)),
            }
            if self.bridge.is_none() {
                break;
            }
        }
        Ok(output)
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
        if message.is::<CharacterListUpdate>() {
            self.session.receive_character_update(decode(message)?);
            return Ok(Vec::new());
        }
        if message.is::<CreateCharacterResponse>() {
            self.session.receive_character_created(decode(message)?);
            return Ok(Vec::new());
        }
        if message.is::<DeleteCharacterResponse>() {
            self.session.receive_character_deleted(decode(message)?);
            return Ok(Vec::new());
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
                SessionEffect::ResetNetworkWorld => self.stop()?,
                SessionEffect::Transition(screen) => output.push(AccountEvent::Screen(screen)),
            }
        }
        Ok(())
    }

    fn connected_bridge(&self) -> Result<&NetworkBridge, String> {
        self.bridge
            .as_ref()
            .ok_or_else(|| "No active account connection".into())
    }

    pub fn stop(&mut self) -> Result<(), String> {
        if let Some(mut bridge) = self.bridge.take() {
            bridge.stop()?;
        }
        Ok(())
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
