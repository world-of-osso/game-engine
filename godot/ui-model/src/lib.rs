//! Registry-authoritative login UI model. Rendering and authentication belong to the host.

pub mod ui {
    pub use ui_toolkit::{anchor, strata};

    pub use ui_toolkit::{frame, layout, registry};

    pub mod widgets {
        pub use ui_toolkit::widgets::{font_string, texture};
    }

    pub mod screens {
        pub(crate) use crate::screen_title;
        pub use crate::{default_button_atlas, trash_button_component};
    }

    pub use crate::ui_errors_data;
}

#[path = "../../../src/ui/screens/char_create_component/mod.rs"]
pub mod char_create_component;
#[path = "../../../src/scenes/char_create/data.rs"]
pub mod char_create_data;

#[path = "../../../src/ui/screens/campsite_component.rs"]
pub mod campsite_component;
#[path = "../../../src/ui/screens/char_select_component.rs"]
pub mod char_select_component;
#[path = "../../../src/ui/screens/char_select_delete_confirm_component.rs"]
mod char_select_delete_confirm_component;
#[path = "../../../src/ui/screens/char_select_top_nav_component.rs"]
pub mod char_select_top_nav_component;
#[path = "../../../src/ui/screens/default_button_atlas.rs"]
pub mod default_button_atlas;
#[path = "../../../src/ui/screens/trash_button_component.rs"]
pub mod trash_button_component;

#[path = "../../../src/ui/ui_errors_data.rs"]
pub mod ui_errors_data;
#[path = "../../../src/ui/screens/ui_errors_frame_component.rs"]
pub mod ui_errors_frame_component;

#[path = "../../../src/ui/screens/game_menu_main.rs"]
pub mod game_menu_main;
#[path = "../../../src/ui/panel_style_data.rs"]
mod panel_style_data;

#[path = "../../../src/ui/screens/loading_component.rs"]
pub mod loading_component;
#[path = "../../../src/ui/screens/login_component.rs"]
pub mod login;
#[path = "../../../src/ui/screens/screen_title.rs"]
mod screen_title;

use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

use char_create_component::{
    CHAR_CREATE_ROOT, CharCreateUiState, apply_character_create_styles, char_create_screen,
};
use char_select_component::{
    CharDisplayEntry, CharSelectState, DeleteConfirmUiState, apply_char_select_postsetup,
    char_select_screen,
};
use game_menu_main::{GAME_MENU_ROOT, main_menu_screen};
use loading_component::{LoadingScreenState, loading_screen};
use login::{
    PASSWORD_INPUT, SharedConnecting, SharedRealmSelectable, SharedRealmText, SharedStatusText,
    USERNAME_INPUT, login_screen,
};
use shared::protocol::CharacterListEntry;
use ui_errors_data::UiErrorsData;
use ui_errors_frame_component::ui_errors_frame_screen;

/// Authored UIErrorsFrame tree and message lifetime, independent of the active screen.
pub struct UiErrorsModel {
    pub screen: Screen,
    pub shared: SharedContext,
    pub registry: FrameRegistry,
    pub errors: UiErrorsData,
}

impl UiErrorsModel {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        let errors = UiErrorsData::default();
        let mut shared = SharedContext::new();
        shared.insert(errors.clone());
        Self {
            screen: Screen::new(ui_errors_frame_screen),
            shared,
            registry: FrameRegistry::new(screen_width, screen_height),
            errors,
        }
    }

    pub fn sync(&mut self) {
        self.shared.insert(self.errors.clone());
        self.screen.sync(&self.shared, &mut self.registry);
    }

    pub fn tick(&mut self, dt: f32) {
        self.errors.tick(dt);
        self.sync();
    }
}

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

/// Authored character-creation tree. The host owns catalog, actions and preview rendering.
pub struct CharacterCreateModel {
    pub screen: Screen,
    pub shared: SharedContext,
    pub registry: FrameRegistry,
}

impl CharacterCreateModel {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        let mut shared = SharedContext::new();
        shared.insert(CharCreateUiState {
            viewport_width: screen_width as u32,
            viewport_height: screen_height as u32,
            ..Default::default()
        });
        Self {
            screen: Screen::new(char_create_screen),
            shared,
            registry: FrameRegistry::new(screen_width, screen_height),
        }
    }

    pub fn sync(&mut self) {
        self.screen.sync(&self.shared, &mut self.registry);
        apply_character_create_postsetup(&self.shared, &mut self.registry);
    }
}

/// Apply authored styles and root dimensions after a screen sync or resize.
pub fn apply_character_create_postsetup(shared: &SharedContext, registry: &mut FrameRegistry) {
    let open = shared
        .get::<CharCreateUiState>()
        .and_then(|state| state.open_dropdown);
    apply_character_create_styles(registry, open);
    let (width, height) = (registry.screen_width, registry.screen_height);
    if let Some(id) = registry.get_by_name(CHAR_CREATE_ROOT.0)
        && let Some(root) = registry.get_mut(id)
    {
        root.width = ui_toolkit::frame::Dimension::Fixed(width);
        root.height = ui_toolkit::frame::Dimension::Fixed(height);
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
        apply_char_select_postsetup(&mut self.registry);
    }
}

/// Original main-menu tree only. The host owns opening, actions, and Options routing.
pub struct GameMenuModel {
    pub screen: Screen,
    pub shared: SharedContext,
    pub registry: FrameRegistry,
}

impl GameMenuModel {
    pub fn new(screen_width: f32, screen_height: f32, logged_in: bool) -> Self {
        let mut shared = SharedContext::new();
        shared.insert(logged_in);
        let mut registry = FrameRegistry::new(screen_width, screen_height);
        registry.register_panel_style("default", panel_style_data::default_panel_style());
        Self {
            screen: Screen::new(main_menu_screen),
            shared,
            registry,
        }
    }

    pub fn sync(&mut self) {
        self.screen.sync(&self.shared, &mut self.registry);
        let (width, height) = (self.registry.screen_width, self.registry.screen_height);
        if let Some(id) = self.registry.get_by_name(GAME_MENU_ROOT.0)
            && let Some(root) = self.registry.get_mut(id)
        {
            root.width = ui_toolkit::frame::Dimension::Fixed(width);
            root.height = ui_toolkit::frame::Dimension::Fixed(height);
        }
    }
}

/// Authored loading tree and reactive state; world readiness belongs to the host.
pub struct LoadingModel {
    pub screen: Screen,
    pub shared: SharedContext,
    pub registry: FrameRegistry,
}

impl LoadingModel {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        let mut shared = SharedContext::new();
        shared.insert(LoadingScreenState::default());
        let mut registry = FrameRegistry::new(screen_width, screen_height);
        registry.register_three_slice_style(
            "loading_bar_shell",
            loading_component::loading_bar_shell(),
        );
        Self {
            screen: Screen::new(loading_screen),
            shared,
            registry,
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
