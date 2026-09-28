//! Main-menu overlay; the account screen and world remain attached underneath.

use game_engine_session::SessionScreen;
use game_engine_ui_model::game_menu_main::{
    ACTION_ADDONS, ACTION_EXIT, ACTION_LOGOUT, ACTION_OPTIONS, ACTION_RESUME, ACTION_SUPPORT,
};
use godot::prelude::*;

use crate::{GameClient, ui::RegistryUi};

impl GameClient {
    pub(super) fn open_game_menu(&mut self) -> Result<(), String> {
        if self.game_menu_ui.is_some() {
            return Ok(());
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("GameMenuUI");
        ui.set_layer(10);
        self.base_mut().add_child(&ui);
        let result = ui.bind_mut().show_game_menu(true);
        if let Err(error) = result {
            ui.free();
            return Err(error);
        }
        if let Some(mut viewport) = self.base().get_viewport() {
            viewport.gui_release_focus();
        }
        self.physical_input.clear();
        self.game_menu_ui = Some(ui);
        Ok(())
    }

    pub(super) fn close_game_menu(&mut self) {
        if let Some(ui) = self.game_menu_ui.take() {
            ui.free();
            self.physical_input.clear();
        }
    }

    pub(super) fn handle_game_menu_key(&mut self, key: godot::global::Key) -> Result<bool, String> {
        if self.game_menu_ui.is_some() {
            if key == godot::global::Key::ESCAPE {
                self.close_game_menu();
            }
            return Ok(true);
        }
        if key != godot::global::Key::ESCAPE
            || self.account.session.screen != SessionScreen::InWorld
            || !self.account.session.gameplay_input_allowed()
        {
            return Ok(false);
        }
        self.open_game_menu()?;
        Ok(true)
    }

    pub(super) fn poll_game_menu_actions(&mut self) -> Result<(), String> {
        let Some(ui) = self.game_menu_ui.as_mut() else {
            return Ok(());
        };
        let error = ui.bind_mut().sync_input();
        if !error.is_empty() {
            return Err(error.to_string());
        }
        let action = ui.bind_mut().pop_action().to_string();
        match action.as_str() {
            "" => {}
            ACTION_RESUME => self.close_game_menu(),
            ACTION_EXIT => self.base().get_tree().quit(),
            // Preserve the original Support placeholder; other destinations remain unported.
            ACTION_SUPPORT => godot_print!("menu_support: placeholder"),
            ACTION_OPTIONS | ACTION_ADDONS | ACTION_LOGOUT => {
                godot_warn!("Game menu action not yet converted: {action}");
            }
            _ => return Err(format!("Unknown game menu action: {action}")),
        }
        Ok(())
    }
}
