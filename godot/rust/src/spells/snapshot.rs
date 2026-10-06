//! Spell automation snapshot projection.

use crate::GameClient;
use game_engine_ui_model::main_action_bar_component::MAIN_BAR_BUTTONS;
use godot::prelude::*;
use shared::components::{Health, UnitLevel, UnitPowers};
use shared::protocol::{ActionRef, CombatLogKind};

impl GameClient {
    /// Spell state for automation.
    pub(crate) fn spells_snapshot(&self) -> VarDictionary {
        let mut state = VarDictionary::new();
        let spells = &self.account.spells;
        let ids = |values: &[u32]| {
            values
                .iter()
                .map(|&id| i64::from(id))
                .collect::<godot::builtin::PackedInt64Array>()
        };
        state.set("catalog_ready", self.spells.catalog().is_some());
        state.set("known", &ids(spells.known()));
        state.set("spec", i64::from(spells.spec().unwrap_or(0)));
        let bar: Vec<u32> = (0..MAIN_BAR_BUTTONS)
            .map(|index| match spells.slot(self.main_bar_slot(index)) {
                Some(ActionRef::Spell(id)) => id,
                _ => 0,
            })
            .collect();
        state.set("bar", &ids(&bar));
        let vigor = self
            .vigor_bar_state()
            .shown
            .map_or([0, 0], |frames| [frames.total, frames.full]);
        state.set("vigor", &ids(&vigor.map(u32::from)));
        let cooldowns: Vec<u32> = bar
            .iter()
            .map(|&id| {
                let timer = (id != 0)
                    .then(|| spells.button_cooldown(id, self.spell_triggers_gcd(id)))
                    .flatten();
                timer.map_or(0, |timer| (timer.remaining * 1000.0) as u32)
            })
            .collect();
        state.set("cooldown_ms", &ids(&cooldowns));
        state.set(
            "gcd_ms",
            spells
                .gcd()
                .map_or(0, |timer| (timer.remaining * 1000.0) as i64),
        );
        state.set("sent", &ids(&self.spells.sent));
        let errors: PackedStringArray = self
            .spells
            .errors
            .iter()
            .map(|error| GString::from(error.as_str()))
            .collect();
        state.set("errors", &errors);
        state.set(
            "casting",
            self.world
                .local_player_id()
                .and_then(|player| self.cast_bars.get(player))
                .filter(|bar| bar.casting || bar.channeling)
                .map_or(0, |bar| i64::from(bar.spell_id)),
        );
        let damage: Vec<u32> = self
            .account
            .combat_log
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Damage && event.source == self.world.local_player_id()
            })
            .map(|event| event.amount.max(0) as u32)
            .collect();
        state.set("damage_dealt", &ids(&damage));
        state.set("spellbook_open", self.spellbook_open());
        let tooltip: PackedStringArray = self
            .tooltip_text_lines()
            .iter()
            .map(|line| GString::from(line.as_str()))
            .collect();
        state.set("tooltip", &tooltip);
        state.set("combat_text", self.spells.floating.len() as i64);
        let player = self
            .world
            .local_player_id()
            .and_then(|id| self.replica.unit(id));
        let power = player
            .and_then(|unit| unit.get::<UnitPowers>()?.entries.first().cloned())
            .map_or(-1, |entry| i64::from(entry.current));
        state.set("power", power);
        state.set(
            "level",
            player
                .and_then(|unit| unit.get::<UnitLevel>())
                .map_or(0, |level| i64::from(level.0)),
        );
        let target_health = self
            .targeting_target()
            .and_then(|id| self.replica.unit(id)?.get::<Health>())
            .map_or(-1.0, |health| f64::from(health.current));
        state.set("target_health", target_health);
        let book: Vec<VarDictionary> = self
            .spells
            .book
            .categories
            .iter()
            .flat_map(|category| &category.groups)
            .flat_map(|group| &group.items)
            .map(|item| {
                let mut entry = VarDictionary::new();
                entry.set("id", i64::from(item.spell_id));
                entry.set("name", item.name.as_str());
                entry.set("available_at", i64::from(item.available_at.unwrap_or(0)));
                entry
            })
            .collect();
        let mut book_array = VarArray::new();
        for entry in book {
            book_array.push(&entry.to_variant());
        }
        state.set("spellbook", &book_array);
        state
    }
}
