//! Native PvP queue, entry dialog, objective HUD and match score/results.
use crate::{GameClient, frame_error::FrameError, ui::RegistryUi};
use game_engine_session::SessionScreen;
use game_engine_ui_model::battleground::{PvpView, pvp_screen};
use godot::prelude::*;
#[derive(Default)]
pub(crate) struct BattlegroundUi {
    pub(crate) view: PvpView,
    pub(crate) ui: Option<Gd<RegistryUi>>,
}
impl BattlegroundUi {
    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Some(ui) = &mut self.ui {
            visit(ui)?;
        }
        Ok(())
    }
}
impl GameClient {
    /// PVPUIFrame and PVPMatch ScrollBoxes use the host-routed wheel/thumb/stepper input.
    pub(super) fn battleground_pointer(&mut self, event: &Gd<godot::classes::InputEvent>) -> bool {
        if self.game_menu_ui.is_some() {
            return false;
        }
        let Some(ui) = self.battleground.ui.as_mut() else {
            return false;
        };
        let taken = ui.bind_mut().scroll_list_input(event);
        match taken {
            Ok(false) => false,
            Ok(true) => {
                self.mark_viewport_input_handled();
                true
            }
            Err(error) => {
                crate::frame_error::report_once(&format!("PvP scroll input: {error}"));
                false
            }
        }
    }
    pub(super) fn toggle_pvp(&mut self) -> Result<(), FrameError> {
        self.battleground.view.visible = !self.battleground.view.visible;
        if self.battleground.view.visible {
            self.account
                .send_pvp(game_engine_ui_model::battleground::PvpAction::Query)?;
        }
        Ok(())
    }
    pub(super) fn update_battleground(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            return Ok(());
        }
        if let Some(mut ui) = self.battleground.ui.clone() {
            loop {
                let action = ui.bind_mut().pop_action().to_string();
                if action.is_empty() {
                    break;
                }
                match self.battleground.view.action(&action) {
                    Some(game_engine_ui_model::battleground::PvpAction::Target(entity)) => {
                        self.set_target(Some(entity));
                    }
                    Some(request) => self.account.send_pvp(request)?,
                    None => {}
                }
            }
        }
        if !self.battleground.view.visible && self.battleground.view.battleground().is_none() {
            if let Some(ui) = self.battleground.ui.take() {
                ui.free();
            }
            return Ok(());
        }
        self.sync_battleground()?;
        Ok(())
    }
    fn sync_battleground(&mut self) -> Result<(), String> {
        let scale = self.effective_ui_scale();
        let size = self
            .base()
            .get_viewport()
            .ok_or("PvP has no viewport")?
            .get_visible_rect()
            .size;
        self.battleground.view.viewport = [size.x / scale, size.y / scale];
        self.battleground.view.honor_faction = self
            .account
            .session
            .characters
            .iter()
            .find(|entry| Some(entry.character_id) == self.account.session.selected_character_id)
            .and_then(|entry| game_engine_ui_model::char_create_data::race_by_id(entry.race))
            .map(|race| race.faction);
        let view = self.battleground.view.clone();
        self.extract_art(&crate::quests::screen_texture_fdids(
            view.clone(),
            pvp_screen,
        ));
        if let Some(ui) = &mut self.battleground.ui {
            ui.bind_mut().set_ui_scale(scale)?;
            return ui.bind_mut().set_state(view);
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("BattlegroundUI");
        ui.set_layer(7);
        self.base_mut().add_child(&ui);
        let result = {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale).and_then(|()| host.show_pvp(view))
        };
        if let Err(error) = result {
            ui.free();
            return Err(error);
        }
        self.battleground.ui = Some(ui);
        Ok(())
    }
}
