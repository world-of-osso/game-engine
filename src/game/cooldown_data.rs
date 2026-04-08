//! Spell cooldown tracking synced from the server.
//!
//! Maintains a map of spell ID → cooldown state. The server sends cooldown
//! start events; the client ticks them down and marks spells as ready.

use bevy::prelude::*;

/// A single spell cooldown.
#[derive(Clone, Debug, PartialEq)]
pub struct SpellCooldown {
    pub spell_id: u32,
    /// Total cooldown duration in seconds.
    pub duration: f32,
    /// Remaining time in seconds.
    pub remaining: f32,
}

impl SpellCooldown {
    /// Progress fraction (0.0 = just started, 1.0 = ready).
    pub fn progress(&self) -> f32 {
        if self.duration <= 0.0 {
            return 1.0;
        }
        (1.0 - self.remaining / self.duration).clamp(0.0, 1.0)
    }

    /// Whether the cooldown has finished.
    pub fn is_ready(&self) -> bool {
        self.remaining <= 0.0
    }

    /// Formatted remaining time (e.g. "1:30", "45s", "Ready").
    pub fn display_text(&self) -> String {
        if self.is_ready() {
            return String::new();
        }
        let secs = self.remaining.ceil() as u32;
        if secs >= 60 {
            format!("{}:{:02}", secs / 60, secs % 60)
        } else {
            format!("{secs}s")
        }
    }
}

/// Centralized cooldown tracker for all spells.
#[derive(Resource, Default, Clone, Debug)]
pub struct CooldownTracker {
    cooldowns: Vec<SpellCooldown>,
}

impl CooldownTracker {
    /// Start or restart a cooldown for a spell (from server event).
    pub fn start_cooldown(&mut self, spell_id: u32, duration: f32) {
        if let Some(cd) = self.cooldowns.iter_mut().find(|c| c.spell_id == spell_id) {
            cd.duration = duration;
            cd.remaining = duration;
        } else {
            self.cooldowns.push(SpellCooldown {
                spell_id,
                duration,
                remaining: duration,
            });
        }
    }

    /// Query the cooldown state for a spell. Returns None if no cooldown active.
    pub fn get(&self, spell_id: u32) -> Option<&SpellCooldown> {
        self.cooldowns
            .iter()
            .find(|c| c.spell_id == spell_id && !c.is_ready())
    }

    /// Whether a spell is on cooldown.
    pub fn is_on_cooldown(&self, spell_id: u32) -> bool {
        self.get(spell_id).is_some()
    }

    /// Remaining seconds for a spell (0.0 if ready or no cooldown).
    pub fn remaining(&self, spell_id: u32) -> f32 {
        self.get(spell_id).map_or(0.0, |c| c.remaining)
    }

    /// Tick all cooldowns by dt seconds.
    pub fn tick(&mut self, dt: f32) {
        for cd in &mut self.cooldowns {
            if cd.remaining > 0.0 {
                cd.remaining = (cd.remaining - dt).max(0.0);
            }
        }
    }

    /// Remove finished cooldowns to keep the list clean.
    pub fn cleanup(&mut self) {
        self.cooldowns.retain(|c| !c.is_ready());
    }

    /// Reset a specific spell's cooldown (e.g. Cold Snap, Preparation).
    pub fn reset_cooldown(&mut self, spell_id: u32) {
        if let Some(cd) = self.cooldowns.iter_mut().find(|c| c.spell_id == spell_id) {
            cd.remaining = 0.0;
        }
    }

    /// Number of active (non-ready) cooldowns.
    pub fn active_count(&self) -> usize {
        self.cooldowns.iter().filter(|c| !c.is_ready()).count()
    }
}

/// Global cooldown state.
#[derive(Resource, Clone, Debug)]
pub struct GlobalCooldown {
    pub duration: f32,
    pub remaining: f32,
}

impl Default for GlobalCooldown {
    fn default() -> Self {
        Self {
            duration: 1.5,
            remaining: 0.0,
        }
    }
}

impl GlobalCooldown {
    pub fn trigger(&mut self) {
        self.remaining = self.duration;
    }

    pub fn is_active(&self) -> bool {
        self.remaining > 0.0
    }

    pub fn progress(&self) -> f32 {
        if self.duration <= 0.0 {
            return 1.0;
        }
        (1.0 - self.remaining / self.duration).clamp(0.0, 1.0)
    }

    pub fn tick(&mut self, dt: f32) {
        if self.remaining > 0.0 {
            self.remaining = (self.remaining - dt).max(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- SpellCooldown ---

    #[test]
    fn cooldown_progress() {
        let cd = SpellCooldown {
            spell_id: 1,
            duration: 10.0,
            remaining: 5.0,
        };
        assert!((cd.progress() - 0.5).abs() < 0.01);
    }

    #[test]
    fn cooldown_ready_when_zero() {
        let cd = SpellCooldown {
            spell_id: 1,
            duration: 10.0,
            remaining: 0.0,
        };
        assert!(cd.is_ready());
        assert_eq!(cd.progress(), 1.0);
    }

    #[test]
    fn cooldown_display_text() {
        let cd = SpellCooldown {
            spell_id: 1,
            duration: 120.0,
            remaining: 95.5,
        };
        assert_eq!(cd.display_text(), "1:36");

        let short = SpellCooldown {
            spell_id: 2,
            duration: 15.0,
            remaining: 8.3,
        };
        assert_eq!(short.display_text(), "9s");

        let ready = SpellCooldown {
            spell_id: 3,
            duration: 10.0,
            remaining: 0.0,
        };
        assert_eq!(ready.display_text(), "");
    }

    // --- CooldownTracker ---

    #[test]
    fn tracker_start_and_query() {
        let mut tracker = CooldownTracker::default();
        tracker.start_cooldown(853, 60.0);
        assert!(tracker.is_on_cooldown(853));
        assert!((tracker.remaining(853) - 60.0).abs() < 0.01);
        assert!(!tracker.is_on_cooldown(999));
    }

    #[test]
    fn tracker_tick_counts_down() {
        let mut tracker = CooldownTracker::default();
        tracker.start_cooldown(853, 10.0);
        tracker.tick(3.0);
        assert!((tracker.remaining(853) - 7.0).abs() < 0.01);
    }

    #[test]
    fn tracker_tick_expires() {
        let mut tracker = CooldownTracker::default();
        tracker.start_cooldown(853, 5.0);
        tracker.tick(10.0);
        assert!(!tracker.is_on_cooldown(853));
        assert_eq!(tracker.remaining(853), 0.0);
    }

    #[test]
    fn tracker_restart_cooldown() {
        let mut tracker = CooldownTracker::default();
        tracker.start_cooldown(853, 60.0);
        tracker.tick(30.0);
        tracker.start_cooldown(853, 60.0); // restart
        assert!((tracker.remaining(853) - 60.0).abs() < 0.01);
    }

    #[test]
    fn tracker_reset_cooldown() {
        let mut tracker = CooldownTracker::default();
        tracker.start_cooldown(853, 60.0);
        tracker.reset_cooldown(853);
        assert!(!tracker.is_on_cooldown(853));
    }

    #[test]
    fn tracker_cleanup_removes_ready() {
        let mut tracker = CooldownTracker::default();
        tracker.start_cooldown(1, 5.0);
        tracker.start_cooldown(2, 15.0);
        tracker.tick(10.0);
        assert_eq!(tracker.active_count(), 1);
        tracker.cleanup();
        assert_eq!(tracker.cooldowns.len(), 1);
        assert_eq!(tracker.cooldowns[0].spell_id, 2);
    }

    // --- GlobalCooldown ---

    #[test]
    fn gcd_default_is_1_5_seconds() {
        let gcd = GlobalCooldown::default();
        assert_eq!(gcd.duration, 1.5);
        assert!(!gcd.is_active());
    }

    #[test]
    fn gcd_trigger_and_tick() {
        let mut gcd = GlobalCooldown::default();
        gcd.trigger();
        assert!(gcd.is_active());
        assert!((gcd.remaining - 1.5).abs() < 0.01);
        gcd.tick(1.0);
        assert!(gcd.is_active());
        gcd.tick(1.0);
        assert!(!gcd.is_active());
    }

    #[test]
    fn gcd_progress() {
        let mut gcd = GlobalCooldown::default();
        gcd.trigger();
        assert!((gcd.progress() - 0.0).abs() < 0.01);
        gcd.tick(0.75);
        assert!((gcd.progress() - 0.5).abs() < 0.01);
    }
}
