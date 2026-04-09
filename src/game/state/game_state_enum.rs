use bevy::prelude::*;
use std::str::FromStr;

/// Game state machine controlling which systems are active.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    Login,
    Eula,
    Connecting,
    CharSelect,
    SelectionDebug,
    InWorldSelectionDebug,
    DebugCharacter,
    SkyboxDebug,
    CharCreate,
    CampsitePopup,
    Loading,
    InWorld,
    GameMenu,
    TrashButton,
    Reconnecting,
    ParticleDebug,
}

impl GameState {
    pub const CLI_VALUES: [&str; 16] = [
        "login",
        "eula",
        "connecting",
        "charselect",
        "selectiondebug",
        "inworldselectiondebug",
        "debugcharacter",
        "skyboxdebug",
        "charcreate",
        "campsitepopup",
        "loading",
        "inworld",
        "gamemenu",
        "trashbutton",
        "reconnecting",
        "particledebug",
    ];

    pub fn is_logged_in(self) -> bool {
        !matches!(
            self,
            Self::Login
                | Self::Eula
                | Self::Connecting
                | Self::SelectionDebug
                | Self::InWorldSelectionDebug
        )
    }

    pub fn as_cli_str(self) -> &'static str {
        match self {
            Self::Login => "login",
            Self::Eula => "eula",
            Self::Connecting => "connecting",
            Self::CharSelect => "charselect",
            Self::SelectionDebug => "selectiondebug",
            Self::InWorldSelectionDebug => "inworldselectiondebug",
            Self::DebugCharacter => "debugcharacter",
            Self::SkyboxDebug => "skyboxdebug",
            Self::CharCreate => "charcreate",
            Self::CampsitePopup => "campsitepopup",
            Self::Loading => "loading",
            Self::InWorld => "inworld",
            Self::GameMenu => "gamemenu",
            Self::TrashButton => "trashbutton",
            Self::Reconnecting => "reconnecting",
            Self::ParticleDebug => "particledebug",
        }
    }
}

impl FromStr for GameState {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "connecting" => Ok(Self::Connecting),
            "reconnecting" => Ok(Self::Reconnecting),
            _ => ScreenArg::from_str(value)
                .map(Self::from)
                .map_err(|_| format!("expected one of: {}", Self::CLI_VALUES.join(", "))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenArg {
    Login,
    Eula,
    CharSelect,
    SelectionDebug,
    InWorldSelectionDebug,
    DebugCharacter,
    SkyboxDebug,
    CharCreate,
    CharCreateCustomize,
    CampsitePopup,
    Loading,
    InWorld,
    GameMenu,
    OptionsMenu,
    TrashButton,
    ParticleDebug,
}

impl ScreenArg {
    pub const CLI_VALUES: [&str; 16] = [
        "login",
        "eula",
        "charselect",
        "selectiondebug",
        "inworldselectiondebug",
        "debugcharacter",
        "skyboxdebug",
        "charcreate",
        "charcreate-customize",
        "campsitepopup",
        "loading",
        "inworld",
        "gamemenu",
        "optionsmenu",
        "trashbutton",
        "particledebug",
    ];
}

impl From<ScreenArg> for GameState {
    fn from(value: ScreenArg) -> Self {
        match value {
            ScreenArg::Login => Self::Login,
            ScreenArg::Eula => Self::Eula,
            ScreenArg::CharSelect => Self::CharSelect,
            ScreenArg::SelectionDebug => Self::SelectionDebug,
            ScreenArg::InWorldSelectionDebug => Self::InWorldSelectionDebug,
            ScreenArg::DebugCharacter => Self::DebugCharacter,
            ScreenArg::SkyboxDebug => Self::SkyboxDebug,
            ScreenArg::CharCreate | ScreenArg::CharCreateCustomize => Self::CharCreate,
            ScreenArg::CampsitePopup => Self::CampsitePopup,
            ScreenArg::Loading => Self::Loading,
            ScreenArg::InWorld => Self::InWorld,
            ScreenArg::GameMenu | ScreenArg::OptionsMenu => Self::GameMenu,
            ScreenArg::TrashButton => Self::TrashButton,
            ScreenArg::ParticleDebug => Self::ParticleDebug,
        }
    }
}

impl FromStr for ScreenArg {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "login" => Ok(Self::Login),
            "eula" => Ok(Self::Eula),
            "charselect" => Ok(Self::CharSelect),
            "selectiondebug" => Ok(Self::SelectionDebug),
            "inworldselectiondebug" | "inworld-selectiondebug" => Ok(Self::InWorldSelectionDebug),
            "debugcharacter" => Ok(Self::DebugCharacter),
            "skyboxdebug" | "skybox-debug" => Ok(Self::SkyboxDebug),
            "charcreate" => Ok(Self::CharCreate),
            "charcreate-customize" => Ok(Self::CharCreateCustomize),
            "campsitepopup" => Ok(Self::CampsitePopup),
            "loading" => Ok(Self::Loading),
            "inworld" => Ok(Self::InWorld),
            "gamemenu" | "menu" => Ok(Self::GameMenu),
            "optionsmenu" | "options" => Ok(Self::OptionsMenu),
            "trashbutton" => Ok(Self::TrashButton),
            "particledebug" => Ok(Self::ParticleDebug),
            _ => Err(format!("expected one of: {}", Self::CLI_VALUES.join(", "))),
        }
    }
}
