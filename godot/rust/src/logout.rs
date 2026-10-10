//! Logout countdown and authenticated character-list return, releasing the character's world.
use std::time::Duration;

use game_engine_core::input_bindings_data::InputAction;
use game_engine_session::{SessionScreen, logout::LogoutRequestOutcome};
use godot::{
    classes::{CanvasLayer, Label},
    prelude::*,
};

use crate::GameClient;
use crate::replicated::UnitFields;

impl GameClient {
    pub(super) fn request_logout(&mut self) -> Result<(), String> {
        let in_combat = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id))
            .is_some_and(UnitFields::in_combat);
        match self.logout.request(in_combat, self.in_rest_area) {
            LogoutRequestOutcome::BlockedInCombat => {
                godot_warn!("Cannot logout while in combat");
            }
            LogoutRequestOutcome::Immediate => self.finish_logout()?,
            LogoutRequestOutcome::StartedCountdown | LogoutRequestOutcome::AlreadyPending => {
                self.close_game_menu()
            }
        }
        self.sync_logout_overlay()
    }

    pub(super) fn update_logout(&mut self, delta: f64) -> Result<(), String> {
        if self.account.session.screen != SessionScreen::InWorld {
            return Ok(());
        }
        if self.logout.tick(Duration::from_secs_f64(delta)) {
            return self.finish_logout();
        }
        if self.logout_cancelled_by_input()? {
            self.logout.cancel();
        }
        self.sync_logout_overlay()
    }

    fn logout_cancelled_by_input(&self) -> Result<bool, String> {
        let viewport = self
            .base()
            .get_viewport()
            .ok_or("Logout root has no viewport")?;
        let keyboard = !viewport.gui_is_dragging()
            && !viewport
                .gui_get_focus_owner()
                .is_some_and(|focus| focus.is_class("LineEdit") || focus.is_class("TextEdit"));
        let input = self.physical_input.gameplay_state(keyboard);
        Ok([
            InputAction::MoveForward,
            InputAction::MoveBackward,
            InputAction::StrafeLeft,
            InputAction::StrafeRight,
            InputAction::Jump,
            InputAction::AutoRun,
            InputAction::TurnLeft,
            InputAction::TurnRight,
        ]
        .into_iter()
        .any(|action| self.client_options.bindings.is_pressed(action, &input)))
    }

    fn finish_logout(&mut self) -> Result<(), String> {
        self.account
            .logout_to_character_select()
            .map_err(|error| error.to_string())?;
        self.reset_world()?;
        self.show_account_screen(self.account.session.screen)
    }

    pub(super) fn sync_logout_overlay(&mut self) -> Result<(), String> {
        let text = self.logout.remaining_text();
        let mut overlay = self
            .base()
            .try_get_node_as::<CanvasLayer>("LogoutOverlay")
            .ok_or("Client scene is missing LogoutOverlay")?;
        let mut label = overlay
            .try_get_node_as::<Label>("Panel/Countdown")
            .ok_or("Logout overlay is missing Countdown")?;
        label.set_text(&text);
        let visible = self.account.session.screen == SessionScreen::InWorld && !text.is_empty();
        overlay.set_visible(visible);
        Ok(())
    }
}
