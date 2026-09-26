//! Registry-authoritative login UI model. Rendering and authentication belong to the host.

pub mod ui {
    pub use ui_toolkit::{anchor, strata};

    pub mod widgets {
        pub use ui_toolkit::widgets::font_string;
    }

    pub mod screens {
        pub use crate::trash_button_component;
    }
}

#[path = "../../../src/ui/screens/campsite_component.rs"]
pub mod campsite_component;
#[path = "../../../src/ui/screens/char_select_component.rs"]
pub mod char_select_component;
#[path = "../../../src/ui/screens/char_select_delete_confirm_component.rs"]
mod char_select_delete_confirm_component;
#[path = "../../../src/ui/screens/trash_button_component.rs"]
pub mod trash_button_component;

#[path = "../../../src/ui/screens/login_component.rs"]
pub mod login;

use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

use char_select_component::{
    CharDisplayEntry, CharSelectState, DeleteConfirmUiState, char_select_screen,
};
use login::{
    PASSWORD_INPUT, SharedConnecting, SharedRealmSelectable, SharedRealmText, SharedStatusText,
    USERNAME_INPUT, login_screen,
};
use shared::protocol::CharacterListEntry;

/// Convert the protocol roster and session-selected index into the original character-select UI text.
/// Selection policy (including first-character default and name preselection) belongs to the session.
pub fn char_select_state_from_roster(
    characters: &[CharacterListEntry],
    selected_index: Option<usize>,
) -> CharSelectState {
    let entries = characters
        .iter()
        .map(|character| CharDisplayEntry {
            name: character.name.clone(),
            info: format!(
                "Level {}   Race {}   Class {}",
                character.level, character.race, character.class
            ),
            status: "Ready to enter world".to_owned(),
        })
        .collect();
    let selected = selected_index.and_then(|index| characters.get(index));
    let selected_name = selected
        .map(|character| character.name.clone())
        .unwrap_or_else(|| "Character Selection".to_owned());
    let status_text = match selected {
        Some(character) => format!(
            "Realm: World of Osso    Level {}    Race {}    Class {}",
            character.level, character.race, character.class
        ),
        None if characters.is_empty() => "No characters available on this realm".to_owned(),
        None => "Select a character to enter the world".to_owned(),
    };
    CharSelectState {
        characters: entries,
        selected_index,
        selected_name,
        status_text,
    }
}

/// Authored character selection tree. The host owns roster, action routing and deletion effects.
pub struct CharacterSelectModel {
    pub screen: Screen,
    pub shared: SharedContext,
    pub registry: FrameRegistry,
}

impl CharacterSelectModel {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        let mut shared = SharedContext::new();
        shared.insert(CharSelectState::default());
        shared.insert(DeleteConfirmUiState::default());
        Self {
            screen: Screen::new(char_select_screen),
            shared,
            registry: FrameRegistry::new(screen_width, screen_height),
        }
    }

    pub fn sync(&mut self) {
        self.screen.sync(&self.shared, &mut self.registry);
    }
}

/// Authored login tree plus mutable input and reactive state; no renderer or auth client.
pub struct LoginModel {
    pub screen: Screen,
    pub shared: SharedContext,
    pub registry: FrameRegistry,
}

impl LoginModel {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        let mut shared = SharedContext::new();
        shared.insert(SharedStatusText(String::new()));
        shared.insert(SharedConnecting(false));
        shared.insert(SharedRealmText("Development".into()));
        shared.insert(SharedRealmSelectable(true));
        Self {
            screen: Screen::new(login_screen),
            shared,
            registry: FrameRegistry::new(screen_width, screen_height),
        }
    }

    pub fn sync(&mut self) {
        self.screen.sync(&self.shared, &mut self.registry);
    }

    /// Current entered values, or None before the login tree is first synced.
    pub fn credentials(&self) -> Option<(String, String)> {
        let username = self.editbox_text(USERNAME_INPUT.0)?;
        let password = self.editbox_text(PASSWORD_INPUT.0)?;
        Some((username.to_owned(), password.to_owned()))
    }

    fn editbox_text(&self, name: &str) -> Option<&str> {
        let id = self.registry.get_by_name(name)?;
        let frame = self.registry.get(id)?;
        let WidgetData::EditBox(data) = frame.widget_data.as_ref()? else {
            return None;
        };
        Some(&data.text)
    }
}
