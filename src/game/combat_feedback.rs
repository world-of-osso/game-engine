//! Client-side combat feedback effects.
//!
//! Hit flash (red overlay on damage), screen shake on heavy hits,
//! and integration with floating combat text. These are purely visual
//! effects driven by incoming combat events.

use bevy::prelude::*;

/// Camera screen shake state.
#[derive(Resource, Debug, Clone)]
pub struct ScreenShake {
    /// Remaining shake duration in seconds.
    pub remaining: f32,
    /// Shake intensity (max pixel offset).
    pub intensity: f32,
    /// Shake frequency (oscillations per second).
    pub frequency: f32,
    /// Elapsed time (for oscillation phase).
    pub elapsed: f32,
}

impl Default for ScreenShake {
    fn default() -> Self {
        Self {
            remaining: 0.0,
            intensity: 0.0,
            frequency: 20.0,
            elapsed: 0.0,
        }
    }
}

impl ScreenShake {
    /// Trigger a new shake. If already shaking, takes the stronger of the two.
    pub fn trigger(&mut self, intensity: f32, duration: f32) {
        if intensity > self.intensity || self.remaining <= 0.0 {
            self.intensity = intensity;
            self.remaining = duration;
            self.elapsed = 0.0;
        }
    }

    /// Whether a shake is currently active.
    pub fn is_active(&self) -> bool {
        self.remaining > 0.0
    }

    /// Current camera offset for this frame. Decays over the duration.
    pub fn offset(&self) -> (f32, f32) {
        if !self.is_active() {
            return (0.0, 0.0);
        }
        let decay = self.remaining / (self.remaining + self.elapsed).max(0.001);
        let phase = self.elapsed * self.frequency * std::f32::consts::TAU;
        let x = phase.sin() * self.intensity * decay;
        let y = (phase * 1.3).cos() * self.intensity * decay * 0.7;
        (x, y)
    }

    /// Advance the shake timer.
    pub fn tick(&mut self, dt: f32) {
        if self.remaining > 0.0 {
            self.elapsed += dt;
            self.remaining = (self.remaining - dt).max(0.0);
        }
    }
}

/// Hit flash overlay state (red tint on damage taken).
#[derive(Resource, Debug, Clone)]
pub struct HitFlash {
    /// Remaining flash duration.
    pub remaining: f32,
    /// Total flash duration (for alpha calculation).
    pub duration: f32,
    /// Flash color.
    pub color: [f32; 4],
}

impl Default for HitFlash {
    fn default() -> Self {
        Self {
            remaining: 0.0,
            duration: 0.0,
            color: [1.0, 0.0, 0.0, 0.3],
        }
    }
}

impl HitFlash {
    /// Trigger a hit flash.
    pub fn trigger(&mut self, duration: f32) {
        self.remaining = duration;
        self.duration = duration;
    }

    /// Whether the flash is active.
    pub fn is_active(&self) -> bool {
        self.remaining > 0.0
    }

    /// Current alpha (fades out linearly).
    pub fn alpha(&self) -> f32 {
        if self.duration <= 0.0 || !self.is_active() {
            return 0.0;
        }
        (self.remaining / self.duration) * self.color[3]
    }

    /// Advance the flash timer.
    pub fn tick(&mut self, dt: f32) {
        if self.remaining > 0.0 {
            self.remaining = (self.remaining - dt).max(0.0);
        }
    }
}

/// Recommended shake intensity based on damage relative to max health.
pub fn shake_intensity_for_damage(damage: f32, max_health: f32) -> f32 {
    if max_health <= 0.0 {
        return 0.0;
    }
    let fraction = (damage / max_health).clamp(0.0, 1.0);
    // Light shake at 10%+ HP, heavy shake at 30%+
    if fraction < 0.1 {
        0.0
    } else if fraction < 0.3 {
        2.0
    } else {
        5.0
    }
}

/// Recommended shake duration based on damage fraction.
pub fn shake_duration_for_damage(damage: f32, max_health: f32) -> f32 {
    if max_health <= 0.0 {
        return 0.0;
    }
    let fraction = (damage / max_health).clamp(0.0, 1.0);
    (0.1 + fraction * 0.3).min(0.4)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- ScreenShake ---

    #[test]
    fn shake_inactive_by_default() {
        let shake = ScreenShake::default();
        assert!(!shake.is_active());
        assert_eq!(shake.offset(), (0.0, 0.0));
    }

    #[test]
    fn shake_trigger_activates() {
        let mut shake = ScreenShake::default();
        shake.trigger(5.0, 0.3);
        assert!(shake.is_active());
        assert_eq!(shake.intensity, 5.0);
    }

    #[test]
    fn shake_stronger_replaces_weaker() {
        let mut shake = ScreenShake::default();
        shake.trigger(2.0, 0.3);
        shake.trigger(5.0, 0.5);
        assert_eq!(shake.intensity, 5.0);
        assert!((shake.remaining - 0.5).abs() < 0.01);
    }

    #[test]
    fn shake_weaker_does_not_replace() {
        let mut shake = ScreenShake::default();
        shake.trigger(5.0, 0.3);
        shake.trigger(2.0, 0.5);
        assert_eq!(shake.intensity, 5.0);
    }

    #[test]
    fn shake_offset_nonzero_when_active() {
        let mut shake = ScreenShake::default();
        shake.trigger(5.0, 0.3);
        shake.tick(0.05);
        let (x, y) = shake.offset();
        assert!(x.abs() + y.abs() > 0.0, "offset should be nonzero");
    }

    #[test]
    fn shake_expires_after_duration() {
        let mut shake = ScreenShake::default();
        shake.trigger(5.0, 0.3);
        shake.tick(0.5);
        assert!(!shake.is_active());
        assert_eq!(shake.offset(), (0.0, 0.0));
    }

    // --- HitFlash ---

    #[test]
    fn flash_inactive_by_default() {
        let flash = HitFlash::default();
        assert!(!flash.is_active());
        assert_eq!(flash.alpha(), 0.0);
    }

    #[test]
    fn flash_trigger_activates() {
        let mut flash = HitFlash::default();
        flash.trigger(0.2);
        assert!(flash.is_active());
        assert!((flash.alpha() - 0.3).abs() < 0.01); // full alpha
    }

    #[test]
    fn flash_alpha_fades() {
        let mut flash = HitFlash::default();
        flash.trigger(1.0);
        flash.tick(0.5);
        assert!(flash.alpha() < 0.3);
        assert!(flash.alpha() > 0.0);
    }

    #[test]
    fn flash_expires() {
        let mut flash = HitFlash::default();
        flash.trigger(0.2);
        flash.tick(0.3);
        assert!(!flash.is_active());
        assert_eq!(flash.alpha(), 0.0);
    }

    // --- Damage-based intensity ---

    #[test]
    fn no_shake_for_small_damage() {
        assert_eq!(shake_intensity_for_damage(5.0, 100.0), 0.0);
    }

    #[test]
    fn light_shake_for_medium_damage() {
        assert_eq!(shake_intensity_for_damage(15.0, 100.0), 2.0);
    }

    #[test]
    fn heavy_shake_for_large_damage() {
        assert_eq!(shake_intensity_for_damage(40.0, 100.0), 5.0);
    }

    #[test]
    fn shake_intensity_zero_health() {
        assert_eq!(shake_intensity_for_damage(10.0, 0.0), 0.0);
    }

    #[test]
    fn shake_duration_scales_with_damage() {
        let low = shake_duration_for_damage(10.0, 100.0);
        let high = shake_duration_for_damage(50.0, 100.0);
        assert!(high > low);
    }

    #[test]
    fn shake_duration_capped() {
        let d = shake_duration_for_damage(100.0, 100.0);
        assert!(d <= 0.4);
    }
}
