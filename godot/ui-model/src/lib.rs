//! Registry-authoritative login UI model. Rendering and authentication belong to the host.

pub mod ui {
    pub use ui_toolkit::{anchor, strata};

    pub mod widgets {
        pub use ui_toolkit::widgets::font_string;
    }
}

#[path = "../../../src/ui/screens/login_component.rs"]
pub mod login;

use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

use login::{
    PASSWORD_INPUT, SharedConnecting, SharedRealmSelectable, SharedRealmText, SharedStatusText,
    USERNAME_INPUT, login_screen,
};

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
