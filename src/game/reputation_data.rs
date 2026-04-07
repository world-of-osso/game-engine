use bevy::prelude::*;

pub mod textures {
    // --- Frame chrome ---
    /// Reputation bar texture (paperdoll frame).
    pub const REP_BAR: u32 = 136567;
    /// Reputation bar highlight (hover).
    pub const REP_BAR_HIGHLIGHT: u32 = 136566;
    /// Reputation watch bar (mini bar).
    pub const REP_WATCH_BAR: u32 = 136581;

    // --- Faction icons ---
    /// Alliance PvP emblem (faction placeholder).
    pub const FACTION_ICON_ALLIANCE: u32 = 136998;
    /// Horde PvP emblem (faction placeholder).
    pub const FACTION_ICON_HORDE: u32 = 137000;
    /// Generic faction change icon.
    pub const FACTION_ICON_GENERIC: u32 = 939373;
    /// Champions of Azeroth (BfA faction example).
    pub const FACTION_ICON_CHAMPIONS: u32 = 2032592;
    /// Tortollan Seekers (BfA faction example).
    pub const FACTION_ICON_TORTOLLAN: u32 = 2032598;

    // --- Paragon ---
    /// Treasure chest icon (paragon reward).
    pub const PARAGON_REWARD_CHEST: u32 = 1542843;

    // --- Bar fill ---
    /// Casting/health bar fill (shared).
    pub const BAR_FILL: u32 = 4505182;
}

// --- Standing ---

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Standing {
    Hated,
    Hostile,
    Unfriendly,
    #[default]
    Neutral,
    Friendly,
    Honored,
    Revered,
    Exalted,
}

impl Standing {
    pub fn label(self) -> &'static str {
        match self {
            Self::Hated => "Hated",
            Self::Hostile => "Hostile",
            Self::Unfriendly => "Unfriendly",
            Self::Neutral => "Neutral",
            Self::Friendly => "Friendly",
            Self::Honored => "Honored",
            Self::Revered => "Revered",
            Self::Exalted => "Exalted",
        }
    }

    /// Reputation threshold to reach this standing (cumulative from 0).
    pub fn threshold(self) -> i32 {
        match self {
            Self::Hated => -42000,
            Self::Hostile => -6000,
            Self::Unfriendly => -3000,
            Self::Neutral => 0,
            Self::Friendly => 3000,
            Self::Honored => 9000,
            Self::Revered => 21000,
            Self::Exalted => 42000,
        }
    }

    /// Max reputation points within this standing bracket.
    pub fn bracket_size(self) -> u32 {
        match self {
            Self::Hated => 36000,
            Self::Hostile => 3000,
            Self::Unfriendly => 3000,
            Self::Neutral => 3000,
            Self::Friendly => 6000,
            Self::Honored => 12000,
            Self::Revered => 21000,
            Self::Exalted => 0,
        }
    }

    pub const ALL: [Standing; 8] = [
        Self::Hated,
        Self::Hostile,
        Self::Unfriendly,
        Self::Neutral,
        Self::Friendly,
        Self::Honored,
        Self::Revered,
        Self::Exalted,
    ];
}

// --- Paragon ---

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ParagonProgress {
    pub current: u32,
    pub max: u32,
    pub reward_pending: bool,
    pub completions: u32,
}

impl ParagonProgress {
    pub fn fraction(&self) -> f32 {
        if self.max == 0 {
            return 0.0;
        }
        (self.current as f32 / self.max as f32).min(1.0)
    }

    pub fn progress_text(&self) -> String {
        format!("{}/{}", self.current, self.max)
    }
}

// --- Faction ---

#[derive(Clone, Debug, PartialEq)]
pub struct Faction {
    pub id: u32,
    pub name: String,
    pub standing: Standing,
    /// Current progress within the standing bracket.
    pub current: u32,
    /// Maximum for this standing bracket (same as `standing.bracket_size()`).
    pub max: u32,
    pub paragon: Option<ParagonProgress>,
    pub at_war: bool,
}

impl Faction {
    pub fn progress_fraction(&self) -> f32 {
        if self.max == 0 {
            return if self.standing == Standing::Exalted {
                1.0
            } else {
                0.0
            };
        }
        (self.current as f32 / self.max as f32).min(1.0)
    }

    pub fn progress_text(&self) -> String {
        format!("{}/{}", self.current, self.max)
    }

    pub fn has_paragon(&self) -> bool {
        self.paragon.is_some()
    }

    pub fn is_exalted(&self) -> bool {
        self.standing == Standing::Exalted
    }
}

// --- Category ---

#[derive(Clone, Debug, PartialEq)]
pub struct FactionCategory {
    pub name: String,
    pub factions: Vec<Faction>,
}

// --- Runtime resource ---

/// Runtime reputation state, held as a Bevy Resource.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct ReputationState {
    pub categories: Vec<FactionCategory>,
}

impl ReputationState {
    pub fn faction_count(&self) -> usize {
        self.categories.iter().map(|c| c.factions.len()).sum()
    }

    pub fn exalted_count(&self) -> usize {
        self.categories
            .iter()
            .flat_map(|c| &c.factions)
            .filter(|f| f.is_exalted())
            .count()
    }

    pub fn find_faction(&self, faction_id: u32) -> Option<&Faction> {
        self.categories
            .iter()
            .flat_map(|c| &c.factions)
            .find(|f| f.id == faction_id)
    }

    pub fn pending_paragon_rewards(&self) -> usize {
        self.categories
            .iter()
            .flat_map(|c| &c.factions)
            .filter(|f| f.paragon.as_ref().is_some_and(|p| p.reward_pending))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn faction(standing: Standing, current: u32) -> Faction {
        Faction {
            id: 1,
            name: "Test".into(),
            standing,
            current,
            max: standing.bracket_size(),
            paragon: None,
            at_war: false,
        }
    }

    // --- Standing ---

    #[test]
    fn standing_labels() {
        assert_eq!(Standing::Hated.label(), "Hated");
        assert_eq!(Standing::Exalted.label(), "Exalted");
        assert_eq!(Standing::Neutral.label(), "Neutral");
    }

    #[test]
    fn standing_thresholds_ascending() {
        for pair in Standing::ALL.windows(2) {
            assert!(
                pair[0].threshold() < pair[1].threshold(),
                "{:?} threshold {} not less than {:?} threshold {}",
                pair[0],
                pair[0].threshold(),
                pair[1],
                pair[1].threshold()
            );
        }
    }

    #[test]
    fn standing_bracket_sizes() {
        assert_eq!(Standing::Hated.bracket_size(), 36000);
        assert_eq!(Standing::Friendly.bracket_size(), 6000);
        assert_eq!(Standing::Exalted.bracket_size(), 0);
    }

    // --- Paragon ---

    #[test]
    fn paragon_fraction() {
        let p = ParagonProgress {
            current: 5000,
            max: 10000,
            ..Default::default()
        };
        assert!((p.fraction() - 0.5).abs() < 0.01);
    }

    #[test]
    fn paragon_fraction_zero_max() {
        let p = ParagonProgress::default();
        assert_eq!(p.fraction(), 0.0);
    }

    #[test]
    fn paragon_progress_text() {
        let p = ParagonProgress {
            current: 3000,
            max: 10000,
            ..Default::default()
        };
        assert_eq!(p.progress_text(), "3000/10000");
    }

    // --- Faction ---

    #[test]
    fn faction_progress_fraction() {
        let f = faction(Standing::Honored, 6000);
        assert!((f.progress_fraction() - 0.5).abs() < 0.01);
    }

    #[test]
    fn exalted_progress_is_full() {
        let f = faction(Standing::Exalted, 0);
        assert_eq!(f.progress_fraction(), 1.0);
    }

    #[test]
    fn faction_is_exalted() {
        assert!(faction(Standing::Exalted, 0).is_exalted());
        assert!(!faction(Standing::Revered, 0).is_exalted());
    }

    #[test]
    fn faction_has_paragon() {
        let mut f = faction(Standing::Exalted, 0);
        assert!(!f.has_paragon());
        f.paragon = Some(ParagonProgress {
            current: 1000,
            max: 10000,
            ..Default::default()
        });
        assert!(f.has_paragon());
    }

    // --- ReputationState ---

    #[test]
    fn state_faction_count() {
        let state = ReputationState {
            categories: vec![
                FactionCategory {
                    name: "A".into(),
                    factions: vec![
                        faction(Standing::Neutral, 0),
                        faction(Standing::Friendly, 0),
                    ],
                },
                FactionCategory {
                    name: "B".into(),
                    factions: vec![faction(Standing::Hated, 0)],
                },
            ],
        };
        assert_eq!(state.faction_count(), 3);
    }

    #[test]
    fn state_exalted_count() {
        let state = ReputationState {
            categories: vec![FactionCategory {
                name: "A".into(),
                factions: vec![
                    faction(Standing::Exalted, 0),
                    faction(Standing::Honored, 0),
                    faction(Standing::Exalted, 0),
                ],
            }],
        };
        assert_eq!(state.exalted_count(), 2);
    }

    #[test]
    fn state_find_faction() {
        let mut f = faction(Standing::Friendly, 3000);
        f.id = 42;
        f.name = "Stormwind".into();
        let state = ReputationState {
            categories: vec![FactionCategory {
                name: "A".into(),
                factions: vec![f],
            }],
        };
        let found = state.find_faction(42).expect("should find");
        assert_eq!(found.name, "Stormwind");
        assert!(state.find_faction(999).is_none());
    }

    #[test]
    fn state_pending_paragon_rewards() {
        let mut f1 = faction(Standing::Exalted, 0);
        f1.paragon = Some(ParagonProgress {
            reward_pending: true,
            ..Default::default()
        });
        let mut f2 = faction(Standing::Exalted, 0);
        f2.paragon = Some(ParagonProgress {
            reward_pending: false,
            ..Default::default()
        });
        let state = ReputationState {
            categories: vec![FactionCategory {
                name: "A".into(),
                factions: vec![f1, f2, faction(Standing::Honored, 0)],
            }],
        };
        assert_eq!(state.pending_paragon_rewards(), 1);
    }

    #[test]
    fn texture_fdids_are_nonzero() {
        assert_ne!(textures::REP_BAR, 0);
        assert_ne!(textures::REP_BAR_HIGHLIGHT, 0);
        assert_ne!(textures::REP_WATCH_BAR, 0);
        assert_ne!(textures::FACTION_ICON_ALLIANCE, 0);
        assert_ne!(textures::FACTION_ICON_HORDE, 0);
        assert_ne!(textures::FACTION_ICON_GENERIC, 0);
        assert_ne!(textures::FACTION_ICON_CHAMPIONS, 0);
        assert_ne!(textures::FACTION_ICON_TORTOLLAN, 0);
        assert_ne!(textures::PARAGON_REWARD_CHEST, 0);
        assert_ne!(textures::BAR_FILL, 0);
    }
}
