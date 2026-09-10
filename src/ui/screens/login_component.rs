use std::fmt;

use crate::ui::anchor::FrameName;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginAction {
    Connect,
    Reconnect,
    CycleRealm,
    CreateAccount,
    Menu,
    Exit,
}

impl fmt::Display for LoginAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connect => f.write_str("connect"),
            Self::Reconnect => f.write_str("reconnect"),
            Self::CycleRealm => f.write_str("cycle_realm"),
            Self::CreateAccount => f.write_str("create_account"),
            Self::Menu => f.write_str("menu"),
            Self::Exit => f.write_str("exit"),
        }
    }
}

impl LoginAction {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "connect" => Some(Self::Connect),
            "reconnect" => Some(Self::Reconnect),
            "cycle_realm" => Some(Self::CycleRealm),
            "create_account" => Some(Self::CreateAccount),
            "menu" => Some(Self::Menu),
            "exit" => Some(Self::Exit),
            _ => None,
        }
    }
}

pub const LOGIN_ROOT: FrameName = FrameName("LoginRoot");
pub const USERNAME_INPUT: FrameName = FrameName("UsernameInput");
pub const PASSWORD_INPUT: FrameName = FrameName("PasswordInput");
pub const CONNECT_BUTTON: FrameName = FrameName("ConnectButton");
pub const RECONNECT_BUTTON: FrameName = FrameName("ReconnectButton");
pub const REALM_BUTTON: FrameName = FrameName("RealmButton");
pub const EXIT_BUTTON: FrameName = FrameName("ExitButton");
pub const CREATE_ACCOUNT_BUTTON: FrameName = FrameName("CreateAccountButton");
pub const MENU_BUTTON: FrameName = FrameName("MenuButton");
pub const LOGIN_STATUS: FrameName = FrameName("LoginStatus");
