use bevy::prelude::*;

pub mod textures {
    pub const ICON_ALCHEMY: u32 = 136240;
    pub const ICON_BLACKSMITHING: u32 = 136241;
    pub const ICON_MINING: u32 = 136248;
    /// Professions book frame (left page).
    pub const BOOK_LEFT: u32 = 383588;
    /// Skill progress bar fill.
    pub const PROGRESS_FILL: u32 = 383590;
    /// Item slot border (shared).
    pub const SLOT_BORDER: u32 = 130862;
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReagentRequirement {
    pub item_name: String,
    pub icon_fdid: u32,
    pub required: u32,
    pub have: u32,
}

impl ReagentRequirement {
    pub fn is_satisfied(&self) -> bool {
        self.have >= self.required
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecipeDef {
    pub id: u32,
    pub name: String,
    pub profession: String,
    pub skill_required: u32,
    pub reagents: Vec<ReagentRequirement>,
    pub learned: bool,
}

impl RecipeDef {
    pub fn can_craft(&self, skill_level: u32) -> bool {
        self.learned
            && skill_level >= self.skill_required
            && self.reagents.iter().all(|r| r.is_satisfied())
    }
}

/// Runtime professions state.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct ProfessionsState {
    pub recipes: Vec<RecipeDef>,
    pub skill_level: u32,
    pub skill_max: u32,
    pub craft_queue: Vec<u32>,
}

impl ProfessionsState {
    pub fn learned_count(&self) -> usize {
        self.recipes.iter().filter(|r| r.learned).count()
    }

    pub fn craftable_count(&self) -> usize {
        self.recipes
            .iter()
            .filter(|r| r.can_craft(self.skill_level))
            .count()
    }

    pub fn skill_text(&self) -> String {
        format!("{}/{}", self.skill_level, self.skill_max)
    }

    pub fn is_crafting(&self) -> bool {
        !self.craft_queue.is_empty()
    }

    /// Filter recipes by name (case-insensitive substring).
    pub fn filter_by_name(&self, query: &str) -> Vec<&RecipeDef> {
        let q = query.to_lowercase();
        self.recipes
            .iter()
            .filter(|r| r.name.to_lowercase().contains(&q))
            .collect()
    }

    /// Filter to only craftable recipes (learned + skill + reagents).
    pub fn craftable_recipes(&self) -> Vec<&RecipeDef> {
        self.recipes
            .iter()
            .filter(|r| r.can_craft(self.skill_level))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reagent(name: &str, required: u32, have: u32) -> ReagentRequirement {
        ReagentRequirement {
            item_name: name.into(),
            icon_fdid: 0,
            required,
            have,
        }
    }

    fn recipe(
        name: &str,
        skill: u32,
        learned: bool,
        reagents: Vec<ReagentRequirement>,
    ) -> RecipeDef {
        RecipeDef {
            id: 1,
            name: name.into(),
            profession: "Alchemy".into(),
            skill_required: skill,
            reagents,
            learned,
        }
    }

    #[test]
    fn reagent_satisfied() {
        assert!(reagent("Herb", 2, 5).is_satisfied());
        assert!(!reagent("Herb", 5, 2).is_satisfied());
    }

    #[test]
    fn can_craft_checks() {
        let r = recipe("Potion", 50, true, vec![reagent("Herb", 2, 5)]);
        assert!(r.can_craft(100));
        assert!(!r.can_craft(30));
        let unlearned = recipe("Potion", 50, false, vec![]);
        assert!(!unlearned.can_craft(100));
    }

    #[test]
    fn learned_and_craftable_counts() {
        let state = ProfessionsState {
            recipes: vec![
                recipe("A", 10, true, vec![reagent("X", 1, 1)]),
                recipe("B", 10, true, vec![reagent("X", 5, 1)]),
                recipe("C", 10, false, vec![]),
            ],
            skill_level: 100,
            skill_max: 300,
            craft_queue: vec![],
        };
        assert_eq!(state.learned_count(), 2);
        assert_eq!(state.craftable_count(), 1);
    }

    #[test]
    fn skill_text_format() {
        let state = ProfessionsState {
            skill_level: 150,
            skill_max: 300,
            ..Default::default()
        };
        assert_eq!(state.skill_text(), "150/300");
    }

    #[test]
    fn craft_queue() {
        let mut state = ProfessionsState::default();
        assert!(!state.is_crafting());
        state.craft_queue.push(1);
        assert!(state.is_crafting());
    }

    #[test]
    fn texture_fdids_are_nonzero() {
        assert_ne!(textures::ICON_ALCHEMY, 0);
        assert_ne!(textures::ICON_BLACKSMITHING, 0);
        assert_ne!(textures::BOOK_LEFT, 0);
        assert_ne!(textures::PROGRESS_FILL, 0);
    }

    // --- Recipe filtering ---

    fn make_state() -> ProfessionsState {
        ProfessionsState {
            recipes: vec![
                recipe(
                    "Minor Healing Potion",
                    1,
                    true,
                    vec![reagent("Peacebloom", 1, 5), reagent("Silverleaf", 1, 3)],
                ),
                recipe("Healing Potion", 55, true, vec![reagent("Liferoot", 1, 0)]),
                recipe(
                    "Greater Healing Potion",
                    155,
                    true,
                    vec![reagent("Sungrass", 1, 2)],
                ),
                recipe("Flask of Titans", 300, false, vec![]),
            ],
            skill_level: 200,
            skill_max: 300,
            craft_queue: vec![],
        }
    }

    #[test]
    fn filter_by_name_finds_matches() {
        let state = make_state();
        let results = state.filter_by_name("healing");
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn filter_by_name_case_insensitive() {
        let state = make_state();
        let results = state.filter_by_name("FLASK");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Flask of Titans");
    }

    #[test]
    fn filter_by_name_no_match() {
        let state = make_state();
        assert!(state.filter_by_name("Elixir").is_empty());
    }

    #[test]
    fn craftable_recipes_filters_correctly() {
        let state = make_state();
        let craftable = state.craftable_recipes();
        // Minor Healing: learned, skill 1 <= 200, reagents satisfied → YES
        // Healing: learned, skill 55 <= 200, but Liferoot 0/1 → NO
        // Greater Healing: learned, skill 155 <= 200, reagents satisfied → YES
        // Flask: not learned → NO
        assert_eq!(craftable.len(), 2);
        assert_eq!(craftable[0].name, "Minor Healing Potion");
        assert_eq!(craftable[1].name, "Greater Healing Potion");
    }

    // --- Reagent availability ---

    #[test]
    fn reagent_exact_match() {
        assert!(reagent("X", 5, 5).is_satisfied());
    }

    #[test]
    fn reagent_zero_required() {
        assert!(reagent("X", 0, 0).is_satisfied());
    }

    #[test]
    fn multiple_reagents_all_must_satisfy() {
        let r = recipe(
            "Complex",
            1,
            true,
            vec![
                reagent("A", 2, 5),
                reagent("B", 3, 3),
                reagent("C", 1, 0), // not satisfied
            ],
        );
        assert!(!r.can_craft(100));
    }

    #[test]
    fn can_craft_at_exact_skill_level() {
        let r = recipe("Exact", 100, true, vec![reagent("A", 1, 1)]);
        assert!(r.can_craft(100));
        assert!(!r.can_craft(99));
    }

    #[test]
    fn craftable_count_matches_filter() {
        let state = make_state();
        assert_eq!(state.craftable_count(), state.craftable_recipes().len());
    }
}
