//! Blizzard_ClassTalentSearch.lua and Blizzard_SpellSearchTextFilter.lua.
use crate::talents::TalentView;
use game_engine_core::talent_data::TalentNode;

pub const NOT_ON_ACTION_BAR: &str = "Not on Action Bar";
// Blizzard_FrameXMLBase/Constants.lua:296; SpellSearchTemplates.xml:81.
pub const MIN_SEARCH_CHARACTERS: usize = 3;
pub const MAX_PREVIEW_ENTRIES: usize = 3;

impl TalentView {
    /// HeroTalentsContainer.lua:339-362 marks the selector for inactive subtree matches.
    pub fn inactive_hero_search_atlas(&self) -> Option<&'static str> {
        if let Some(selector) = &self.graph.hero_selection
            && let Some(atlas) = self.search_match_atlas(selector)
        {
            return Some(atlas);
        }
        let active = self
            .graph
            .hero_selection
            .as_ref()
            .and_then(|selector| {
                selector
                    .entries
                    .iter()
                    .find(|entry| self.editor.rank(selector.id, entry.id) > 0)
            })
            .map(|entry| entry.subtree_id);
        self.graph
            .heroes
            .iter()
            .filter(|tree| Some(tree.id) != active)
            .flat_map(|tree| &tree.nodes)
            .filter_map(|node| self.search_match_atlas(node))
            .max_by_key(|atlas| match *atlas {
                "talents-search-exactmatch" => 4,
                "talents-search-match" => 3,
                "talents-search-relatedmatch" => 1,
                _ => 2,
            })
    }
    /// Retail previews names, while submitted text also searches descriptions.
    pub fn search_preview_entries(&self) -> Vec<(u32, &str)> {
        if !self.editor.search_preview {
            return Vec::new();
        }
        self.search_name_entries(&self.editor.search_text)
    }
    pub fn search_name_entries(&self, text: &str) -> Vec<(u32, &str)> {
        let query = text.to_lowercase();
        if query.chars().count() < MIN_SEARCH_CHARACTERS {
            return Vec::new();
        }
        let mut entries: Vec<_> = self
            .names
            .iter()
            .filter(|(_, name)| name.to_lowercase().contains(&query))
            .map(|(&id, name)| (id, name.as_str()))
            .collect();
        // SpellSearchFilter.lua:4-24: exact match first, then case-insensitive name.
        entries.sort_by_cached_key(|(id, name)| {
            let lower = name.to_lowercase();
            (lower != query, lower, *id)
        });
        entries.dedup_by(|a, b| a.1 == b.1);
        entries
    }

    pub fn search_match_atlas(&self, node: &TalentNode) -> Option<&'static str> {
        let query = self.editor.search_filter.to_lowercase();
        if query.chars().count() < MIN_SEARCH_CHARACTERS {
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
