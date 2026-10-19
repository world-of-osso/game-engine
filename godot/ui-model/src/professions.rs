//! Owner profession book and crafting decisions; server snapshots remain authoritative.
use crate::bag_data::InventoryState;
use shared::protocol::{CraftRecipe, ProfessionSnapshot, SetRecipeFavorite};

pub const FAVORITES_GROUP: &str = "Favorites";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recipe {
    pub spell_id: u32,
    pub skill_line: u32,
    /// Parent profession SkillLine, used only for window title/art.
    pub profession: u32,
    pub category: String,
    pub category_order: i32,
    pub name: String,
    pub min_rank: u16,
    pub trivial_low: u16,
    pub trivial_high: u16,
    /// Fixed CREATE_ITEM24 output only. Quality operation results remain unassigned.
    pub output: Option<(u32, u32)>,
    pub reagents: Vec<(u32, u32)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProfessionBook {
    pub snapshot: ProfessionSnapshot,
    pub recipes: Vec<Recipe>,
    pub visible: bool,
    pub selected: Option<u32>,
    pub search: String,
    /// Inverted storage keeps both Retail Show checkboxes enabled by default.
    pub hide_learned: bool,
    pub hide_unlearned: bool,
    pub quantity: u32,
}

impl ProfessionBook {
    pub fn groups(&self) -> Vec<(String, Vec<&Recipe>)> {
        let known: std::collections::HashSet<_> = self.snapshot.spells.iter().copied().collect();
        let search = self.search.trim().to_lowercase();
        let mut groups = std::collections::BTreeMap::<(bool, String), Vec<&Recipe>>::new();
        for recipe in &self.recipes {
            let learned = known.contains(&recipe.spell_id);
            if (learned && self.hide_learned) || (!learned && self.hide_unlearned) {
                continue;
            }
            if recipe.name.to_lowercase().contains(&search) {
                groups
                    .entry((!learned, recipe.category.clone()))
                    .or_default()
                    .push(recipe);
            }
        }
        for recipes in groups.values_mut() {
            recipes.sort_by(|left, right| left.name.cmp(&right.name));
        }
        let mut groups: Vec<_> = groups.into_iter().collect();
        groups.sort_by_key(|((unlearned, _), recipes)| (*unlearned, recipes[0].category_order));
        let mut groups: Vec<_> = groups
            .into_iter()
            .map(|((_, category), recipes)| (category, recipes))
            .collect();
        if search.is_empty() && !self.hide_learned {
            let favorite_ids: std::collections::HashSet<_> =
                self.snapshot.favorites.iter().copied().collect();
            let mut favorites: Vec<_> = self
                .recipes
                .iter()
                .filter(|recipe| {
                    known.contains(&recipe.spell_id) && favorite_ids.contains(&recipe.spell_id)
                })
                .collect();
            favorites.sort_by(|left, right| left.name.cmp(&right.name));
            if !favorites.is_empty() {
                groups.insert(0, (FAVORITES_GROUP.into(), favorites));
            }
        }
        groups
    }

    /// Professions.SetDefaultFilters restores both Show checkboxes without clearing search.
    pub fn reset_visibility_filters(&mut self) {
        self.hide_learned = false;
        self.hide_unlearned = false;
    }

    pub fn favorite_request(&self, spell_id: u32, favorite: bool) -> Option<SetRecipeFavorite> {
        self.snapshot
            .spells
            .contains(&spell_id)
            .then_some(SetRecipeFavorite { spell_id, favorite })
    }

    pub fn selected_recipe(&self) -> Option<&Recipe> {
        let spell = self.selected?;
        self.recipes.iter().find(|recipe| recipe.spell_id == spell)
    }

    pub fn selected_is_learned(&self) -> bool {
        self.selected
            .is_some_and(|spell| self.snapshot.spells.contains(&spell))
    }

    /// `(item ID, owned across bags, needed per cast)`; equipment is never consumed.
    pub fn reagent_counts(&self, bags: &InventoryState) -> Vec<(u32, u32, u32)> {
        let Some(recipe) = self.selected_recipe() else {
            return Vec::new();
        };
        recipe
            .reagents
            .iter()
            .map(|&(item, needed)| {
                let owned = bags
                    .slots
                    .iter()
                    .flatten()
                    .filter(|slot| slot.item_id == item)
                    .fold(0u32, |count, slot| count.saturating_add(slot.count));
                (item, owned, needed)
            })
            .collect()
    }

    pub fn craftable_count(&self, bags: &InventoryState) -> u32 {
        // MinSkillLineRank gates automatic learning, not casts of a trainer-learned spell.
        // Recipe knowledge is authoritative (TrinityCore Player::LearnSkillRewardedSpells).
        if !self.selected_is_learned()
            || self
                .selected_recipe()
                .and_then(|recipe| recipe.output)
                .is_none()
        {
            return 0;
        }
        self.reagent_counts(bags)
            .iter()
            .filter(|(_, _, needed)| *needed > 0)
            .map(|(_, owned, needed)| owned / needed)
            .min()
            .unwrap_or(1)
            .min(u32::from(u16::MAX))
    }

    pub fn craft_request(&self, bags: &InventoryState, all: bool) -> Option<CraftRecipe> {
        let available = self.craftable_count(bags);
        let casts = if all { available } else { self.quantity };
        if casts == 0 || casts > available {
            return None;
        }
        Some(CraftRecipe {
            spell_id: self.selected_recipe()?.spell_id,
            casts: u16::try_from(casts).ok()?,
        })
    }
}

#[cfg(test)]
#[path = "professions_tests.rs"]
mod tests;
