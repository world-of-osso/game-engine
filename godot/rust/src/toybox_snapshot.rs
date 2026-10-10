use crate::GameClient;
use game_engine_ui_model::toybox::ToyAction;
use godot::prelude::*;

impl GameClient {
    pub(super) fn toybox_snapshot(&self) -> VarDictionary {
        let model = &self.toybox.model;
        let mut state = VarDictionary::new();
        state.set("catalog_count", model.catalog.len() as i64);
        state.set("page", model.page as i64);
        state.set("open", model.open);
        state.set("error", model.error.as_deref().unwrap_or(""));
        let mut toys = Array::<VarDictionary>::new();
        for toy in &model.catalog {
            if !toy.learned {
                continue;
            }
            let mut row = VarDictionary::new();
            row.set("item_id", i64::from(toy.item_id));
            row.set("learned", toy.learned);
            row.set("favourite", toy.favourite);
            row.set("name", toy.name.as_str());
            row.set(
                "cooldown",
                toy.spell_id
                    .map_or(0.0, |spell| model.cooldowns.remaining(spell)),
            );
            toys.push(&row);
        }
        state.set("learned", toys);
        let mut slots = VarDictionary::new();
        for slot in 0..crate::player_spells::ACTION_SLOT_COUNT {
            if let Some(action) = self
                .account
                .spells
                .slot(slot)
                .and_then(|slot| ToyAction::from_slot(slot, model))
            {
                slots.set(slot as i64, i64::from(action.item_id));
            }
        }
        state.set("slots", slots);
        state
    }
}
