use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

pub struct AchievementDef {
    pub id: i32,
    pub name: &'static str,
    pub description: &'static str,
    pub points: u32,
    pub category_id: i32,
}

pub struct AchievementCategoryDef {
    pub id: i32,
    pub name: &'static str,
    pub parent_id: i32,
}

/// Runtime completion state tracked per player session.
#[derive(Resource, Default, Clone, Debug, PartialEq)]
pub struct AchievementCompletionState {
    /// Achievement IDs that have been fully earned.
    pub earned: HashSet<i32>,
    /// In-progress achievements: achievement_id → (current, required).
    pub progress: HashMap<i32, (u32, u32)>,
}

pub static ACHIEVEMENT_CATEGORIES: &[AchievementCategoryDef] = &[
    AchievementCategoryDef {
        id: 92,
        name: "General",
        parent_id: -1,
    },
    AchievementCategoryDef {
        id: 96,
        name: "Quests",
        parent_id: -1,
    },
    AchievementCategoryDef {
        id: 97,
        name: "Exploration",
        parent_id: -1,
    },
    AchievementCategoryDef {
        id: 95,
        name: "PvP",
        parent_id: -1,
    },
    AchievementCategoryDef {
        id: 168,
        name: "Dungeons & Raids",
        parent_id: -1,
    },
    AchievementCategoryDef {
        id: 169,
        name: "Professions",
        parent_id: -1,
    },
    AchievementCategoryDef {
        id: 201,
        name: "Reputation",
        parent_id: -1,
    },
    AchievementCategoryDef {
        id: 155,
        name: "World Events",
        parent_id: -1,
    },
    AchievementCategoryDef {
        id: 81,
        name: "Feats of Strength",
        parent_id: -1,
    },
];

pub static ACHIEVEMENTS: &[AchievementDef] = &[
    // General — level milestones
    AchievementDef {
        id: 1,
        name: "Level 10",
        description: "Reach level 10.",
        points: 10,
        category_id: 92,
    },
    AchievementDef {
        id: 2,
        name: "Level 20",
        description: "Reach level 20.",
        points: 10,
        category_id: 92,
    },
    AchievementDef {
        id: 3,
        name: "Level 40",
        description: "Reach level 40.",
        points: 10,
        category_id: 92,
    },
    AchievementDef {
        id: 4,
        name: "Level 60",
        description: "Reach level 60.",
        points: 10,
        category_id: 92,
    },
    AchievementDef {
        id: 5,
        name: "Level 70",
        description: "Reach level 70.",
        points: 10,
        category_id: 92,
    },
    AchievementDef {
        id: 6,
        name: "Level 80",
        description: "Reach level 80.",
        points: 10,
        category_id: 92,
    },
    // Quests
    AchievementDef {
        id: 10,
        name: "Quest Adept",
        description: "Complete 50 quests.",
        points: 10,
        category_id: 96,
    },
    AchievementDef {
        id: 11,
        name: "Loremaster Initiate",
        description: "Complete 100 quests.",
        points: 10,
        category_id: 96,
    },
    AchievementDef {
        id: 12,
        name: "250 Quests Completed",
        description: "Complete 250 quests.",
        points: 25,
        category_id: 96,
    },
    // Exploration
    AchievementDef {
        id: 20,
        name: "Explore Elwynn Forest",
        description: "Explore the wilds of Elwynn Forest.",
        points: 10,
        category_id: 97,
    },
    AchievementDef {
        id: 21,
        name: "Explore Durotar",
        description: "Explore the harsh terrain of Durotar.",
        points: 10,
        category_id: 97,
    },
    // PvP
    AchievementDef {
        id: 30,
        name: "Honorable Kill",
        description: "Earn an honorable kill.",
        points: 10,
        category_id: 95,
    },
    AchievementDef {
        id: 31,
        name: "100 Honorable Kills",
        description: "Earn 100 honorable kills.",
        points: 10,
        category_id: 95,
    },
    // Dungeons & Raids
    AchievementDef {
        id: 40,
        name: "The Deadmines",
        description: "Defeat the final boss of the Deadmines.",
        points: 10,
        category_id: 168,
    },
    AchievementDef {
        id: 41,
        name: "Shadowfang Keep",
        description: "Defeat the final boss of Shadowfang Keep.",
        points: 10,
        category_id: 168,
    },
    // Professions
    AchievementDef {
        id: 50,
        name: "Professional Journeyman",
        description: "Reach Journeyman level in a profession.",
        points: 10,
        category_id: 169,
    },
    AchievementDef {
        id: 51,
        name: "Professional Expert",
        description: "Reach Expert level in a profession.",
        points: 10,
        category_id: 169,
    },
    // Reputation
    AchievementDef {
        id: 60,
        name: "Ambassador",
        description: "Earn Exalted status with five home-city factions.",
        points: 10,
        category_id: 201,
    },
    // World Events
    AchievementDef {
        id: 70,
        name: "To Honor One's Elders",
        description: "Complete the Lunar Festival achievements.",
        points: 25,
        category_id: 155,
    },
    // Feats of Strength
    AchievementDef {
        id: 80,
        name: "Old School Ride",
        description: "Obtain a rare vintage mount.",
        points: 0,
        category_id: 81,
    },
];

/// Returns categories to display for the given tab index.
/// Tab 0 = Achievements (all categories), tab 1 = Statistics (empty).
pub fn categories_for_tab(tab_index: usize) -> Vec<&'static AchievementCategoryDef> {
    if tab_index == 0 {
        ACHIEVEMENT_CATEGORIES.iter().collect()
    } else {
        vec![]
    }
}

/// Returns all achievements belonging to `category_id`.
pub fn achievements_for_category(category_id: i32) -> Vec<&'static AchievementDef> {
    ACHIEVEMENTS
        .iter()
        .filter(|a| a.category_id == category_id)
        .collect()
}

/// Returns a flat ordered list of `(id, name, is_child)` for the category
/// sidebar.  All current categories are root-level, so `is_child` is always
/// `false`.
pub fn build_category_tree() -> Vec<(i32, &'static str, bool)> {
    ACHIEVEMENT_CATEGORIES
        .iter()
        .map(|c| (c.id, c.name, c.parent_id != -1))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categories_for_tab_achievements_returns_all() {
        let cats = categories_for_tab(0);
        assert_eq!(cats.len(), 9);
    }

    #[test]
    fn categories_for_tab_statistics_returns_empty() {
        let cats = categories_for_tab(1);
        assert!(cats.is_empty());
    }

    #[test]
    fn achievements_for_category_general() {
        let achs = achievements_for_category(92);
        assert_eq!(achs.len(), 6, "expected 6 level achievements in General");
        for a in &achs {
            assert_eq!(a.category_id, 92);
        }
    }

    #[test]
    fn build_category_tree_has_all_roots() {
        let tree = build_category_tree();
        assert_eq!(tree.len(), 9);
        for (_, _, is_child) in &tree {
            assert!(!is_child, "all categories should be root level");
        }
    }

    #[test]
    fn completion_state_tracks_earned() {
        let mut state = AchievementCompletionState::default();
        state.earned.insert(42);
        assert!(state.earned.contains(&42));
        assert!(!state.earned.contains(&99));
    }
}
