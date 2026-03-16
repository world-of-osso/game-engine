use bevy::prelude::*;
use std::str::FromStr;

/// Game state machine controlling which systems are active.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    Login,
    Connecting,
    CharSelect,
    CharCreate,
    Loading,
    InWorld,
    Reconnecting,
}

impl GameState {
    pub const CLI_VALUES: [&str; 7] = [
        "login",
        "connecting",
        "charselect",
        "charcreate",
        "loading",
        "inworld",
        "reconnecting",
    ];

    pub fn as_cli_str(self) -> &'static str {
        match self {
            Self::Login => "login",
            Self::Connecting => "connecting",
            Self::CharSelect => "charselect",
            Self::CharCreate => "charcreate",
            Self::Loading => "loading",
            Self::InWorld => "inworld",
            Self::Reconnecting => "reconnecting",
        }
    }
}

impl FromStr for GameState {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "login" => Ok(Self::Login),
            "connecting" => Ok(Self::Connecting),
            "charselect" => Ok(Self::CharSelect),
            "charcreate" => Ok(Self::CharCreate),
            "loading" => Ok(Self::Loading),
            "inworld" => Ok(Self::InWorld),
            "reconnecting" => Ok(Self::Reconnecting),
            _ => Err(format!("expected one of: {}", Self::CLI_VALUES.join(", "))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenArg {
    Login,
    CharSelect,
    CharCreate,
    CharCreateCustomize,
    InWorld,
}

impl ScreenArg {
    pub const CLI_VALUES: [&str; 5] = [
        "login",
        "charselect",
        "charcreate",
        "charcreate-customize",
        "inworld",
    ];
}

impl From<ScreenArg> for GameState {
    fn from(value: ScreenArg) -> Self {
        match value {
            ScreenArg::Login => Self::Login,
            ScreenArg::CharSelect => Self::CharSelect,
            ScreenArg::CharCreate | ScreenArg::CharCreateCustomize => Self::CharCreate,
            ScreenArg::InWorld => Self::InWorld,
        }
    }
}

impl FromStr for ScreenArg {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "login" => Ok(Self::Login),
            "charselect" => Ok(Self::CharSelect),
            "charcreate" => Ok(Self::CharCreate),
            "charcreate-customize" => Ok(Self::CharCreateCustomize),
            "inworld" => Ok(Self::InWorld),
            _ => Err(format!("expected one of: {}", Self::CLI_VALUES.join(", "))),
        }
    }
}
