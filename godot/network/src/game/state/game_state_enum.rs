pub use crate::screen_arg_data::ScreenArg;
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
    M2Debug,
    SkyboxDebug,
    CharCreate,
    CampsitePopup,
    Loading,
    InWorld,
    GameMenu,
    TrashButton,
    Reconnecting,
    ParticleDebug,
    NameplateDebug,
}

impl GameState {
    pub const CLI_VALUES: [&str; 18] = [
        "login",
        "eula",
        "connecting",
        "charselect",
        "selectiondebug",
        "inworldselectiondebug",
        "debugcharacter",
        "m2debug",
        "skyboxdebug",
        "charcreate",
        "campsitepopup",
        "loading",
        "inworld",
        "gamemenu",
        "trashbutton",
        "reconnecting",
        "particledebug",
        "nameplatedebug",
    ];

    pub fn is_logged_in(self) -> bool {
        !matches!(
            self,
            Self::Login
                | Self::Eula
                | Self::Connecting
                | Self::SelectionDebug
                | Self::InWorldSelectionDebug
                | Self::NameplateDebug
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
            Self::M2Debug => "m2debug",
            Self::SkyboxDebug => "skyboxdebug",
            Self::CharCreate => "charcreate",
            Self::CampsitePopup => "campsitepopup",
            Self::Loading => "loading",
            Self::InWorld => "inworld",
            Self::GameMenu => "gamemenu",
            Self::TrashButton => "trashbutton",
            Self::Reconnecting => "reconnecting",
            Self::ParticleDebug => "particledebug",
            Self::NameplateDebug => "nameplatedebug",
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

impl From<ScreenArg> for GameState {
    fn from(value: ScreenArg) -> Self {
        match value {
            ScreenArg::Login => Self::Login,
            ScreenArg::Eula => Self::Eula,
            ScreenArg::CharSelect => Self::CharSelect,
            ScreenArg::SelectionDebug => Self::SelectionDebug,
            ScreenArg::InWorldSelectionDebug => Self::InWorldSelectionDebug,
            ScreenArg::DebugCharacter => Self::DebugCharacter,
            ScreenArg::M2Debug => Self::M2Debug,
            ScreenArg::SkyboxDebug => Self::SkyboxDebug,
            ScreenArg::CharCreate | ScreenArg::CharCreateCustomize => Self::CharCreate,
            ScreenArg::CampsitePopup => Self::CampsitePopup,
            ScreenArg::Loading => Self::Loading,
            ScreenArg::InWorld => Self::InWorld,
            ScreenArg::GameMenu | ScreenArg::OptionsMenu => Self::GameMenu,
            ScreenArg::TrashButton => Self::TrashButton,
            ScreenArg::ParticleDebug => Self::ParticleDebug,
            ScreenArg::NameplateDebug => Self::NameplateDebug,
        }
    }
}
