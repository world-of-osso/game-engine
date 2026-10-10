//! Blizzard_ClassTalentSearch.lua and Blizzard_SpellSearchTextFilter.lua.
use crate::talents::TalentView;
use game_engine_core::talent_data::TalentNode;

pub const NOT_ON_ACTION_BAR: &str = "Not on Action Bar";

impl TalentView {
    /// Retail previews names, while submitted text also searches descriptions.
    pub fn search_preview_entries(&self) -> Vec<(u32, &str)> {
        let query = self.editor.search_text.to_lowercase();
        if query.chars().count() < 2 || !self.editor.search_preview {
            return Vec::new();
        }
        let mut entries: Vec<_> = self
            .names
            .iter()
            .filter(|(_, name)| name.to_lowercase().contains(&query))
            .map(|(&id, name)| (id, name.as_str()))
            .collect();
        entries.sort_by(|a, b| a.1.cmp(b.1).then(a.0.cmp(&b.0)));
        entries.dedup_by(|a, b| a.1 == b.1);
        entries
    }

    pub fn search_match_atlas(&self, node: &TalentNode) -> Option<&'static str> {
        let query = self.editor.search_filter.to_lowercase();
        if query.chars().count() < 2 {
            return None;
        }
        if query == NOT_ON_ACTION_BAR.to_lowercase() {
            return node
                .entries
                .iter()
                .any(|entry| {
                    self.entry_rank(node, entry.id) > 0
                        && !self.on_action_bar.contains(&entry.spell_id)
                        && self
                            .rules
                            .as_ref()
                            .and_then(|tree| tree.node(node.id))
                            .and_then(|rule| rule.entry(entry.id))
                            .is_some_and(|rule| !rule.passive && rule.spell_id != 0)
                })
                .then_some("talents-search-notonactionbar");
        }
        let exact_description = self
            .names
            .iter()
            .find(|(_, name)| name.to_lowercase() == query)
            .and_then(|(entry, _)| self.descriptions.get(entry))
            .map(|text| text.to_lowercase());
        let mut best = 0;
        for entry in &node.entries {
            let Some(name) = self.names.get(&entry.id) else {
                continue;
            };
            let name = name.to_lowercase();
            if name.is_empty() {
                continue;
            }
            let rank = if name == query {
                4
            } else if name.contains(&query)
                || self
                    .replaced_names
                    .get(&entry.id)
                    .is_some_and(|name| name.to_lowercase().contains(&query))
            {
                3
            } else if self
                .descriptions
                .get(&entry.id)
                .is_some_and(|text| text.to_lowercase().contains(&query))
            {
                2
            } else if exact_description
                .as_ref()
                .is_some_and(|text| text.contains(&name))
            {
                1
            } else {
                0
            };
            best = best.max(rank);
        }
        match best {
            4 => Some("talents-search-exactmatch"),
            2 | 3 => Some("talents-search-match"),
            1 => Some("talents-search-relatedmatch"),
            _ => None,
        }
    }
}
