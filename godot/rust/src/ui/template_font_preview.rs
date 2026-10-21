//! Offline production font consumers not exposed by the ordinary panel previews.
use super::{RegistryUi, party_preview};
use game_engine_ui_model::{
    game_tooltip::{GameTooltip, GameTooltipView, TooltipScreen, place},
    group_loot_component::GroupLootUiState,
    tooltip_presentation::{TooltipLineState, TooltipPresentation},
};
use godot::prelude::*;
use shared::protocol::{GroupLootPlayer, GroupLootRoll, LootContent};
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_template_font_preview(&mut self, forever: bool, panel: GString) -> GString {
        let result = party_preview::load_data_root().and_then(|()| {
            set_thread_skin(if forever {
                ActiveSkin::Forever
            } else {
                ActiveSkin::Modern
            });
            self.set_ui_scale(1.0)?;
            match panel.to_string().as_str() {
                "GroupLoot" => self.show_group_loot(GroupLootUiState {
                    prompts: vec![font_preview_roll()],
                    history: vec![font_preview_roll()],
                    viewer: 11,
                    history_open: true,
                }),
                "Tooltip" => self.show_game_tooltip(font_preview_tooltip()),
                other => Err(format!("Unknown font preview panel {other}")),
            }
        });
        result.err().unwrap_or_default().as_str().into()
    }
}

fn font_preview_tooltip() -> GameTooltipView {
    let tooltip = GameTooltip::new(
        TooltipPresentation {
            title: "Moss Agate".into(),
            lines: vec![TooltipLineState::new("A green gemstone.")],
            ..Default::default()
        },
        None,
    );
    GameTooltipView {
        main: place(
            tooltip,
            TooltipScreen {
                size: [1920.0, 1080.0],
                cursor: [960.0, 540.0],
            },
        ),
        ..Default::default()
    }
}

fn font_preview_roll() -> GroupLootRoll {
    GroupLootRoll {
        roll_id: 1,
        corpse: 77,
        item: LootContent::Item {
            item_id: 1206,
            name: "Moss Agate".into(),
            quality: 2,
            count: 1,
        },
        seconds_remaining: 60.0,
        players: vec![GroupLootPlayer {
            character_id: 11,
            name: "Alice".into(),
            can_need: false,
            need_reason: Some("Your class may not roll need on this item.".into()),
            can_transmog: false,
            response: None,
        }],
        winner: None,
        complete: false,
        delivered: false,
    }
}
