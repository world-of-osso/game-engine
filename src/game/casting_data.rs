use bevy::prelude::*;

/// Texture FDIDs for the casting bar.
pub mod textures {
    /// Bar fill texture.
    pub const BAR_FILL: u32 = 4505182;
    /// Border chrome (full size).
    pub const BORDER: u32 = 130874;
    /// Border chrome (small variant).
    pub const BORDER_SMALL: u32 = 130873;
    /// Spark at fill edge.
    pub const SPARK: u32 = 130877;
    /// Flash effect on cast complete.
    pub const FLASH: u32 = 130876;
}

/// Whether this is a cast or a channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CastType {
    #[default]
    Cast,
    Channel,
}

/// Runtime casting state for the local player.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct CastingState {
    pub active: Option<ActiveCast>,
}

/// An in-progress cast or channel.
#[derive(Clone, Debug, PartialEq)]
pub struct ActiveCast {
    pub spell_name: String,
    pub spell_id: u32,
    pub icon_fdid: u32,
    pub cast_type: CastType,
    pub interruptible: bool,
    /// Total cast/channel duration in seconds.
    pub duration: f32,
    /// Elapsed time in seconds.
    pub elapsed: f32,
}

impl ActiveCast {
    /// Progress fraction 0.0..=1.0.
    /// For casts: fills left-to-right (elapsed / duration).
    /// For channels: drains right-to-left (remaining / duration).
    pub fn progress(&self) -> f32 {
        if self.duration <= 0.0 {
            return 1.0;
        }
        let frac = self.elapsed / self.duration;
        match self.cast_type {
            CastType::Cast => frac.clamp(0.0, 1.0),
            CastType::Channel => (1.0 - frac).clamp(0.0, 1.0),
        }
    }

    /// Remaining time in seconds.
    pub fn remaining(&self) -> f32 {
        (self.duration - self.elapsed).max(0.0)
    }

    /// Timer display text (e.g. "1.5" or "0.3").
    pub fn timer_text(&self) -> String {
        let r = self.remaining();
        format!("{r:.1}")
    }

    pub fn is_finished(&self) -> bool {
        self.elapsed >= self.duration
    }
}

impl CastingState {
    /// Start a new cast.
    pub fn start(&mut self, cast: ActiveCast) {
        self.active = Some(cast);
    }

    /// Advance the active cast by `dt` seconds.
    pub fn tick(&mut self, dt: f32) {
        if let Some(cast) = &mut self.active {
            cast.elapsed = (cast.elapsed + dt).min(cast.duration);
        }
    }

    /// Cancel/interrupt the active cast.
    pub fn cancel(&mut self) {
        self.active = None;
    }

    /// Remove finished casts.
    pub fn clear_finished(&mut self) {
        if self.active.as_ref().is_some_and(|c| c.is_finished()) {
            self.active = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fireball_cast() -> ActiveCast {
        ActiveCast {
            spell_name: "Fireball".into(),
            spell_id: 133,
            icon_fdid: 135810,
            cast_type: CastType::Cast,
            interruptible: true,
            duration: 2.5,
            elapsed: 0.0,
        }
    }

    fn drain_life_channel() -> ActiveCast {
        ActiveCast {
            cast_type: CastType::Channel,
            spell_name: "Drain Life".into(),
            duration: 5.0,
            elapsed: 0.0,
            ..fireball_cast()
        }
    }

    #[test]
    fn cast_progress_fills_forward() {
        let mut cast = fireball_cast();
        assert!((cast.progress() - 0.0).abs() < 0.01);
        cast.elapsed = 1.25;
        assert!((cast.progress() - 0.5).abs() < 0.01);
        cast.elapsed = 2.5;
        assert!((cast.progress() - 1.0).abs() < 0.01);
    }

    #[test]
    fn channel_progress_drains_backward() {
        let mut ch = drain_life_channel();
        assert!((ch.progress() - 1.0).abs() < 0.01);
        ch.elapsed = 2.5;
        assert!((ch.progress() - 0.5).abs() < 0.01);
        ch.elapsed = 5.0;
        assert!((ch.progress() - 0.0).abs() < 0.01);
    }

    #[test]
    fn timer_text_format() {
        let cast = ActiveCast {
            elapsed: 1.0,
            ..fireball_cast()
        };
        assert_eq!(cast.timer_text(), "1.5");
    }

    #[test]
    fn tick_advances_elapsed() {
        let mut state = CastingState::default();
        state.start(fireball_cast());
        state.tick(1.0);
        let cast = state.active.as_ref().unwrap();
        assert!((cast.elapsed - 1.0).abs() < 0.01);
    }

    #[test]
    fn tick_clamps_at_duration() {
        let mut state = CastingState::default();
        state.start(fireball_cast());
        state.tick(10.0);
        let cast = state.active.as_ref().unwrap();
        assert!((cast.elapsed - 2.5).abs() < 0.01);
    }

    #[test]
    fn cancel_clears_cast() {
        let mut state = CastingState::default();
        state.start(fireball_cast());
        state.cancel();
        assert!(state.active.is_none());
    }

    #[test]
    fn clear_finished_removes_completed() {
        let mut state = CastingState::default();
        state.start(fireball_cast());
        state.tick(3.0);
        assert!(state.active.as_ref().unwrap().is_finished());
        state.clear_finished();
        assert!(state.active.is_none());
    }

    #[test]
    fn clear_finished_keeps_active() {
        let mut state = CastingState::default();
        state.start(fireball_cast());
        state.tick(1.0);
        state.clear_finished();
        assert!(state.active.is_some());
    }

    #[test]
    fn texture_fdids_are_nonzero() {
        assert_ne!(textures::BAR_FILL, 0);
        assert_ne!(textures::BORDER, 0);
        assert_ne!(textures::SPARK, 0);
        assert_ne!(textures::FLASH, 0);
    }

    // --- Cast progress interpolation ---

    #[test]
    fn progress_zero_duration_returns_one() {
        let cast = ActiveCast {
            duration: 0.0,
            ..fireball_cast()
        };
        assert_eq!(cast.progress(), 1.0);
    }

    #[test]
    fn progress_overcapped_elapsed_clamps() {
        let cast = ActiveCast {
            elapsed: 10.0,
            duration: 2.5,
            ..fireball_cast()
        };
        assert_eq!(cast.progress(), 1.0);
    }

    #[test]
    fn channel_progress_overcapped_clamps_to_zero() {
        let ch = ActiveCast {
            elapsed: 10.0,
            ..drain_life_channel()
        };
        assert_eq!(ch.progress(), 0.0);
    }

    #[test]
    fn remaining_at_start() {
        let cast = fireball_cast();
        assert!((cast.remaining() - 2.5).abs() < 0.01);
    }

    #[test]
    fn remaining_at_finish() {
        let cast = ActiveCast {
            elapsed: 2.5,
            ..fireball_cast()
        };
        assert_eq!(cast.remaining(), 0.0);
    }

    #[test]
    fn timer_text_at_zero() {
        let cast = ActiveCast {
            elapsed: 2.5,
            ..fireball_cast()
        };
        assert_eq!(cast.timer_text(), "0.0");
    }

    // --- Interrupt state ---

    #[test]
    fn interruptible_cast_flag() {
        let cast = fireball_cast();
        assert!(cast.interruptible);
    }

    #[test]
    fn uninterruptible_cast() {
        let cast = ActiveCast {
            spell_name: "Hearthstone".into(),
            interruptible: false,
            ..fireball_cast()
        };
        assert!(!cast.interruptible);
    }

    #[test]
    fn cancel_on_interruptible_channel() {
        let mut state = CastingState::default();
        state.start(drain_life_channel());
        state.tick(2.0);
        // Interrupt mid-channel
        let was_active = state.active.is_some();
        state.cancel();
        assert!(was_active);
        assert!(state.active.is_none());
    }

    #[test]
    fn tick_on_empty_state_no_panic() {
        let mut state = CastingState::default();
        state.tick(1.0); // should not panic
        assert!(state.active.is_none());
    }

    #[test]
    fn clear_finished_on_empty_state() {
        let mut state = CastingState::default();
        state.clear_finished(); // should not panic
        assert!(state.active.is_none());
    }
}
