use bevy::prelude::*;

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
}
