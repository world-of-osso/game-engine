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
    CampsitePopup,
    Loading,
    InWorld,
    GameMenu,
    TrashButton,
    Reconnecting,
}

impl GameState {
    pub const CLI_VALUES: [&str; 10] = [
        "login",
        "connecting",
        "charselect",
        "charcreate",
        "campsitepopup",
        "loading",
        "inworld",
        "gamemenu",
        "trashbutton",
        "reconnecting",
    ];

    pub fn is_logged_in(self) -> bool {
        !matches!(self, Self::Login | Self::Connecting)
    }

    pub fn as_cli_str(self) -> &'static str {
        match self {
            Self::Login => "login",
            Self::Connecting => "connecting",
            Self::CharSelect => "charselect",
            Self::CharCreate => "charcreate",
            Self::CampsitePopup => "campsitepopup",
            Self::Loading => "loading",
            Self::InWorld => "inworld",
            Self::GameMenu => "gamemenu",
            Self::TrashButton => "trashbutton",
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
            "campsitepopup" => Ok(Self::CampsitePopup),
            "loading" => Ok(Self::Loading),
            "inworld" => Ok(Self::InWorld),
            "gamemenu" | "menu" => Ok(Self::GameMenu),
            "trashbutton" => Ok(Self::TrashButton),
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
    CampsitePopup,
    InWorld,
    GameMenu,
    TrashButton,
}

impl ScreenArg {
    pub const CLI_VALUES: [&str; 8] = [
        "login",
        "charselect",
        "charcreate",
        "charcreate-customize",
        "campsitepopup",
        "inworld",
        "gamemenu",
        "trashbutton",
    ];
}

impl From<ScreenArg> for GameState {
    fn from(value: ScreenArg) -> Self {
        match value {
            ScreenArg::Login => Self::Login,
            ScreenArg::CharSelect => Self::CharSelect,
            ScreenArg::CharCreate | ScreenArg::CharCreateCustomize => Self::CharCreate,
            ScreenArg::CampsitePopup => Self::CampsitePopup,
            ScreenArg::InWorld => Self::InWorld,
            ScreenArg::GameMenu => Self::GameMenu,
            ScreenArg::TrashButton => Self::TrashButton,
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
            "campsitepopup" => Ok(Self::CampsitePopup),
            "inworld" => Ok(Self::InWorld),
            "gamemenu" | "menu" => Ok(Self::GameMenu),
            "trashbutton" => Ok(Self::TrashButton),
            _ => Err(format!("expected one of: {}", Self::CLI_VALUES.join(", "))),
        }
    }
}
