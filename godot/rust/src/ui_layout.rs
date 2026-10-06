//! Edit Mode layout presets: the selected character's active layout picks the UI skin.

use game_engine_core::{
    client_options_data::options_path,
    ui_layout_data::{self, ActiveLayout, LayoutSettings, LayoutSkin},
};
use game_engine_session::SessionScreen;
use game_engine_ui_model::hud_layout;
use game_engine_ui_model::options_menu_component::{LayoutOptionsView, LayoutSystem};
use ui_toolkit::atlas::{self, ActiveSkin};

use crate::GameClient;

fn layout_path() -> std::path::PathBuf {
    options_path().with_file_name("ui_layout.ron")
}

impl GameClient {
    /// In the world the selected character's saved layout applies; glue screens draw Modern.
    pub(super) fn apply_screen_ui_layout(&mut self, screen: SessionScreen) -> Result<(), String> {
        let layout = match screen {
            SessionScreen::InWorld => {
                let id = self
                    .account
                    .session
                    .selected_character_id
                    .ok_or("In-world UI layout requires a selected server character ID")?;
                ui_layout_data::active_layout(&layout_path(), id)?
            }
            SessionScreen::Login
            | SessionScreen::CharacterSelect
            | SessionScreen::CharacterCreate => ActiveLayout::default(),
            SessionScreen::Loading | SessionScreen::GameMenu => return Ok(()),
        };
        self.apply_ui_layout(layout)
    }

    /// Save `name` as the selected character's layout and draw it.
    pub(super) fn select_ui_layout(&mut self, name: &str) -> Result<(), String> {
        let id = self
            .account
            .session
            .selected_character_id
            .ok_or("Layout requires a selected server character ID")?;
        let layout = ui_layout_data::set_active_layout(&layout_path(), id, name)?;
        self.apply_ui_layout(layout)
    }

    /// Save `settings` to the selected character's layout (a new player layout while a
    /// preset is active) and draw it.
    pub(super) fn save_ui_layout_settings(
        &mut self,
        settings: LayoutSettings,
    ) -> Result<(), String> {
        let id = self
            .account
            .session
            .selected_character_id
            .ok_or("Layout settings require a selected server character ID")?;
        let layout = ui_layout_data::save_layout_settings(&layout_path(), id, settings)?;
        self.apply_ui_layout(layout)
    }

    /// The drawn layout and every selectable one, as the Options HUD page shows them.
    pub(super) fn ui_layout_options(
        &self,
        system: LayoutSystem,
    ) -> Result<LayoutOptionsView, String> {
        Ok(LayoutOptionsView {
            active: self.ui_layout.name.clone(),
            names: ui_layout_data::layout_names(&layout_path())?,
            skin: self.ui_layout.skin,
            settings: self.ui_layout.settings,
            system,
            party_dropdown: None,
        })
    }

    /// Resolve atlases under the layout's skin, publish its settings and resync every
    /// canvas, each of which mirrors both into its SharedContext so the Screens that read
    /// them rebuild.
    fn apply_ui_layout(&mut self, layout: ActiveLayout) -> Result<(), String> {
        let skin = match layout.skin {
            LayoutSkin::Modern => ActiveSkin::Modern,
            LayoutSkin::Forever => ActiveSkin::Forever,
        };
        let settings = layout.settings;
        self.ui_layout = layout;
        if atlas::thread_skin() == skin && hud_layout::active_layout_settings() == settings {
            return Ok(());
        }
        atlas::set_thread_skin(skin);
        hud_layout::set_active_layout_settings(settings);
        self.for_each_registry_ui(|ui| ui.bind_mut().sync_skin())
    }
}
