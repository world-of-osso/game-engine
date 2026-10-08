//! Trainer services reuse the shared spell and item tooltip content.
use super::{
    GameTooltip,
    spell::{SpellTooltipInput, spell_tooltip},
};
use game_engine_core::spell_catalog::CatalogSpell;

pub fn trainer_spell_tooltip(
    spell: &CatalogSpell,
    input: &SpellTooltipInput,
    _player_level: Option<u16>,
) -> Result<GameTooltip, String> {
    Ok(spell_tooltip(spell, input))
}
