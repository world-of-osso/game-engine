//! Dynamic spell list data model.
//!
//! Replaces the hardcoded Paladin spell book with a server-populated runtime
//! resource. Spells are grouped by tab (General, class, spec) and can be
//! updated as the player learns new abilities or changes spec.

use bevy::prelude::*;

/// A single known spell.
#[derive(Clone, Debug, PartialEq)]
pub struct SpellEntry {
    pub spell_id: u32,
    pub name: String,
    pub icon_fdid: u32,
    pub passive: bool,
    pub cooldown_seconds: f32,
    pub tab: String,
}

/// Runtime spell list populated from server data.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct SpellList {
    pub spells: Vec<SpellEntry>,
}

impl SpellList {
    /// Get all spells for a given tab.
    pub fn spells_for_tab(&self, tab: &str) -> Vec<&SpellEntry> {
        self.spells.iter().filter(|s| s.tab == tab).collect()
    }

    /// Get all unique tab names in display order (insertion order).
    pub fn tabs(&self) -> Vec<&str> {
        let mut tabs: Vec<&str> = Vec::new();
        for spell in &self.spells {
            if !tabs.contains(&spell.tab.as_str()) {
                tabs.push(&spell.tab);
            }
        }
        tabs
    }

    /// Find a spell by ID.
    pub fn find(&self, spell_id: u32) -> Option<&SpellEntry> {
        self.spells.iter().find(|s| s.spell_id == spell_id)
    }

    /// Add or update a spell.
    pub fn upsert(&mut self, entry: SpellEntry) {
        if let Some(existing) = self
            .spells
            .iter_mut()
            .find(|s| s.spell_id == entry.spell_id)
        {
            *existing = entry;
        } else {
            self.spells.push(entry);
        }
    }

    /// Remove a spell by ID (e.g. unlearned).
    pub fn remove(&mut self, spell_id: u32) {
        self.spells.retain(|s| s.spell_id != spell_id);
    }

    /// Replace the entire spell list (e.g. on initial sync from server).
    pub fn replace_all(&mut self, spells: Vec<SpellEntry>) {
        self.spells = spells;
    }

    /// Total spell count.
    pub fn len(&self) -> usize {
        self.spells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.spells.is_empty()
    }

    /// Active (non-passive) spells for a tab.
    pub fn active_spells_for_tab(&self, tab: &str) -> Vec<&SpellEntry> {
        self.spells
            .iter()
            .filter(|s| s.tab == tab && !s.passive)
            .collect()
    }

    /// Passive spells for a tab.
    pub fn passive_spells_for_tab(&self, tab: &str) -> Vec<&SpellEntry> {
        self.spells
            .iter()
            .filter(|s| s.tab == tab && s.passive)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spell(id: u32, name: &str, tab: &str) -> SpellEntry {
        SpellEntry {
            spell_id: id,
            name: name.into(),
            icon_fdid: id + 100000,
            passive: false,
            cooldown_seconds: 0.0,
            tab: tab.into(),
        }
    }

    fn passive(id: u32, name: &str, tab: &str) -> SpellEntry {
        SpellEntry {
            passive: true,
            ..spell(id, name, tab)
        }
    }

    fn sample_list() -> SpellList {
        SpellList {
            spells: vec![
                spell(6603, "Auto Attack", "General"),
                spell(35395, "Crusader Strike", "Paladin"),
                spell(19750, "Flash of Light", "Paladin"),
                passive(137026, "Plate Specialization", "Paladin"),
                spell(184575, "Blade of Justice", "Retribution"),
            ],
        }
    }

    #[test]
    fn tabs_returns_unique_ordered() {
        let list = sample_list();
        assert_eq!(list.tabs(), vec!["General", "Paladin", "Retribution"]);
    }

    #[test]
    fn spells_for_tab_filters() {
        let list = sample_list();
        let paladin = list.spells_for_tab("Paladin");
        assert_eq!(paladin.len(), 3);
    }

    #[test]
    fn active_spells_excludes_passive() {
        let list = sample_list();
        let active = list.active_spells_for_tab("Paladin");
        assert_eq!(active.len(), 2);
        assert!(active.iter().all(|s| !s.passive));
    }

    #[test]
    fn passive_spells_only() {
        let list = sample_list();
        let passives = list.passive_spells_for_tab("Paladin");
        assert_eq!(passives.len(), 1);
        assert_eq!(passives[0].name, "Plate Specialization");
    }

    #[test]
    fn find_spell_by_id() {
        let list = sample_list();
        let found = list.find(35395).unwrap();
        assert_eq!(found.name, "Crusader Strike");
        assert!(list.find(99999).is_none());
    }

    #[test]
    fn upsert_adds_new() {
        let mut list = SpellList::default();
        list.upsert(spell(100, "New Spell", "General"));
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn upsert_updates_existing() {
        let mut list = sample_list();
        let count = list.len();
        list.upsert(SpellEntry {
            spell_id: 35395,
            name: "Crusader Strike (Rank 2)".into(),
            icon_fdid: 999,
            passive: false,
            cooldown_seconds: 6.0,
            tab: "Paladin".into(),
        });
        assert_eq!(list.len(), count); // no new entry
        assert_eq!(list.find(35395).unwrap().name, "Crusader Strike (Rank 2)");
    }

    #[test]
    fn remove_spell() {
        let mut list = sample_list();
        let count = list.len();
        list.remove(35395);
        assert_eq!(list.len(), count - 1);
        assert!(list.find(35395).is_none());
    }

    #[test]
    fn remove_nonexistent_no_change() {
        let mut list = sample_list();
        let count = list.len();
        list.remove(99999);
        assert_eq!(list.len(), count);
    }

    #[test]
    fn replace_all_clears_and_sets() {
        let mut list = sample_list();
        list.replace_all(vec![spell(1, "Only Spell", "Tab1")]);
        assert_eq!(list.len(), 1);
        assert_eq!(list.tabs(), vec!["Tab1"]);
    }

    #[test]
    fn empty_list() {
        let list = SpellList::default();
        assert!(list.is_empty());
        assert!(list.tabs().is_empty());
        assert!(list.spells_for_tab("General").is_empty());
    }
}
