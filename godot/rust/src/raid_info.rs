//! Native Retail saved-instance frame; both skins share the same rows and actions.
use crate::{GameClient, background_load::BackgroundLoad, frame_error::FrameError, ui::RegistryUi};
use game_engine_session::SessionScreen;
use game_engine_ui_model::raid_info::{RaidCatalog, RaidInfoState};
use godot::prelude::*;

pub(crate) struct RaidInfo {
    pub(crate) catalog: BackgroundLoad<Result<RaidCatalog, String>>,
    ui: Option<Gd<RegistryUi>>,
    state: RaidInfoState,
}
impl RaidInfo {
    pub(crate) fn new(data: &std::path::Path) -> Self {
        let db2 = data.join("db2/12.1.0.69933");
        Self {
            catalog: BackgroundLoad::start("raid-info", move || RaidCatalog::load(&db2)),
            ui: None,
            state: Default::default(),
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
    }
}
impl GameClient {
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
        if self.account.session.screen != SessionScreen::InWorld {
            self.raid_info.reset();
            return Ok(());
        }
        if !self.raid_info.state.visible {
            return Ok(());
        }
        if let Some(mut ui) = self.raid_info.ui.clone() {
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
        }
        if !self.raid_info.state.visible {
            self.raid_info.reset();
            return Ok(());
        }
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
