//! Offline twelve-character roster through the production projection and data-backed labels.
use game_engine_ui_model::{char_select_data::CharSelectNames, char_select_state_from_roster};
use godot::prelude::*;
use shared::protocol::CharacterListEntry;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};

use super::{RegistryUi, party_preview};

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_rosterfix_preview(&mut self) -> GString {
        self.show_rosterfix_skin(ActiveSkin::Modern)
    }

    #[func]
    fn show_forever_rosterfix_preview(&mut self) -> GString {
        self.show_rosterfix_skin(ActiveSkin::Forever)
    }

    fn show_rosterfix_skin(&mut self, skin: ActiveSkin) -> GString {
        let result = party_preview::load_data_root().and_then(|()| {
            set_thread_skin(skin);
            self.set_ui_scale(1.0)?;
            let names = CharSelectNames::load(&game_engine_ui_model::paths::resolve_data_path(""))?;
            let state = char_select_state_from_roster(&preview_roster(), Some(9), &names)?;
            let error = self.show_character_select();
            if !error.is_empty() {
                return Err(error.to_string());
            }
            self.set_state(state)
        });
        GString::from(result.err().unwrap_or_default().as_str())
    }
}

fn preview_roster() -> Vec<CharacterListEntry> {
    [
        ("Skyhighwar", 95, 1),
        ("Skyhighhunt", 95, 3),
        ("Skyhighrog", 95, 4),
        ("Skyhighmage", 95, 8),
        ("Skyhighdru", 95, 11),
        ("Skywindwar", 96, 1),
        ("Skywindhunt", 96, 3),
        ("Skywindrog", 96, 4),
        ("Skywindsham", 96, 7),
        ("Skywinddru", 96, 11),
        ("Skyhighfem", 95, 1),
        ("Skywindfem", 96, 1),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (name, race, class))| CharacterListEntry {
        character_id: index as u64 + 1,
        name: name.into(),
        level: 1,
        race,
        class,
        appearance: Default::default(),
        equipment_appearance: Default::default(),
    })
    .collect()
}
