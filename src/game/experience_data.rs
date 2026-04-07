//! Experience bar and XP tracking data model.
//!
//! Tracks current XP, level, rested XP bonus, and provides progress
//! calculations for the experience bar UI.

use bevy::prelude::*;

/// Maximum player level.
pub const MAX_LEVEL: u32 = 80;

/// XP required to advance from a given level to the next.
/// Simplified curve: base 400 + 100 per level, scaling up.
pub fn xp_to_next_level(level: u32) -> u64 {
    if level >= MAX_LEVEL {
        return 0;
    }
    (400 + level as u64 * 100) * (1 + level as u64 / 10)
}

/// Runtime experience state for the local player.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct ExperienceState {
    pub level: u32,
    pub current_xp: u64,
    pub rested_xp: u64,
}

impl Default for ExperienceState {
    fn default() -> Self {
        Self {
            level: 1,
            current_xp: 0,
            rested_xp: 0,
        }
    }
}

impl ExperienceState {
    /// XP needed to reach the next level from the current level.
    pub fn xp_required(&self) -> u64 {
        xp_to_next_level(self.level)
    }

    /// Progress fraction (0.0–1.0) toward the next level.
    pub fn progress(&self) -> f32 {
        let required = self.xp_required();
        if required == 0 {
            return 1.0;
        }
        (self.current_xp as f32 / required as f32).clamp(0.0, 1.0)
    }

    /// Whether the player is at max level.
    pub fn is_max_level(&self) -> bool {
        self.level >= MAX_LEVEL
    }

    /// Display text for the XP bar (e.g. "1200 / 5000").
    pub fn bar_text(&self) -> String {
        if self.is_max_level() {
            "Max Level".into()
        } else {
            format!("{} / {}", self.current_xp, self.xp_required())
        }
    }

    /// Rested XP progress as a fraction of the current level's requirement.
    pub fn rested_progress(&self) -> f32 {
        let required = self.xp_required();
        if required == 0 {
            return 0.0;
        }
        let rested_end = (self.current_xp + self.rested_xp).min(required);
        let rested_start = self.current_xp;
        if rested_end <= rested_start {
            return 0.0;
        }
        (rested_end - rested_start) as f32 / required as f32
    }

    /// Add XP, handling level-ups. Returns the number of levels gained.
    pub fn add_xp(&mut self, mut amount: u64) -> u32 {
        let mut levels_gained = 0;
        while amount > 0 && !self.is_max_level() {
            let needed = self.xp_required() - self.current_xp;
            if amount >= needed {
                amount -= needed;
                self.current_xp = 0;
                self.level += 1;
                levels_gained += 1;
            } else {
                self.current_xp += amount;
                amount = 0;
            }
        }
        // Consume rested XP (grants bonus equal to XP earned, up to rested pool)
        levels_gained
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state() {
        let state = ExperienceState::default();
        assert_eq!(state.level, 1);
        assert_eq!(state.current_xp, 0);
        assert_eq!(state.rested_xp, 0);
    }

    #[test]
    fn xp_required_increases_with_level() {
        let low = xp_to_next_level(1);
        let mid = xp_to_next_level(20);
        let high = xp_to_next_level(60);
        assert!(mid > low);
        assert!(high > mid);
    }

    #[test]
    fn xp_required_zero_at_max() {
        assert_eq!(xp_to_next_level(MAX_LEVEL), 0);
        assert_eq!(xp_to_next_level(MAX_LEVEL + 1), 0);
    }

    #[test]
    fn progress_at_zero() {
        let state = ExperienceState::default();
        assert_eq!(state.progress(), 0.0);
    }

    #[test]
    fn progress_midway() {
        let required = xp_to_next_level(1);
        let state = ExperienceState {
            level: 1,
            current_xp: required / 2,
            rested_xp: 0,
        };
        assert!((state.progress() - 0.5).abs() < 0.01);
    }

    #[test]
    fn progress_at_max_level() {
        let state = ExperienceState {
            level: MAX_LEVEL,
            current_xp: 0,
            rested_xp: 0,
        };
        assert_eq!(state.progress(), 1.0);
        assert!(state.is_max_level());
    }

    #[test]
    fn bar_text_normal() {
        let state = ExperienceState {
            level: 5,
            current_xp: 300,
            rested_xp: 0,
        };
        let required = xp_to_next_level(5);
        assert_eq!(state.bar_text(), format!("300 / {required}"));
    }

    #[test]
    fn bar_text_max_level() {
        let state = ExperienceState {
            level: MAX_LEVEL,
            current_xp: 0,
            rested_xp: 0,
        };
        assert_eq!(state.bar_text(), "Max Level");
    }

    #[test]
    fn add_xp_partial() {
        let mut state = ExperienceState::default();
        let gained = state.add_xp(100);
        assert_eq!(gained, 0);
        assert_eq!(state.current_xp, 100);
        assert_eq!(state.level, 1);
    }

    #[test]
    fn add_xp_level_up() {
        let mut state = ExperienceState::default();
        let required = xp_to_next_level(1);
        let gained = state.add_xp(required);
        assert_eq!(gained, 1);
        assert_eq!(state.level, 2);
        assert_eq!(state.current_xp, 0);
    }

    #[test]
    fn add_xp_multiple_level_ups() {
        let mut state = ExperienceState::default();
        let xp1 = xp_to_next_level(1);
        let xp2 = xp_to_next_level(2);
        let gained = state.add_xp(xp1 + xp2 + 50);
        assert_eq!(gained, 2);
        assert_eq!(state.level, 3);
        assert_eq!(state.current_xp, 50);
    }

    #[test]
    fn add_xp_caps_at_max_level() {
        let mut state = ExperienceState {
            level: MAX_LEVEL - 1,
            current_xp: 0,
            rested_xp: 0,
        };
        let gained = state.add_xp(u64::MAX / 2);
        assert_eq!(gained, 1);
        assert_eq!(state.level, MAX_LEVEL);
    }

    #[test]
    fn rested_progress_with_bonus() {
        let required = xp_to_next_level(5);
        let state = ExperienceState {
            level: 5,
            current_xp: required / 4,
            rested_xp: required / 4,
        };
        let rested = state.rested_progress();
        assert!((rested - 0.25).abs() < 0.01);
    }

    #[test]
    fn rested_progress_zero_when_no_rested() {
        let state = ExperienceState::default();
        assert_eq!(state.rested_progress(), 0.0);
    }

    #[test]
    fn rested_progress_capped_at_level_boundary() {
        let required = xp_to_next_level(5);
        let state = ExperienceState {
            level: 5,
            current_xp: required - 100,
            rested_xp: 500, // more rested than remaining XP
        };
        let rested = state.rested_progress();
        // Should only show the 100 XP gap, not the full 500
        let expected = 100.0 / required as f32;
        assert!((rested - expected).abs() < 0.01);
    }
}
