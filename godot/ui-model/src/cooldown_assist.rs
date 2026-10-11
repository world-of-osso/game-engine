//! Cooldown Viewer presentation from authored entries and authoritative spell samples.
//! The optional viewer highlight extends Retail's action-button Assisted Highlight.

use game_engine_core::cooldown_assist::{CooldownEntry, Recommendation};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CooldownViewerModel {
    /// Cooldown IDs, not spell IDs: linked and overridden spells retain the same slot.
    selection: BTreeMap<u32, BTreeMap<u32, bool>>,
    order: BTreeMap<u32, Vec<u32>>,
    pub highlight_action_bar: bool,
    /// World of Osso extension, off by default; not a Retail behavior.
    pub highlight_viewer: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpellSample {
    pub spell_id: u32,
    pub name: String,
    pub icon_fdid: u32,
    pub duration: f32,
    pub remaining: f32,
    pub charges: Option<u8>,
    pub aura_active: bool,
    pub aura_duration: f32,
    pub aura_remaining: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CooldownIconView {
    pub cooldown_id: u32,
    pub spell_id: u32,
    pub name: String,
    pub icon_fdid: u32,
    pub fraction: f32,
    pub remaining: f32,
    pub charges: Option<u8>,
    pub assisted_highlight: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CooldownViewerView {
    /// Essential, Utility, Tracked Buff icons, Tracked Buff bars.
    pub categories: [Vec<CooldownIconView>; 4],
    pub recommended_spell: Option<u32>,
}

impl CooldownViewerModel {
    pub fn selected(&self, spec: u32, entry: &CooldownEntry) -> bool {
        self.selection
            .get(&spec)
            .and_then(|choices| choices.get(&entry.id))
            .copied()
            .unwrap_or_else(|| !entry.hidden_by_default())
    }

    pub fn select(&mut self, spec: u32, cooldown_id: u32, selected: bool) {
        self.selection
            .entry(spec)
            .or_default()
            .insert(cooldown_id, selected);
    }

    /// Settings reorder IDs within their category; projection never mixes categories.
    pub fn move_before(&mut self, spec: u32, cooldown_id: u32, before: u32) {
        if cooldown_id == before {
            return;
        }
        let order = self.order.entry(spec).or_default();
        order.retain(|id| *id != cooldown_id);
        let index = order
            .iter()
            .position(|id| *id == before)
            .unwrap_or_else(|| {
                order.push(before);
                order.len() - 1
            });
        order.insert(index, cooldown_id);
    }

    pub fn project(
        &self,
        spec: u32,
        entries: &[&CooldownEntry],
        recommendation: Recommendation,
        mut sample: impl FnMut(u32) -> Option<SpellSample>,
    ) -> CooldownViewerView {
        let recommended_spell = match recommendation {
            Recommendation::Spell(id) => Some(id),
            _ => None,
        };
        let mut view = CooldownViewerView {
            recommended_spell,
            ..Default::default()
        };
        for entry in entries {
            if entry.category > 3 || !self.selected(spec, entry) {
                continue;
            }
            // Runtime adapter resolves overrides before sampling. Linked spells may be
            // unlearned while the underlying authored slot remains stable.
            let Some(state) = sample(entry.spell_id) else {
                continue;
            };
            let tracked = entry.category >= 2;
            let linked_aura = entry
                .linked_spells
                .iter()
                .filter_map(|link| sample(link.spell_id))
                .find(|sample| sample.aura_active);
            let aura = if state.aura_active {
                Some(&state)
            } else {
                linked_aura.as_ref()
            };
            let displays_aura = !entry.hides_aura() && aura.is_some();
            if tracked && !displays_aura {
                continue;
            }
            let (duration, remaining) = if displays_aura {
                let aura = aura.expect("active aura selected above");
                (aura.aura_duration, aura.aura_remaining)
            } else {
                (state.duration, state.remaining)
            };
            let fraction = if duration > 0.0 {
                (remaining / duration).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let assisted_highlight =
                self.highlight_viewer && recommended_spell == Some(state.spell_id);
            view.categories[entry.category as usize].push(CooldownIconView {
                cooldown_id: entry.id,
                spell_id: state.spell_id,
                name: state.name,
                icon_fdid: state.icon_fdid,
                fraction,
                remaining,
                charges: state.charges,
                assisted_highlight,
            });
        }
        if let Some(order) = self.order.get(&spec) {
            for category in &mut view.categories {
                category.sort_by_key(|entry| {
                    order
                        .iter()
                        .position(|id| *id == entry.cooldown_id)
                        .unwrap_or(order.len())
                });
            }
        }
        view
    }

    /// Matching action buttons use the exact recommendation used by the assistant;
    /// disabling bar highlighting does not disable the assistant itself.
    pub fn highlights_action(&self, spell_id: u32, recommendation: Recommendation) -> bool {
        self.highlight_action_bar && recommendation == Recommendation::Spell(spell_id)
    }
}
