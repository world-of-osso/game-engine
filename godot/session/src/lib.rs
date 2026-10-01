//! Headless account and character-session decisions. The host owns transport, token I/O,
//! screen projection, network resets, and world/reconnect completion.

pub mod logout;

use shared::protocol::{
    CharacterListEntry, CharacterListUpdate, CreateCharacterResponse, DeleteCharacterResponse,
    EnterWorldResponse, ForcedDisconnect, LoginRequest, LoginResponse, RegisterRequest,
    RegisterResponse, SelectCharacter, TransferAborted,
};
use std::path::{Path, PathBuf};

const TOKEN_FILE: &str = "auth_token";
const TEST_PLACEHOLDER_UUID: &str = "11111111-1111-1111-1111-111111111111";

/// Normalize tokens loaded from the existing plaintext token cache.
pub fn normalize_auth_token(token: &str) -> Option<String> {
    let token = token.trim();
    if token.is_empty()
        || token.eq_ignore_ascii_case("saved-token")
        || token == TEST_PLACEHOLDER_UUID
    {
        return None;
    }
    Some(token.to_owned())
}

/// Build the original server-keyed token filename within the host-provided data directory.
/// The old host used `<engine CARGO_MANIFEST_DIR>/data`; this crate does no disk I/O.
pub fn token_path(data_dir: &Path, server: Option<&str>) -> PathBuf {
    match server {
        Some(address) if !is_worldofosso_host(address) => {
            let sanitized = address.replace(':', "_").replace('/', "_");
            data_dir.join(format!("{TOKEN_FILE}.{sanitized}"))
        }
        _ => data_dir.join(TOKEN_FILE),
    }
}

fn is_worldofosso_host(address: &str) -> bool {
    let host = address.split(':').next().unwrap_or(address);
    host.eq_ignore_ascii_case("worldofosso.com")
        || host.to_ascii_lowercase().ends_with(".worldofosso.com")
}

/// The host sends the enclosed request on the existing `AuthChannel`.
#[derive(Debug)]
pub enum AuthRequest {
    Login(LoginRequest),
    Register(RegisterRequest),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SessionScreen {
    #[default]
    Login,
    CharacterSelect,
    CharacterCreate,
    Loading,
    InWorld,
    /// Standalone menu preview; closing the overlay does not select an account screen.
    GameMenu,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SessionOptions<'a> {
    pub preselected_name: Option<&'a str>,
    pub auto_enter_world: bool,
    pub startup_screen: Option<SessionScreen>,
}

/// Concrete commands for the host; none perform I/O in this crate.
#[derive(Debug)]
pub enum SessionEffect {
    PersistToken(String),
    SelectCharacter(SelectCharacter),
    ResetNetworkWorld,
    RequestDisconnect,
    Transition(SessionScreen),
    /// `feedback` changed; the host shows it even when the screen stays the same.
    ShowFeedback,
}

/// Successful creation emits a UI result even if the server omitted the character entry.
#[derive(Debug, PartialEq, Eq)]
pub struct CharacterCreationResult {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PendingWorldPort {
    #[default]
    None,
    Loading(u32),
    Loaded(u32),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReconnectPhase {
    #[default]
    Inactive,
    PendingConnect,
    AwaitingWorld,
}

/// Login feedback after the connection dropped.
const CONNECTION_LOST: &str = "Connection lost.";

#[derive(Default)]
pub struct Session {
    pub token: Option<String>,
    pub feedback: Option<String>,
    pub characters: Vec<CharacterListEntry>,
    pub selected_index: Option<usize>,
    pub selected_character_id: Option<u64>,
    pub selected_character_name: Option<String>,
    pub screen: SessionScreen,
    pub reconnect_phase: ReconnectPhase,
    pub terrain_refresh_seen: bool,
    reconnect_auto_enter_world: bool,
    reconnect_preselected_name: Option<String>,
    pending_forced_disconnect: Option<ForcedDisconnect>,
    pending_world_port: PendingWorldPort,
}

impl Session {
    pub fn gameplay_input_allowed(&self) -> bool {
        self.reconnect_phase == ReconnectPhase::Inactive
    }

    pub fn receive_connected(&mut self) {
        if self.reconnect_phase != ReconnectPhase::Inactive {
            self.reconnect_phase = ReconnectPhase::AwaitingWorld;
        }
    }

    pub fn receive_terrain_refresh(&mut self) {
        if self.reconnect_phase != ReconnectPhase::Inactive {
            self.terrain_refresh_seen = true;
        }
    }

    /// Local unit presence is the completion boundary, not rendered terrain or model readiness.
    pub fn finish_reconnect(&mut self, local_player_present: bool) -> bool {
        if self.reconnect_phase == ReconnectPhase::AwaitingWorld
            && self.terrain_refresh_seen
            && local_player_present
        {
            self.clear_reconnect();
            return true;
        }
        false
    }

    fn clear_reconnect(&mut self) {
        self.reconnect_phase = ReconnectPhase::Inactive;
        self.terrain_refresh_seen = false;
        self.reconnect_auto_enter_world = false;
        self.reconnect_preselected_name = None;
    }

    pub fn pending_world_port(&self) -> PendingWorldPort {
        self.pending_world_port
    }

    pub fn begin_world_port(&mut self, map_id: u32) {
        self.pending_world_port = PendingWorldPort::Loading(map_id);
        self.screen = SessionScreen::Loading;
    }

    pub fn finish_world_port(&mut self) {
        if let PendingWorldPort::Loading(map_id) = self.pending_world_port {
            self.pending_world_port = PendingWorldPort::Loaded(map_id);
        }
    }

    pub fn loaded_world_port(&self) -> Option<u32> {
        match self.pending_world_port {
            PendingWorldPort::Loaded(map_id) => Some(map_id),
            _ => None,
        }
    }

    pub fn take_world_port_ack(&mut self) -> Option<u32> {
        let map_id = self.loaded_world_port()?;
        self.reset_world_port();
        Some(map_id)
    }

    pub fn reset_world_port(&mut self) {
        self.pending_world_port = PendingWorldPort::None;
    }

    pub fn receive_transfer_aborted(&self, aborted: TransferAborted, map_name: &str) -> String {
        aborted.reason.text(map_name)
    }

    /// Queue this request as the transport starts; do not wait for a screen callback.
    pub fn auth_request(&self, username: &str, password: &str, register: bool) -> AuthRequest {
        if register {
            return AuthRequest::Register(RegisterRequest {
                username: username.to_owned(),
                password: password.to_owned(),
            });
        }
        let token = if username.trim().is_empty() && password.trim().is_empty() {
            self.token.as_deref().and_then(normalize_auth_token)
        } else {
            None
        };
        AuthRequest::Login(LoginRequest {
            token,
            username: username.to_owned(),
            password: password.to_owned(),
        })
    }

    pub fn receive_login(
        &mut self,
        response: LoginResponse,
        options: SessionOptions<'_>,
    ) -> Vec<SessionEffect> {
        self.reset_world_port();
        if !response.success {
            self.clear_reconnect();
            let error = response.error.unwrap_or_default();
            self.feedback = Some(user_facing_login_error(&error).to_owned());
            self.screen = SessionScreen::Login;
            return vec![
                SessionEffect::ResetNetworkWorld,
                SessionEffect::Transition(self.screen),
            ];
        }

        self.token = Some(response.token.clone());
        self.feedback = None;
        self.characters = response.characters;
        let reconnecting = self.reconnect_phase != ReconnectPhase::Inactive;
        let name = if reconnecting {
            self.reconnect_preselected_name.as_deref()
        } else {
            options.preselected_name
        };
        self.selected_index = resolve_selected_index(&self.characters, name);
        let mut effects = vec![SessionEffect::PersistToken(response.token)];
        let auto_enter = if reconnecting {
            self.reconnect_auto_enter_world
        } else {
            options.auto_enter_world
        };
        let selected = auto_enter.then(|| self.select_character()).flatten();
        if let Some(selection) = selected {
            effects.push(SessionEffect::SelectCharacter(selection));
            self.screen = SessionScreen::Loading;
        } else {
            self.screen = options
                .startup_screen
                .unwrap_or(SessionScreen::CharacterSelect);
            self.clear_reconnect();
        }
        effects.push(SessionEffect::Transition(self.screen));
        effects
    }

    pub fn receive_registration(&mut self, response: RegisterResponse) -> Vec<SessionEffect> {
        if !response.success {
            self.feedback = Some(response.error.unwrap_or_default());
            self.screen = SessionScreen::Login;
            return vec![SessionEffect::Transition(self.screen)];
        }
        self.token = Some(response.token.clone());
        self.feedback = None;
        self.characters.clear();
        self.screen = SessionScreen::CharacterSelect;
        vec![
            SessionEffect::PersistToken(response.token),
            SessionEffect::Transition(self.screen),
        ]
    }

    pub fn select_character(&self) -> Option<SelectCharacter> {
        let index = self.selected_index?;
        let character = self.characters.get(index)?;
        Some(SelectCharacter {
            character_id: character.character_id,
        })
    }

    pub fn receive_character_created(
        &mut self,
        response: CreateCharacterResponse,
    ) -> CharacterCreationResult {
        let result = CharacterCreationResult {
            success: response.success,
            error: response.error,
        };
        if response.success {
            if let Some(character) = response.character {
                self.characters.push(character);
            }
        }
        result
    }

    pub fn receive_character_deleted(&mut self, response: DeleteCharacterResponse) {
        if response.success {
            self.characters
                .retain(|character| character.character_id != response.character_id);
        }
    }

    pub fn receive_character_update(&mut self, update: CharacterListUpdate) {
        if let Some(existing) = self
            .characters
            .iter_mut()
            .find(|entry| entry.character_id == update.character.character_id)
        {
            *existing = update.character;
        } else {
            self.characters.push(update.character);
        }
    }

    pub fn receive_enter_world(&mut self, response: EnterWorldResponse) -> Vec<SessionEffect> {
        if !response.success {
            self.clear_reconnect();
            self.screen = SessionScreen::CharacterSelect;
            return vec![SessionEffect::Transition(self.screen)];
        }
        if let Some(entry) = self
            .selected_index
            .and_then(|index| self.characters.get(index))
        {
            self.selected_character_id = Some(entry.character_id);
            self.selected_character_name = Some(entry.name.clone());
        }
        self.screen = SessionScreen::Loading;
        vec![SessionEffect::Transition(self.screen)]
    }

    /// Client and server protocols differ. The transport drops the link itself; the reason
    /// becomes the login feedback when `Disconnected` arrives, with no reconnect.
    pub fn receive_protocol_rejected(&mut self, reason: String) {
        self.pending_forced_disconnect = Some(ForcedDisconnect {
            message: reason,
            reconnect_allowed: false,
        });
    }

    /// Preserve notice until the host reports actual disconnection, as the old lifecycle did.
    pub fn receive_forced_disconnect(&mut self, notice: ForcedDisconnect) -> Vec<SessionEffect> {
        self.pending_forced_disconnect = Some(notice);
        vec![SessionEffect::RequestDisconnect]
    }

    pub fn receive_disconnected_with_reason(&mut self, reason: Option<&str>) -> Vec<SessionEffect> {
        self.lose_connection(reason, CONNECTION_LOST)
    }

    /// The transport gave up connecting for `reason`. Handled as any loss, except that a
    /// return to the login screen shows `reason` instead of "Connection lost.".
    pub fn receive_connect_failed(&mut self, reason: &str) -> Vec<SessionEffect> {
        self.lose_connection(Some(reason), reason)
    }

    /// `lost` is the login feedback of a loss that ends on the login screen.
    fn lose_connection(&mut self, reason: Option<&str>, lost: &str) -> Vec<SessionEffect> {
        if reason.is_none()
            && self.pending_forced_disconnect.is_none()
            && self.reconnect_phase == ReconnectPhase::PendingConnect
        {
            return Vec::new();
        }
        self.reset_world_port();
        if let Some(notice) = self.pending_forced_disconnect.take() {
            self.clear_reconnect();
            self.feedback = Some(notice.message);
            let mut effects = vec![SessionEffect::ResetNetworkWorld];
            if self.screen != SessionScreen::Login {
                self.screen = SessionScreen::Login;
                effects.push(SessionEffect::Transition(self.screen));
            }
            effects.push(SessionEffect::ShowFeedback);
            return effects;
        }
        match self.screen {
            SessionScreen::CharacterSelect
                if self
                    .token
                    .as_deref()
                    .and_then(normalize_auth_token)
                    .is_some() =>
            {
                self.begin_reconnect(false);
                vec![SessionEffect::ResetNetworkWorld]
            }
            SessionScreen::CharacterSelect => {
                self.clear_reconnect();
                self.feedback = Some("Connection lost. Char select is now offline.".into());
                vec![SessionEffect::ShowFeedback]
            }
            SessionScreen::InWorld | SessionScreen::Loading
                if self
                    .token
                    .as_deref()
                    .and_then(normalize_auth_token)
                    .is_some() =>
            {
                self.begin_reconnect(true);
                self.screen = SessionScreen::InWorld;
                vec![
                    SessionEffect::ResetNetworkWorld,
                    SessionEffect::Transition(self.screen),
                ]
            }
            SessionScreen::InWorld | SessionScreen::Loading | SessionScreen::CharacterCreate => {
                self.clear_reconnect();
                self.feedback = Some(lost.to_owned());
                self.screen = SessionScreen::Login;
                vec![
                    SessionEffect::Transition(self.screen),
                    SessionEffect::ShowFeedback,
                ]
            }
            SessionScreen::Login | SessionScreen::GameMenu => {
                self.clear_reconnect();
                self.feedback = Some(lost.to_owned());
                vec![SessionEffect::ShowFeedback]
            }
        }
    }

    fn begin_reconnect(&mut self, auto_enter_world: bool) {
        self.reconnect_phase = ReconnectPhase::PendingConnect;
        self.terrain_refresh_seen = false;
        self.reconnect_auto_enter_world = auto_enter_world;
        self.reconnect_preselected_name = auto_enter_world
            .then(|| self.selected_character_name.clone())
            .flatten();
        self.feedback = None;
    }
}

fn resolve_selected_index(
    characters: &[CharacterListEntry],
    preselected_name: Option<&str>,
) -> Option<usize> {
    preselected_name
        .and_then(|name| {
            characters
                .iter()
                .position(|character| character.name.eq_ignore_ascii_case(name))
        })
        .or_else(|| characters.first().map(|_| 0))
}

fn user_facing_login_error(error: &str) -> &str {
    let normalized = error.trim().to_ascii_lowercase();
    if [
        "invalid",
        "incorrect",
        "wrong password",
        "bad password",
        "credentials",
        "password",
    ]
    .iter()
    .any(|term| normalized.contains(term))
    {
        "Incorrect username or password"
    } else if ["banned", "pending admin approval", "register first"]
        .iter()
        .any(|term| normalized.contains(term))
    {
        error
    } else {
        "Login failed. Please try again."
    }
}
