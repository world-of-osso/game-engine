//! Retail storage owns an independent category focus; only confirmed counts qualify.
use crate::housing::{ALL_CATEGORY, FEATURED_CATEGORY, HousingAuthority, HousingDashboard};
use game_engine_core::housing::{Category, Decor, Subcategory};
use std::collections::BTreeSet;

impl HousingDashboard {
    fn owned_storage_subcategory_ids(&self, authority: &HousingAuthority) -> BTreeSet<u32> {
        let Some(state) = &authority.state else {
            return BTreeSet::new();
        };
        let owned: BTreeSet<_> = state
            .stored
            .iter()
            .chain(&state.placed)
            .filter(|entry| entry.quantity > 0)
            .map(|entry| entry.decor_id)
            .collect();
        self.catalog
            .decor
            .iter()
            .filter(|decor| owned.contains(&decor.id))
            .flat_map(|decor| decor.subcategories.iter().copied())
            .collect()
    }

    pub fn storage_categories(&self, authority: &HousingAuthority) -> Vec<&Category> {
        let owned = self.owned_storage_subcategory_ids(authority);
        self.catalog
            .categories
            .iter()
            .filter(|category| {
                if category.id == ALL_CATEGORY {
                    return true;
                }
                if category.id == FEATURED_CATEGORY {
                    return false;
                }
                self.catalog
                    .subcategories
                    .iter()
                    .any(|child| child.category_id == category.id && owned.contains(&child.id))
            })
            .collect()
    }

    pub fn storage_subcategories(
        &self,
        authority: &HousingAuthority,
        category_id: u32,
    ) -> Vec<&Subcategory> {
        let owned = self.owned_storage_subcategory_ids(authority);
        self.catalog
            .subcategories
            .iter()
            .filter(|child| child.category_id == category_id && owned.contains(&child.id))
            .collect()
    }

    pub fn set_storage_search(&mut self, search: String) {
        self.storage_search = search;
        if !self.storage_search.is_empty() {
            self.storage_category = None;
            self.storage_subcategory = None;
        }
    }

    pub fn set_storage_category(
        &mut self,
        id: Option<u32>,
        authority: &HousingAuthority,
    ) -> Result<(), String> {
        let id = id.filter(|id| *id != ALL_CATEGORY);
        if let Some(id) = id {
            let available = self
                .storage_categories(authority)
                .iter()
                .any(|category| category.id == id);
            if !available {
                return Err("Storage category has no confirmed owned entries.".into());
            }
            self.storage_search.clear();
        }
        self.storage_category = id;
        self.storage_subcategory = None;
        Ok(())
    }

    pub fn set_storage_subcategory(
        &mut self,
        id: Option<u32>,
        authority: &HousingAuthority,
    ) -> Result<(), String> {
        let category = self
            .storage_category
            .ok_or("Select a storage category first.")?;
        if let Some(id) = id {
            let available = self
                .storage_subcategories(authority, category)
                .iter()
                .any(|child| child.id == id);
            if !available {
                return Err("Storage subcategory is not in the confirmed owned category.".into());
            }
        }
        self.storage_subcategory = id;
        self.storage_search.clear();
        Ok(())
    }

    pub(crate) fn storage_decor_matches(&self, decor: &Decor) -> bool {
        let category = self.storage_category.is_none_or(|id| {
            self.catalog
                .subcategories
                .iter()
                .any(|child| child.category_id == id && decor.subcategories.contains(&child.id))
        });
        let subcategory = self
            .storage_subcategory
            .is_none_or(|id| decor.subcategories.contains(&id));
        category && subcategory
    }

    pub(crate) fn expanded_storage_category(&self) -> Option<&Category> {
        let id = self.storage_category?;
        let count = self
            .catalog
            .subcategories
            .iter()
            .filter(|child| child.category_id == id)
            .count();
        if count <= 1 {
            return None;
        }
        self.catalog
            .categories
            .iter()
            .find(|category| category.id == id)
    }

    pub(crate) fn storage_category_path(&self) -> String {
        let id = self.storage_category.unwrap_or(ALL_CATEGORY);
        let Some(category) = self
            .catalog
            .categories
            .iter()
            .find(|category| category.id == id)
        else {
            return String::new();
        };
        let child = self.storage_subcategory.and_then(|id| {
            self.catalog
                .subcategories
                .iter()
                .find(|child| child.id == id)
        });
        match child {
            Some(child) => format!("{} | {}", category.name, child.name),
            None => category.name.clone(),
        }
    }
}
