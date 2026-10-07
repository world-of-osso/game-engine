//! Offline Arcane Intellect through the production aura tooltip and placement.
use game_engine_ui_model::{
    aura_display_data::AuraInstance,
    game_tooltip::{GameTooltipView, TooltipScreen, place, spell::aura_tooltip},
};
use godot::prelude::*;

use super::{RegistryUi, party_preview};

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_aura_tooltip_preview(&mut self) -> GString {
        let result = party_preview::load_data_root().and_then(|()| {
            ui_toolkit::atlas::set_thread_skin(ui_toolkit::atlas::ActiveSkin::Modern);
            let tooltip = aura_tooltip(&preview_aura());
            self.show_game_tooltip(GameTooltipView {
                main: place(
                    tooltip,
                    TooltipScreen {
                        size: [1920.0, 1080.0],
                        cursor: [0.0, 0.0],
                    },
                ),
                ..Default::default()
            })
        });
        GString::from(result.err().unwrap_or_default().as_str())
    }
}

fn preview_aura() -> AuraInstance {
    AuraInstance {
        instance_id: 1,
        spell_id: 1459,
        name: "Arcane Intellect".into(),
        description: "Intellect increased by 3%.".into(),
        icon_fdid: 135_932,
        source: String::new(),
        from_local_player: true,
        from_player: true,
        duration: 3600.0,
        remaining: 3542.0,
        stacks: 1,
        is_debuff: false,
        debuff_type: Default::default(),
    }
}
