//! Aura-driven spell presentation; the stored action stays bound to its base spell.

use shared::{
    components::{AuraOverride, AuraView},
    protocol::ActionRef,
};

pub fn update_spellbook_item(
    item: &mut crate::spellbook_frame_component::SpellbookItemView,
    replacement: Option<&game_engine_core::spell_catalog::CatalogSpell>,
    cooldown_fraction: f32,
) {
    if item.available_at.is_some() {
        return;
    }
    if let Some(spell) = replacement {
        item.name = spell.name.to_string();
        item.subtext = if spell.passive {
            "Passive".into()
        } else {
            spell.subtext.to_string()
        };
        item.icon_fdid = spell.icon_fdid;
        item.passive = spell.passive;
    }
    item.cooldown_fraction = cooldown_fraction;
}

/// Server-resolved aura332 pairs. Recompute from current auras so removal restores
/// the base action without changing action slots or known spells.
pub fn resolve_action(action: ActionRef, auras: &[AuraView]) -> ActionRef {
    let ActionRef::Spell(base) = action else {
        return action;
    };
    auras
        .iter()
        .flat_map(|aura| &aura.overrides)
        .find_map(|entry| match entry {
            AuraOverride::ActionBar {
                spell_id,
                replacement,
            } if *spell_id == base => Some(ActionRef::Spell(*replacement)),
            _ => None,
        })
        .unwrap_or(action)
}
