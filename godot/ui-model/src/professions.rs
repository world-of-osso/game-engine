//! Owner profession book and crafting decisions; server snapshots remain authoritative.
use crate::bag_data::InventoryState;
use shared::protocol::{CraftRecipe, ProfessionSnapshot};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recipe {
    pub spell_id: u32,
    pub skill_line: u32,
    pub category: String,
    pub name: String,
    pub min_rank: u16,
    pub trivial_low: u16,
    pub trivial_high: u16,
    pub output: (u32, u32),
    pub reagents: Vec<(u32, u32)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProfessionBook {
    pub snapshot: ProfessionSnapshot,
    pub recipes: Vec<Recipe>,
    pub visible: bool,
    pub selected: Option<u32>,
    pub search: String,
    pub quantity: u32,
}

impl ProfessionBook {
    pub fn groups(&self) -> Vec<(String, Vec<&Recipe>)> {
        let known: std::collections::HashSet<_> = self.snapshot.spells.iter().copied().collect();
        let search = self.search.trim().to_lowercase();
        let mut groups = std::collections::BTreeMap::<String, Vec<&Recipe>>::new();
        for recipe in &self.recipes {
            if known.contains(&recipe.spell_id) && recipe.name.to_lowercase().contains(&search) {
                groups
                    .entry(recipe.category.clone())
                    .or_default()
                    .push(recipe);
            }
        }
        for recipes in groups.values_mut() {
            recipes.sort_by(|left, right| left.name.cmp(&right.name));
        }
        groups.into_iter().collect()
    }

    pub fn selected_recipe(&self) -> Option<&Recipe> {
        let spell = self.selected?;
        self.recipes
            .iter()
            .find(|recipe| recipe.spell_id == spell && self.snapshot.spells.contains(&spell))
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
        let Some(recipe) = self.selected_recipe() else {
            return 0;
        };
        let rank = self
            .snapshot
            .lines
            .iter()
            .find(|line| line.skill_line == recipe.skill_line)
            .map_or(0, |line| line.rank);
        if rank < recipe.min_rank {
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
