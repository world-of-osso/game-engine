//! Edit Mode layout presets: the selected character's active layout picks the UI skin.

use game_engine_core::{
    client_options_data::options_path,
    ui_layout_data::{self, ActiveLayout, LayoutSkin},
};
use game_engine_session::SessionScreen;
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
        let skin = ui_layout_data::set_active_layout(&layout_path(), id, name)?;
        self.apply_ui_layout(ActiveLayout {
            name: name.to_string(),
            skin,
        })
    }

    /// Resolve atlases under the layout's skin and resync every canvas, each of which
    /// mirrors the skin into its SharedContext so the Screens that read it rebuild.
    fn apply_ui_layout(&mut self, layout: ActiveLayout) -> Result<(), String> {
        let skin = match layout.skin {
            LayoutSkin::Modern => ActiveSkin::Modern,
            LayoutSkin::Forever => ActiveSkin::Forever,
        };
        self.ui_layout = layout;
        if atlas::active_skin() == skin {
            return Ok(());
        }
        atlas::set_active_skin(skin);
        self.for_each_registry_ui(|ui| ui.bind_mut().sync_skin())
    }
}
