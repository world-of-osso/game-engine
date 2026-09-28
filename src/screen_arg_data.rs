use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenArg {
    Login,
    Eula,
    CharSelect,
    SelectionDebug,
    InWorldSelectionDebug,
    DebugCharacter,
    M2Debug,
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
    NameplateDebug,
}

impl ScreenArg {
    pub const CLI_VALUES: [&str; 18] = [
        "login",
        "eula",
        "charselect",
        "selectiondebug",
        "inworldselectiondebug",
        "debugcharacter",
        "m2debug",
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
        "nameplatedebug",
    ];

    pub fn as_cli_str(self) -> &'static str {
        match self {
            Self::Login => "login",
            Self::Eula => "eula",
            Self::CharSelect => "charselect",
            Self::SelectionDebug => "selectiondebug",
            Self::InWorldSelectionDebug => "inworldselectiondebug",
            Self::DebugCharacter => "debugcharacter",
            Self::M2Debug => "m2debug",
            Self::SkyboxDebug => "skyboxdebug",
            Self::CharCreate => "charcreate",
            Self::CharCreateCustomize => "charcreate-customize",
            Self::CampsitePopup => "campsitepopup",
            Self::Loading => "loading",
            Self::InWorld => "inworld",
            Self::GameMenu => "gamemenu",
            Self::OptionsMenu => "optionsmenu",
            Self::TrashButton => "trashbutton",
            Self::ParticleDebug => "particledebug",
            Self::NameplateDebug => "nameplatedebug",
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
            "m2debug" | "m2-debug" => Ok(Self::M2Debug),
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
            "nameplatedebug" | "nameplate-debug" => Ok(Self::NameplateDebug),
            _ => Err(format!("expected one of: {}", Self::CLI_VALUES.join(", "))),
        }
    }
}
