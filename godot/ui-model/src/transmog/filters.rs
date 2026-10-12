//! Retail Blizzard_Transmog.lua source and set-completion filters.
//! Raw identities: Blizzard_APIDocumentationGenerated/TransmogSharedDocumentation.lua,
//! Enum.TransmogSource. Special states None/HiddenUntilCollected/CantCollect/NotValid
//! are not acquisition categories and must not be relabelled as menu categories.
use super::*;

pub const SOURCE_FILTERS: [(u8, &str); 7] = [
    (1, "Boss Drop"),
    (2, "Quest"),
    (3, "Vendor"),
    (4, "World Drop"),
    (7, "Achievement"),
    (8, "Profession"),
    (10, "Trading Post"),
];

impl TransmogBook {
    pub fn set_all_source_filters(&mut self, checked: bool) {
        self.excluded_source_types = if checked {
            BTreeSet::new()
        } else {
            (0..=10).collect()
        };
        self.page = 0;
    }

    pub fn toggle_source_filter(&mut self, source_type: u8) -> bool {
        if !SOURCE_FILTERS.iter().any(|(id, _)| *id == source_type) {
            return false;
        }
        if !self.excluded_source_types.remove(&source_type) {
            self.excluded_source_types.insert(source_type);
        }
        self.page = 0;
        true
    }

    pub fn toggle_collected_sets(&mut self) {
        self.exclude_collected_sets = !self.exclude_collected_sets;
        self.page = 0;
    }

    pub fn toggle_uncollected_sets(&mut self) {
        self.exclude_uncollected_sets = !self.exclude_uncollected_sets;
        self.page = 0;
    }

    pub(super) fn matches_set_completion(&self, progress: SetProgress) -> bool {
        if progress.collected == progress.total {
            !self.exclude_collected_sets
        } else {
            !self.exclude_uncollected_sets
        }
    }
}
