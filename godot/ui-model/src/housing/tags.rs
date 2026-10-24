use super::{Decor, HousingAuthority, HousingDashboard};
use game_engine_core::housing::DataTagGroup;
use std::collections::BTreeSet;

/// HOUSING_TAG_MEMBERSHIP_UNQUALIFIED: not a match or nonmatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CatalogTagGap {
    pub decor_id: u32,
    pub group_id: u32,
}

impl CatalogTagGap {
    pub fn code(&self) -> &'static str {
        "HOUSING_TAG_MEMBERSHIP_UNQUALIFIED"
    }
}

#[derive(Debug, Default)]
pub struct CatalogQuery<'a> {
    pub matches: Vec<&'a Decor>,
    pub unqualified: Vec<CatalogTagGap>,
}

impl HousingDashboard {
    /// Local query outcome, not a claim about CAPI defaults/untagged behavior.
    pub fn catalog_query(&self, authority: &HousingAuthority) -> CatalogQuery<'_> {
        let mut result = CatalogQuery::default();
        for decor in self.base_catalog_matches(authority) {
            let gaps = self.catalog_tag_gaps(decor.id);
            if !gaps.is_empty() {
                result.unqualified.extend(gaps);
                continue;
            }
            if self.catalog_tags_match(decor.id) {
                result.matches.push(decor);
            }
        }
        result
    }

    fn catalog_tag_gaps(&self, decor_id: u32) -> Vec<CatalogTagGap> {
        let tags = self.catalog.tags.decor.get(&decor_id);
        self.tag_selections
            .keys()
            .filter_map(|group_id| {
                let group = &self.catalog.tags.groups[group_id];
                let authored = tags.is_some_and(|tags| !tags.is_disjoint(&group.values));
                (!authored).then_some(CatalogTagGap {
                    decor_id,
                    group_id: *group_id,
                })
            })
            .collect()
    }

    fn catalog_tags_match(&self, decor_id: u32) -> bool {
        self.tag_selections.values().all(|selected| {
            self.catalog
                .tags
                .decor
                .get(&decor_id)
                .is_some_and(|tags| !tags.is_disjoint(selected))
        })
    }

    fn catalog_tag_group(&self, id: u32) -> Result<&DataTagGroup, String> {
        if !self.catalog.tags.available {
            return Err("Housing tag sources are unavailable.".into());
        }
        self.catalog
            .tags
            .groups
            .get(&id)
            .ok_or_else(|| format!("Housing DataTagGroup {id} is unavailable."))
    }

    pub fn toggle_catalog_tag(
        &mut self,
        group_id: u32,
        value: u32,
        authority: &HousingAuthority,
    ) -> Result<(), String> {
        let group = self.catalog_tag_group(group_id)?;
        if !group.values.contains(&value) {
            return Err(format!(
                "DataTag {value} is not in housing group {group_id}."
            ));
        }
        let all = group.values.clone();
        let selected = self
            .tag_selections
            .entry(group_id)
            .or_insert_with(|| all.clone());
        if !selected.remove(&value) {
            selected.insert(value);
        }
        if selected == &all {
            self.tag_selections.remove(&group_id);
        }
        self.reset_results(authority);
        Ok(())
    }

    pub fn set_all_catalog_tags(
        &mut self,
        group_id: u32,
        checked: bool,
        authority: &HousingAuthority,
    ) -> Result<(), String> {
        self.catalog_tag_group(group_id)?;
        if checked {
            self.tag_selections.remove(&group_id);
        } else {
            self.tag_selections.insert(group_id, BTreeSet::new());
        }
        self.reset_results(authority);
        Ok(())
    }
}
