//! Offline guild ranks through the production settings screen.
use godot::prelude::*;

use super::{RegistryUi, party_preview};

#[godot_api(secondary)]
impl RegistryUi {
    /// Offline authoritative guild snapshot through the production settings screen.
    #[func]
    pub fn show_guild_ranks_preview(&mut self) -> GString {
        self.show_guild_preview_skin(ui_toolkit::atlas::ActiveSkin::Modern)
    }

    #[func]
    pub fn show_forever_guild_ranks_preview(&mut self) -> GString {
        self.show_guild_preview_skin(ui_toolkit::atlas::ActiveSkin::Forever)
    }

    fn show_guild_preview_skin(&mut self, skin: ui_toolkit::atlas::ActiveSkin) -> GString {
        if let Err(error) = party_preview::load_data_root() {
            return GString::from(error.as_str());
        }
        ui_toolkit::atlas::set_thread_skin(skin);
        GString::from(
            self.set_ui_scale(1.0)
                .and_then(|()| {
                    self.show_guild_ranks(game_engine_ui_model::guild_rank_frame::preview())
                })
                .err()
                .unwrap_or_default()
                .as_str(),
        )
    }
}
