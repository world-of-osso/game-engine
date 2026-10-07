//! Offline pending-registration feedback through the production login component.
use godot::prelude::*;
use shared::protocol::RegisterResponse;

use super::{RegistryUi, party_preview};

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_registration_pending_preview(&mut self) -> GString {
        GString::from(
            self.show_pending_registration()
                .err()
                .unwrap_or_default()
                .as_str(),
        )
    }

    fn show_pending_registration(&mut self) -> Result<(), String> {
        party_preview::load_data_root()?;
        self.initialize_login(1920.0, 1080.0)?;
        self.advance_login_fade(super::LOGIN_FADE_SECS)?;
        self.toggle_registration("Offline preview")?;
        let mut session = game_engine_session::Session::default();
        session.receive_registration(RegisterResponse {
            success: false,
            token: String::new(),
            pending_approval: true,
            error: None,
        });
        self.set_login_feedback(
            session.feedback.as_deref().unwrap_or_default(),
            session.feedback_is_informational,
        )
    }
}
