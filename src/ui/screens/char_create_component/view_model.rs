use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraControl {
    Reset,
    ZoomIn,
    ZoomOut,
    RotateLeft,
    RotateRight,
}

impl CameraControl {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reset => "reset",
            Self::ZoomIn => "zoom_in",
            Self::ZoomOut => "zoom_out",
            Self::RotateLeft => "rotate_left",
            Self::RotateRight => "rotate_right",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "reset" => Some(Self::Reset),
            "zoom_in" => Some(Self::ZoomIn),
            "zoom_out" => Some(Self::ZoomOut),
            "rotate_left" => Some(Self::RotateLeft),
            "rotate_right" => Some(Self::RotateRight),
            _ => None,
        }
    }
}

/// Action strings are also the named-frame automation contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharCreateAction {
    SelectRace(u8),
    SelectClass(u8),
    SelectSex(u8),
    Randomize,
    NextMode,
    Back,
    SelectCategory(u32),
    AdjustOption(u32, i8),
    ToggleOption(u32),
    SelectOptionChoice(u32, u32),
    Camera(CameraControl),
    CreateConfirm,
}

impl fmt::Display for CharCreateAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SelectRace(id) => write!(f, "select_race:{id}"),
            Self::SelectClass(id) => write!(f, "select_class:{id}"),
            Self::SelectSex(sex) => write!(f, "select_sex:{sex}"),
            Self::Randomize => f.write_str("randomize"),
            Self::NextMode => f.write_str("next_mode"),
            Self::Back => f.write_str("back"),
            Self::SelectCategory(id) => write!(f, "select_category:{id}"),
            Self::AdjustOption(id, delta) => write!(f, "adjust_option:{id}:{delta}"),
            Self::ToggleOption(id) => write!(f, "toggle_option:{id}"),
            Self::SelectOptionChoice(option, choice) => {
                write!(f, "select_option_choice:{option}:{choice}")
            }
            Self::Camera(control) => write!(f, "camera:{}", control.as_str()),
            Self::CreateConfirm => f.write_str("create_confirm"),
        }
    }
}

impl CharCreateAction {
    pub fn parse(value: &str) -> Option<Self> {
        let parts = value.split(':').collect::<Vec<_>>();
        match parts.as_slice() {
            ["select_race", id] => id.parse().ok().map(Self::SelectRace),
            ["select_class", id] => id.parse().ok().map(Self::SelectClass),
            ["select_sex", sex] => sex
                .parse::<u8>()
                .ok()
                .filter(|sex| *sex <= 1)
                .map(Self::SelectSex),
            ["select_category", id] => id.parse().ok().map(Self::SelectCategory),
            ["adjust_option", id, delta] => {
                let delta = delta
                    .parse::<i8>()
                    .ok()
                    .filter(|delta| matches!(delta, -1 | 1))?;
                Some(Self::AdjustOption(id.parse().ok()?, delta))
            }
            ["toggle_option", id] => id.parse().ok().map(Self::ToggleOption),
            ["select_option_choice", option, choice] => Some(Self::SelectOptionChoice(
                option.parse().ok()?,
                choice.parse().ok()?,
            )),
            ["camera", control] => CameraControl::parse(control).map(Self::Camera),
            ["randomize"] => Some(Self::Randomize),
            ["next_mode"] => Some(Self::NextMode),
            ["back"] => Some(Self::Back),
            ["create_confirm"] => Some(Self::CreateConfirm),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharCreateMode {
    RaceClass,
    Customize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomizationCategoryUi {
    pub id: u32,
    pub label: String,
    pub icon_atlas: Option<String>,
    pub selected_icon_atlas: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomizationChoiceUi {
    pub id: u32,
    pub label: String,
    pub swatch: Option<[u8; 3]>,
    pub secondary_swatch: Option<[u8; 3]>,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomizationOptionUi {
    pub id: u32,
    pub label: String,
    /// Local Enum.ChrCustomizationOptionType: dropdown=0, checkbox=1, slider=2.
    pub ui_type: u32,
    pub selected_choice_id: u32,
    pub choices: Vec<CustomizationChoiceUi>,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharCreateUiState {
    pub mode: CharCreateMode,
    pub selected_race: u8,
    pub selected_class: u8,
    pub selected_sex: u8,
    pub categories: Vec<CustomizationCategoryUi>,
    pub selected_category: u32,
    /// The selected category's choices, already filtered and ordered by the catalog.
    pub options: Vec<CustomizationOptionUi>,
    pub open_dropdown: Option<u32>,
    pub name: String,
    pub error_text: Option<String>,
    pub name_input_focused: bool,
    /// (class_id, class_name, icon_fdid, available_for_race)
    pub class_availability: Vec<(u8, &'static str, u32, bool)>,
    pub viewport_width: u32,
    pub viewport_height: u32,
}

impl Default for CharCreateUiState {
    fn default() -> Self {
        use crate::char_create_data::{CLASSES, race_can_be_class};
        Self {
            mode: CharCreateMode::RaceClass,
            selected_race: 1,
            selected_class: 1,
            selected_sex: 0,
            categories: Vec::new(),
            selected_category: 0,
            options: Vec::new(),
            open_dropdown: None,
            name: String::new(),
            error_text: None,
            name_input_focused: false,
            class_availability: CLASSES
                .iter()
                .map(|class| {
                    (
                        class.id,
                        class.name,
                        class.icon_fdid,
                        race_can_be_class(1, class.id),
                    )
                })
                .collect(),
            viewport_width: 1920,
            viewport_height: 1080,
        }
    }
}
