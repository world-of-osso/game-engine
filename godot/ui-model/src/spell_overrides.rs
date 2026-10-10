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

/// Retail OverrideActionBar.lua exposes six ordered buttons. A293 overrides the
/// main bar including empty stored slots, not spell-ID pairs like aura332.
pub fn override_spell_set(auras: &[AuraView]) -> Option<&[u32]> {
    auras
        .iter()
        .rev()
        .flat_map(|aura| &aura.overrides)
        .find_map(|entry| match entry {
            AuraOverride::SpellSet { spells, .. } => Some(spells.as_slice()),
            _ => None,
        })
}

pub fn resolve_action_slot(
    slot: usize,
    stored: Option<ActionRef>,
    auras: &[AuraView],
) -> Option<ActionRef> {
    if slot < 12 {
        if let Some(spells) = override_spell_set(auras) {
            return (slot < 6)
                .then(|| spells.get(slot).copied())
                .flatten()
                .filter(|&id| id != 0)
                .map(ActionRef::Spell);
        }
    }
    stored.map(|action| resolve_action(action, auras))
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
