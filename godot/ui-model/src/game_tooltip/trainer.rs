//! Trainer services reuse the shared spell and item tooltip content.
use super::{
    GameTooltip,
    spell::{SpellTooltipInput, spell_tooltip},
};
use game_engine_core::spell_catalog::CatalogSpell;

pub fn trainer_spell_tooltip(
    spell: &CatalogSpell,
    input: &SpellTooltipInput,
    _recipe: Option<&crate::professions::Recipe>,
    _player_level: Option<u16>,
) -> Result<GameTooltip, String> {
    Ok(spell_tooltip(spell, input))
}

impl crate::trainer_frame::TrainerView {
    pub fn service_tooltip(
        &self,
        spell: u32,
        tooltip: GameTooltip,
        owner: [f32; 4],
    ) -> Option<GameTooltip> {
        self.book.service_tooltip(spell, tooltip, owner)
    }
}
