use super::{RegistryUi, ScreenPostsetup, register_metal_frame_style};

impl RegistryUi {
    pub fn show_mount_journal(
        &mut self,
        state: game_engine_ui_collections::mount_journal::MountJournalView,
    ) -> Result<(), String> {
        let parent = self.hud_parent()?;
        let mut registry = parent.registry();
        register_metal_frame_style(
            &mut registry,
            game_engine_ui_model::panel_style_data::MetalTopLeft::Portrait,
        )?;
        self.show_viewport_screen_in(
            state,
            game_engine_ui_collections::mount_journal::mount_journal_screen,
            ScreenPostsetup::None,
            registry,
            parent,
        )?;
        self.toplevel = true;
        Ok(())
    }
}
