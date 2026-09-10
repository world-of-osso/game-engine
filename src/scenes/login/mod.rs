use bevy::prelude::*;
use std::net::ToSocketAddrs;
use std::sync::Arc;

use crate::game_state::GameState;
use crate::networking;

mod form;
pub mod helpers;
mod native;
mod native_caret;
mod native_view;

const FADE_IN_DURATION: f32 = 0.75;
pub(crate) const DEFAULT_SERVER_ADDR: &str = crate::cli_args::DEFAULT_SERVER_ADDR;
pub(crate) const STATUS_CONNECTING: &str = "Connecting...";
pub(crate) const STATUS_FILL_FIELDS: &str = "Please fill in all fields";
pub(crate) const STATUS_RECONNECT_UNAVAILABLE: &str = "No saved session to reconnect";

#[derive(Resource, Debug, Clone, Copy, Default)]
pub(crate) struct LoginRealmSelectionLock(pub bool);

#[derive(Debug, Clone, PartialEq, Eq)]
enum LoginRealmChoice {
    Preset(crate::cli_args::RealmPreset),
    Custom {
        addr: std::net::SocketAddr,
        hostname: String,
    },
}

#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoginRealmSelection {
    choice: LoginRealmChoice,
    locked: bool,
}

impl LoginRealmSelection {
    fn from_server(
        server_addr: Option<std::net::SocketAddr>,
        server_hostname: Option<&str>,
        locked: bool,
    ) -> Self {
        let choice = match (server_addr, server_hostname) {
            (_, Some(hostname)) => crate::cli_args::realm_preset_for_hostname(hostname)
                .map(LoginRealmChoice::Preset)
                .unwrap_or_else(|| LoginRealmChoice::Custom {
                    addr: server_addr.unwrap_or_else(resolve_default_server),
                    hostname: hostname.to_string(),
                }),
            (Some(addr), None) => LoginRealmChoice::Custom {
                addr,
                hostname: addr.to_string(),
            },
            (None, None) => LoginRealmChoice::Preset(crate::client_options::load_preferred_realm()),
        };
        Self { choice, locked }
    }

    fn server_addr(&self) -> Result<std::net::SocketAddr, String> {
        match &self.choice {
            LoginRealmChoice::Preset(preset) => preset.to_server_arg().map(|server| server.addr),
            LoginRealmChoice::Custom { addr, .. } => Ok(*addr),
        }
    }

    fn server_hostname(&self) -> String {
        match &self.choice {
            LoginRealmChoice::Preset(preset) => preset.hostname().to_string(),
            LoginRealmChoice::Custom { hostname, .. } => hostname.clone(),
        }
    }

    fn is_dev(&self) -> bool {
        matches!(
            self.choice,
            LoginRealmChoice::Preset(crate::cli_args::RealmPreset::Dev)
        )
    }

    fn cycle(&mut self) {
        if self.locked {
            return;
        }
        self.choice = match self.choice {
            LoginRealmChoice::Preset(crate::cli_args::RealmPreset::Dev) => {
                LoginRealmChoice::Preset(crate::cli_args::RealmPreset::Prod)
            }
            LoginRealmChoice::Preset(crate::cli_args::RealmPreset::Prod)
            | LoginRealmChoice::Custom { .. } => {
                LoginRealmChoice::Preset(crate::cli_args::RealmPreset::Dev)
            }
        };
    }

    #[cfg(not(test))]
    fn selected_preset(&self) -> Option<crate::cli_args::RealmPreset> {
        match self.choice {
            LoginRealmChoice::Preset(preset) => Some(preset),
            LoginRealmChoice::Custom { .. } => None,
        }
    }
}

fn resolve_default_server() -> std::net::SocketAddr {
    DEFAULT_SERVER_ADDR
        .to_socket_addrs()
        .ok()
        .and_then(|mut addrs| addrs.next())
        .unwrap_or_else(|| "127.0.0.1:5000".parse().unwrap())
}

fn selected_login_server(
    selection: Option<&LoginRealmSelection>,
    server_addr: Option<std::net::SocketAddr>,
    server_hostname: Option<&str>,
) -> Result<(std::net::SocketAddr, String), String> {
    if let Some(selection) = selection {
        return Ok((selection.server_addr()?, selection.server_hostname()));
    }
    Ok((
        server_addr.unwrap_or_else(resolve_default_server),
        server_hostname.unwrap_or(DEFAULT_SERVER_ADDR).to_string(),
    ))
}

fn apply_login_realm_resources(
    commands: &mut Commands,
    selection: &LoginRealmSelection,
) -> Result<(), String> {
    let addr = selection.server_addr()?;
    let hostname = selection.server_hostname();
    commands.insert_resource(networking::ServerAddr(addr));
    commands.insert_resource(networking::ServerHostname(hostname.clone()));
    commands.insert_resource(networking::AuthToken(networking::load_auth_token(Some(
        hostname.as_str(),
    ))));
    Ok(())
}

#[cfg(not(test))]
fn persist_login_realm_selection(selection: &LoginRealmSelection) {
    if let Some(preset) = selection.selected_preset()
        && let Err(error) = crate::client_options::save_preferred_realm(preset)
    {
        warn!(
            "Failed to save preferred realm '{}': {error}",
            preset.alias()
        );
    }
}

#[cfg(test)]
fn persist_login_realm_selection(_selection: &LoginRealmSelection) {}

#[derive(Default)]
struct LoginModifierState {
    ctrl: bool,
    super_key: bool,
}

#[derive(Resource, Default)]
pub(crate) struct LoginStatus(pub(crate) String);

#[derive(Resource)]
pub(crate) struct DevServer;

#[derive(Resource, Clone)]
struct LoginClipboard(Arc<dyn Fn() -> Result<String, String> + Send + Sync>);

impl Default for LoginClipboard {
    fn default() -> Self {
        Self(Arc::new(|| {
            let mut clipboard =
                arboard::Clipboard::new().map_err(|error| format!("clipboard init: {error}"))?;
            clipboard
                .get_text()
                .map_err(|error| format!("clipboard read: {error}"))
        }))
    }
}

impl LoginClipboard {
    fn read_text(&self) -> Result<String, String> {
        (self.0)()
    }
}

pub struct LoginScreenPlugin;

impl Plugin for LoginScreenPlugin {
    fn build(&self, app: &mut App) {
        native::register(app);
    }
}
