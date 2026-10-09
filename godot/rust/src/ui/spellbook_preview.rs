//! Offline PlayerSpellsFrame capture through the production spellbook projection.
use super::{RegistryUi, party_preview};
use game_engine_ui_model::spellbook_frame_component::PlayerSpellsTab;
use game_engine_ui_model::spellbook_preview::load_preview_state;
use godot::prelude::*;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_spellbook_preview(&mut self) -> GString {
        self.show_spellbook_preview_skin(ActiveSkin::Modern)
    }

    #[func]
    fn show_forever_spellbook_preview(&mut self) -> GString {
        self.show_spellbook_preview_skin(ActiveSkin::Forever)
    }

    fn show_spellbook_preview_skin(&mut self, skin: ActiveSkin) -> GString {
        let result = party_preview::load_data_root().and_then(|()| {
            set_thread_skin(skin);
            self.set_ui_scale(1.0)?;
            let page = std::env::var("GODOT_SPELLBOOK_TAB").unwrap_or_default();
            let tab = match page.as_str() {
                "" | "spellbook" => PlayerSpellsTab::Spellbook,
                "specialization" => PlayerSpellsTab::Specialization,
                "talents" => PlayerSpellsTab::Talents,
                _ => return Err(format!("Unknown offline PlayerSpells page: {page}")),
            };
            let data = game_engine_ui_model::paths::resolve_data_path("");
            let mut state = load_preview_state(&data, tab)?;
            let size = self
                .base()
                .get_viewport()
                .ok_or("Spellbook preview has no viewport")?
                .get_visible_rect()
                .size;
            state.viewport = [size.x, size.y];
            self.show_spellbook(state)
        });
        GString::from(result.err().unwrap_or_default().as_str())
    }
}
