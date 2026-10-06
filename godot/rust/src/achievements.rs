//! Native AchievementFrame and achievement alerts over the account's cached pages.
use crate::{GameClient, frame_error::FrameError, ui::RegistryUi};
use game_engine_core::input_bindings_data::InputAction;
use game_engine_session::SessionScreen;
use game_engine_ui_model::achievements::{self as model};
use godot::prelude::*;
use shared::protocol::{AchievementStateUpdate, AchievementToastSnapshot, QueryAchievementCatalog};
use std::collections::VecDeque;

#[derive(Default)]
pub(crate) struct Achievements {
    ui: Option<Gd<RegistryUi>>,
    toast_ui: Option<Gd<RegistryUi>>,
    toasts: VecDeque<AchievementToastSnapshot>,
    toast_remaining: f32,
}
impl Achievements {
    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        for ui in [&mut self.ui, &mut self.toast_ui].into_iter().flatten() {
            visit(ui)?;
        }
        Ok(())
    }
    pub(crate) fn reset(&mut self) {
        free(&mut self.ui);
        free(&mut self.toast_ui);
        self.toasts.clear();
        self.toast_remaining = 0.0;
    }
}
fn free(ui: &mut Option<Gd<RegistryUi>>) {
    if let Some(ui) = ui.take() {
        ui.free();
    }
}
impl GameClient {
    pub(super) fn close_achievements(&mut self) -> bool {
        let visible = self.account.achievements.visible;
        self.account.achievements.visible = false;
        free(&mut self.achievements.ui);
        visible
    }
    pub(super) fn toggle_achievements(&mut self) -> Result<(), FrameError> {
        let requests = self.account.achievements.action(model::OPEN_ACTION);
        self.send_achievement_requests(requests)?;
        Ok(())
    }
    fn send_achievement_requests(
        &mut self,
        requests: Vec<QueryAchievementCatalog>,
    ) -> Result<(), FrameError> {
        for request in requests {
            if let Err(error) = self.account.send_achievement_request(request) {
                self.account.achievements.request_failed(request);
                return Err(error.into());
            }
        }
        Ok(())
    }
    pub(super) fn receive_achievement_update(
        &mut self,
        update: AchievementStateUpdate,
    ) -> Result<(), String> {
        if let Some(error) = update.error {
            self.add_world_error(&error)?;
        }
        if let Some(toast) = update.completed {
            self.achievements.toasts.push_back(toast);
        }
        Ok(())
    }
    pub(super) fn update_achievements(&mut self, delta: f32) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            free(&mut self.achievements.ui);
            return Ok(());
        }
        if self.account.session.gameplay_input_allowed() && self.keyboard_free() {
            let input = self.physical_input.gameplay_state(true);
            if self
                .client_options
                .bindings
                .is_just_pressed(InputAction::ToggleAchievements, &input)
            {
                self.toggle_achievements()?;
            }
        }
        let requests = std::mem::take(&mut self.account.achievement_requests);
        self.send_achievement_requests(requests)?;
        if let Some(mut ui) = self.achievements.ui.clone() {
            loop {
                let action = ui.bind_mut().pop_action().to_string();
                if action.is_empty() {
                    break;
                }
                let requests = self.account.achievements.action(&action);
                self.send_achievement_requests(requests)?;
            }
        }
        self.sync_achievements()?;
        self.advance_achievement_toast(delta)?;
        Ok(())
    }
    fn sync_achievements(&mut self) -> Result<(), String> {
        if !self.account.achievements.visible {
            free(&mut self.achievements.ui);
            return Ok(());
        }
        let state = self.account.achievements.clone();
        for entry in state.entries() {
            self.drawable_fdid(entry.icon_fdid);
        }
        if let Some(ui) = &mut self.achievements.ui {
            return ui.bind_mut().set_state(state);
        }
        self.extract_art(model::ART);
        self.extract_art(&game_engine_ui_model::panel_style_data::metal_sheet_fdids(
            game_engine_ui_model::panel_style_data::MetalTopLeft::Plain,
        ));
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("AchievementsUI");
        self.base_mut().add_child(&ui);
        let shown = ui
            .bind_mut()
            .set_ui_scale(self.effective_ui_scale())
            .and_then(|()| ui.bind_mut().show_achievement_window(state));
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.achievements.ui = Some(ui.clone());
        self.raise_above_layer(&ui)
    }
    fn advance_achievement_toast(&mut self, delta: f32) -> Result<(), String> {
        self.achievements.toast_remaining -= delta;
        if self.achievements.toast_remaining > 0.0 {
            return Ok(());
        }
        free(&mut self.achievements.toast_ui);
        let Some(toast) = self.achievements.toasts.pop_front() else {
            return Ok(());
        };
        self.extract_art(&[130650]);
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("AchievementToastUI");
        ui.set_layer(10);
        self.base_mut().add_child(&ui);
        let shown = ui
            .bind_mut()
            .set_ui_scale(self.effective_ui_scale())
            .and_then(|()| ui.bind_mut().show_achievement_toast(toast));
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.achievements.toast_ui = Some(ui);
        self.achievements.toast_remaining = 5.0;
        Ok(())
    }
}
