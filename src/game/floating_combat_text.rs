//! Floating combat text (FCT) data model.
//!
//! Damage and heal numbers that appear above units and float upward,
//! fading out over their lifetime. Supports crit scaling, color coding
//! by damage type, and staggered spawn offsets to avoid overlap.

use bevy::prelude::*;

/// The kind of combat text, determines color and animation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatTextKind {
    PhysicalDamage,
    SpellDamage,
    Heal,
    CritDamage,
    CritHeal,
    Miss,
    Dodge,
    Parry,
    Block,
    Absorb,
}

impl CombatTextKind {
    /// RGBA color for this text kind.
    pub fn color(self) -> [f32; 4] {
        match self {
            Self::PhysicalDamage => [1.0, 1.0, 1.0, 1.0],
            Self::SpellDamage => [1.0, 1.0, 0.0, 1.0],
            Self::Heal => [0.0, 1.0, 0.0, 1.0],
            Self::CritDamage => [1.0, 0.0, 0.0, 1.0],
            Self::CritHeal => [0.0, 1.0, 0.5, 1.0],
            Self::Miss | Self::Dodge | Self::Parry => [0.7, 0.7, 0.7, 1.0],
            Self::Block | Self::Absorb => [0.8, 0.8, 1.0, 1.0],
        }
    }

    /// Whether this kind shows a number or a text label.
    pub fn is_label(self) -> bool {
        matches!(
            self,
            Self::Miss | Self::Dodge | Self::Parry | Self::Block | Self::Absorb
        )
    }

    /// Font scale multiplier (crits are bigger).
    pub fn font_scale(self) -> f32 {
        match self {
            Self::CritDamage | Self::CritHeal => 1.5,
            _ => 1.0,
        }
    }

    /// Display label for non-numeric kinds.
    pub fn label(self) -> &'static str {
        match self {
            Self::Miss => "Miss",
            Self::Dodge => "Dodge",
            Self::Parry => "Parry",
            Self::Block => "Block",
            Self::Absorb => "Absorb",
            _ => "",
        }
    }
}

const FCT_LIFETIME: f32 = 1.5;
const FCT_RISE_SPEED: f32 = 60.0;
const FCT_FADE_START: f32 = 0.7;

/// A single floating combat text instance.
#[derive(Clone, Debug)]
pub struct FloatingCombatText {
    pub kind: CombatTextKind,
    pub amount: u32,
    pub elapsed: f32,
    pub lifetime: f32,
    /// Horizontal offset to stagger overlapping texts.
    pub x_offset: f32,
}

impl FloatingCombatText {
    pub fn new(kind: CombatTextKind, amount: u32) -> Self {
        Self {
            kind,
            amount,
            elapsed: 0.0,
            lifetime: FCT_LIFETIME,
            x_offset: 0.0,
        }
    }

    /// Display text (number or label).
    pub fn display_text(&self) -> String {
        if self.kind.is_label() {
            self.kind.label().to_string()
        } else {
            self.amount.to_string()
        }
    }

    /// Vertical offset from spawn position (rises over time).
    pub fn y_offset(&self) -> f32 {
        self.elapsed * FCT_RISE_SPEED
    }

    /// Alpha value (fades out after FCT_FADE_START fraction of lifetime).
    pub fn alpha(&self) -> f32 {
        let fraction = self.elapsed / self.lifetime;
        if fraction < FCT_FADE_START {
            1.0
        } else {
            let fade_progress = (fraction - FCT_FADE_START) / (1.0 - FCT_FADE_START);
            (1.0 - fade_progress).clamp(0.0, 1.0)
        }
    }

    /// Whether this text has expired.
    pub fn is_expired(&self) -> bool {
        self.elapsed >= self.lifetime
    }

    /// Advance by dt seconds.
    pub fn tick(&mut self, dt: f32) {
        self.elapsed = (self.elapsed + dt).min(self.lifetime);
    }
}

/// Per-entity collection of active floating texts.
#[derive(Component, Default)]
pub struct FloatingCombatTextStack {
    pub texts: Vec<FloatingCombatText>,
}

impl FloatingCombatTextStack {
    /// Add a new combat text, staggering X offset to avoid overlap.
    pub fn push(&mut self, mut text: FloatingCombatText) {
        let active_count = self.texts.len() as f32;
        text.x_offset = (active_count % 3.0 - 1.0) * 15.0;
        self.texts.push(text);
    }

    /// Tick all texts and remove expired ones.
    pub fn tick(&mut self, dt: f32) {
        for text in &mut self.texts {
            text.tick(dt);
        }
        self.texts.retain(|t| !t.is_expired());
    }

    /// Number of active (non-expired) texts.
    pub fn active_count(&self) -> usize {
        self.texts.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- CombatTextKind ---

    #[test]
    fn damage_types_have_distinct_colors() {
        let phys = CombatTextKind::PhysicalDamage.color();
        let spell = CombatTextKind::SpellDamage.color();
        let heal = CombatTextKind::Heal.color();
        assert_ne!(phys, spell);
        assert_ne!(phys, heal);
        assert_ne!(spell, heal);
    }

    #[test]
    fn crit_has_larger_font_scale() {
        assert!(CombatTextKind::CritDamage.font_scale() > 1.0);
        assert!(CombatTextKind::CritHeal.font_scale() > 1.0);
        assert_eq!(CombatTextKind::PhysicalDamage.font_scale(), 1.0);
    }

    #[test]
    fn miss_dodge_parry_are_labels() {
        assert!(CombatTextKind::Miss.is_label());
        assert!(CombatTextKind::Dodge.is_label());
        assert!(CombatTextKind::Parry.is_label());
        assert!(!CombatTextKind::PhysicalDamage.is_label());
        assert!(!CombatTextKind::Heal.is_label());
    }

    #[test]
    fn label_text() {
        assert_eq!(CombatTextKind::Miss.label(), "Miss");
        assert_eq!(CombatTextKind::Dodge.label(), "Dodge");
        assert_eq!(CombatTextKind::PhysicalDamage.label(), "");
    }

    // --- FloatingCombatText ---

    #[test]
    fn display_text_number() {
        let t = FloatingCombatText::new(CombatTextKind::PhysicalDamage, 500);
        assert_eq!(t.display_text(), "500");
    }

    #[test]
    fn display_text_label() {
        let t = FloatingCombatText::new(CombatTextKind::Miss, 0);
        assert_eq!(t.display_text(), "Miss");
    }

    #[test]
    fn y_offset_increases_over_time() {
        let mut t = FloatingCombatText::new(CombatTextKind::Heal, 100);
        assert_eq!(t.y_offset(), 0.0);
        t.tick(0.5);
        assert!(t.y_offset() > 0.0);
    }

    #[test]
    fn alpha_full_at_start() {
        let t = FloatingCombatText::new(CombatTextKind::Heal, 100);
        assert_eq!(t.alpha(), 1.0);
    }

    #[test]
    fn alpha_fades_after_threshold() {
        let mut t = FloatingCombatText::new(CombatTextKind::Heal, 100);
        // At 70% of lifetime, fade starts
        t.elapsed = t.lifetime * 0.7;
        assert!((t.alpha() - 1.0).abs() < 0.01);
        // At 85% of lifetime, partially faded
        t.elapsed = t.lifetime * 0.85;
        assert!(t.alpha() < 1.0 && t.alpha() > 0.0);
        // At end
        t.elapsed = t.lifetime;
        assert!(t.alpha().abs() < 0.01);
    }

    #[test]
    fn expired_after_lifetime() {
        let mut t = FloatingCombatText::new(CombatTextKind::SpellDamage, 200);
        assert!(!t.is_expired());
        t.tick(FCT_LIFETIME + 1.0);
        assert!(t.is_expired());
    }

    #[test]
    fn tick_clamped_at_lifetime() {
        let mut t = FloatingCombatText::new(CombatTextKind::SpellDamage, 200);
        t.tick(999.0);
        assert!((t.elapsed - t.lifetime).abs() < 0.01);
    }

    // --- FloatingCombatTextStack ---

    #[test]
    fn stack_push_and_tick() {
        let mut stack = FloatingCombatTextStack::default();
        stack.push(FloatingCombatText::new(CombatTextKind::PhysicalDamage, 100));
        stack.push(FloatingCombatText::new(CombatTextKind::Heal, 50));
        assert_eq!(stack.active_count(), 2);
        stack.tick(FCT_LIFETIME + 0.1);
        assert_eq!(stack.active_count(), 0);
    }

    #[test]
    fn stack_stagger_x_offsets() {
        let mut stack = FloatingCombatTextStack::default();
        for i in 0..3 {
            stack.push(FloatingCombatText::new(
                CombatTextKind::SpellDamage,
                i * 100,
            ));
        }
        let offsets: Vec<f32> = stack.texts.iter().map(|t| t.x_offset).collect();
        // Offsets should differ: -15, 0, 15
        assert_ne!(offsets[0], offsets[1]);
        assert_ne!(offsets[1], offsets[2]);
    }

    #[test]
    fn stack_partial_expiry() {
        let mut stack = FloatingCombatTextStack::default();
        let mut old = FloatingCombatText::new(CombatTextKind::PhysicalDamage, 100);
        old.elapsed = FCT_LIFETIME - 0.1;
        stack.texts.push(old);
        stack.push(FloatingCombatText::new(CombatTextKind::Heal, 200));
        assert_eq!(stack.active_count(), 2);
        stack.tick(0.2); // only the first expires
        assert_eq!(stack.active_count(), 1);
        assert_eq!(stack.texts[0].amount, 200);
    }
}
