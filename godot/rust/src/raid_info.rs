//! Native Retail saved-instance frame; both skins share rows and actions.
use crate::{GameClient, background_load::BackgroundLoad, frame_error::FrameError, ui::RegistryUi};
use game_engine_session::SessionScreen;
use game_engine_ui_model::raid_info::{RaidCatalog, RaidInfoState, snapshot_reached_expiry};
use godot::prelude::*;
use std::time::Instant;

pub(crate) struct RaidInfo {
    pub(crate) catalog: BackgroundLoad<Result<RaidCatalog, String>>,
    ui: Option<Gd<RegistryUi>>,
    state: RaidInfoState,
    refresh_requested_for: Option<Instant>,
}
impl RaidInfo {
    pub(crate) fn new(data: &std::path::Path) -> Self {
        let db2 = data.join("db2/12.1.0.69933");
        Self {
            catalog: BackgroundLoad::start("raid-info", move || RaidCatalog::load(&db2)),
            ui: None,
            state: Default::default(),
            refresh_requested_for: None,
        }
    }
    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Some(ui) = &mut self.ui {
            visit(ui)?;
        }
        Ok(())
    }
    pub(crate) fn reset(&mut self) {
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
        self.state = Default::default();
        self.refresh_requested_for = None;
    }
}
impl GameClient {
    pub(super) fn raid_info_pointer(&mut self, event: &Gd<godot::classes::InputEvent>) -> bool {
        if self.game_menu_ui.is_some() || !self.raid_info.state.visible {
            return false;
        }
        let Some(ui) = self.raid_info.ui.as_mut() else {
            return false;
        };
        let taken = ui.bind_mut().scroll_list_input(event);
        match taken {
            Ok(true) => {
                self.mark_viewport_input_handled();
                true
            }
            Ok(false) => false,
            Err(error) => {
                crate::frame_error::report_once(&format!("Raid Info scroll: {error}"));
                false
            }
        }
    }

    pub(super) fn close_raid_info(&mut self) -> bool {
        let visible = self.raid_info.state.visible;
        self.raid_info.reset();
        visible
    }
    pub(super) fn open_raid_info(&mut self) -> Result<(), FrameError> {
        self.account.send_request_raid_info()?;
        self.raid_info.state.visible = true;
        Ok(())
    }
    pub(super) fn update_raid_info(&mut self, _delta: f32) -> Result<(), FrameError> {
        self.raid_info.catalog.poll();
        if self.account.session.screen != SessionScreen::InWorld {
            self.raid_info.reset();
            return Ok(());
        }
        if !self.raid_info.state.visible {
            return Ok(());
        }
        self.receive_raid_info_actions()?;
        if !self.raid_info.state.visible {
            self.raid_info.reset();
            return Ok(());
        }
        self.refresh_expired_raid_snapshot()?;
        self.update_saved_instance_rows()?;
        self.sync_raid_info_frame()
    }
    fn receive_raid_info_actions(&mut self) -> Result<(), FrameError> {
        let Some(mut ui) = self.raid_info.ui.clone() else {
            return Ok(());
        };
        let error = ui.bind_mut().sync_input();
        if !error.is_empty() {
            return Err(error.to_string().into());
        }
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                break;
            }
            if let Some(request) = self.raid_info.state.action(&action) {
                self.account.send_saved_instance_extend(request)?;
            }
        }
        Ok(())
    }
    fn refresh_expired_raid_snapshot(&mut self) -> Result<(), FrameError> {
        let stamp = self.account.instance_info_at;
        let elapsed = stamp.elapsed().as_secs().min(u32::MAX as u64) as u32;
        let expired = snapshot_reached_expiry(&self.account.instance_locks, elapsed);
        if expired && self.raid_info.refresh_requested_for != Some(stamp) {
            self.account.send_request_raid_info()?;
            self.raid_info.refresh_requested_for = Some(stamp);
        }
        Ok(())
    }
    fn update_saved_instance_rows(&mut self) -> Result<(), FrameError> {
        let Some(catalog) = self.raid_info.catalog.loaded() else {
            return Ok(());
        };
        let catalog = catalog.as_ref().map_err(|e| FrameError::from(e.clone()))?;
        let elapsed = self
            .account
            .instance_info_at
            .elapsed()
            .as_secs()
            .min(u32::MAX as u64) as u32;
        self.raid_info.state.rows = catalog.rows(&self.account.instance_locks, elapsed)?;
        Ok(())
    }
    fn sync_raid_info_frame(&mut self) -> Result<(), FrameError> {
        let state = self.raid_info.state.clone();
        if let Some(ui) = &mut self.raid_info.ui {
            ui.bind_mut().set_state(state)?;
            return Ok(());
        }
        self.extract_art(&game_engine_ui_model::panel_style_data::metal_sheet_fdids(
            game_engine_ui_model::panel_style_data::MetalTopLeft::Plain,
        ));
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("RaidInfoUI");
        self.base_mut().add_child(&ui);
        let result = {
            let mut host = ui.bind_mut();
            host.set_ui_scale(self.effective_ui_scale())
                .and_then(|()| host.show_raid_info(state))
        };
        if let Err(error) = result {
            ui.free();
            return Err(error.into());
        }
        self.raid_info.ui = Some(ui.clone());
        self.raise_above_layer(&ui)?;
        Ok(())
    }
}
#[godot_api(secondary)]
impl GameClient {
    /// Retail's RaidInfoFrame show entry point, also available to native automation.
    #[func]
    fn show_raid_info(&mut self) -> GString {
        GString::from(
            self.open_raid_info()
                .err()
                .map(|e| e.to_string())
                .unwrap_or_default()
                .as_str(),
        )
    }
    #[func]
    fn raid_difficulty_state(&self) -> VarDictionary {
        let mut state = VarDictionary::new();
        if let Some(d) = self.account.raid_difficulty {
            state.set("primary", d.difficulty_id);
            state.set("legacy", d.legacy_difficulty_id);
        }
        state.set("saved_count", self.account.instance_locks.len() as i64);
        state
    }
}
