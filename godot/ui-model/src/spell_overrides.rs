//! Aura-driven spell presentation; the stored action stays bound to its base spell.

use shared::{
    components::{AuraOverride, AuraView},
    protocol::ActionRef,
};

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
