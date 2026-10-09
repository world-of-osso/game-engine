//! Offline assignment/key-label proof through the production action bar screen.
use super::{RegistryUi, ScreenPostsetup, party_preview};
use crate::player_spells::PlayerSpells;
use game_engine_core::input_bindings_data::{BindingKey, InputBinding, InputBindingsData};
use game_engine_ui_model::main_action_bar_component::{
    ActionBar, MainActionBarState, main_action_bar_screen,
};
use game_engine_ui_model::spellbook_frame_component::PlayerSpellsTab;
use godot::prelude::*;
use shared::protocol::ActionRef;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_sidebarbinds_preview(&mut self) -> GString {
        self.show_sidebarbinds_skin(ActiveSkin::Modern)
    }

    #[func]
    fn show_forever_sidebarbinds_preview(&mut self) -> GString {
        self.show_sidebarbinds_skin(ActiveSkin::Forever)
    }

    fn show_sidebarbinds_skin(&mut self, skin: ActiveSkin) -> GString {
        let result = (|| {
            party_preview::load_data_root()?;
            set_thread_skin(skin);
            self.set_ui_scale(1.0)?;
            let state = load_sidebarbinds_state()?;
            self.show_viewport_screen(state, main_action_bar_screen, ScreenPostsetup::None)
        })();
        GString::from(result.err().unwrap_or_default().as_str())
    }
}

fn load_sidebarbinds_state() -> Result<MainActionBarState, String> {
    let data = game_engine_ui_model::paths::resolve_data_path("");
    let book = game_engine_ui_model::spellbook_preview::load_preview_state(
        &data,
        PlayerSpellsTab::Spellbook,
    )?;
    let icons: Vec<_> = book
        .categories
        .iter()
        .flat_map(|category| &category.groups)
        .flat_map(|group| &group.items)
        .filter(|item| !item.passive && item.available_at.is_none())
        .collect();
    if icons.is_empty() {
        return Err("Sidebar preview requires known active spells".into());
    }
    let mut state = MainActionBarState {
        player_class: Some(8),
        ..Default::default()
    };
    state.extra_action_bars.action_bar_2 = Some(true);
    state.extra_action_bars.action_bar_3 = Some(true);
    state.extra_action_bars.action_bar_4 = true;
    state.extra_action_bars.action_bar_5 = true;
    let mut spells = PlayerSpells::default();
    for bar in ActionBar::ALL {
        for index in 0..12 {
            let spell = icons[index % icons.len()];
            let request = bar
                .assignment(index, ActionRef::Spell(spell.spell_id), 0)
                .ok_or("Preview assignment outside action bar")?;
            spells.apply_assignment(&request);
            if spells.slot(bar.action_slot(index)) != Some(ActionRef::Spell(spell.spell_id)) {
                return Err("Preview assignment did not update slot".into());
            }
            state.bar_mut(bar)[index].icon_fdid = spell.icon_fdid;
        }
    }
    state.set_hotkeys(&preview_bindings());
    Ok(state)
}

/// Explicit preview overrides only; Retail defaults remain unbound on all extra bars.
fn preview_bindings() -> InputBindingsData {
    let mut bindings = InputBindingsData::default();
    for (bar, key) in [
        (ActionBar::BottomLeft, BindingKey::KeyQ),
        (ActionBar::BottomRight, BindingKey::KeyE),
        (ActionBar::Right, BindingKey::KeyR),
        (ActionBar::Left, BindingKey::KeyT),
    ] {
        bindings.assign(bar.binding_actions()[0], InputBinding::Keyboard(key));
        bindings.assign(bar.binding_actions()[1], InputBinding::ShiftKeyboard(key));
    }
    bindings
}
