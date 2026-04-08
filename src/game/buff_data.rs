use bevy::prelude::*;

/// Sample spell icon texture FDIDs for buffs and debuffs.
pub mod textures {
    // Buff icons
    /// Power Word: Fortitude.
    pub const FORTITUDE: u32 = 135987;
    /// Power Word: Shield.
    pub const PW_SHIELD: u32 = 135940;
    /// Mark of the Wild / Regeneration.
    pub const MARK_OF_WILD: u32 = 136078;
    /// Blessing of Protection.
    pub const BLESSING_PROTECTION: u32 = 135880;
    // Debuff icons
    /// Shadow Word: Pain.
    pub const SHADOW_WORD_PAIN: u32 = 136207;
    /// Slow (nature).
    pub const SLOW: u32 = 136091;
    /// Nullify Poison.
    pub const NULLIFY_POISON: u32 = 136067;
    /// Remove Disease.
    pub const REMOVE_DISEASE: u32 = 136083;
    /// Anti-Shadow (generic magic debuff).
    pub const ANTI_SHADOW: u32 = 136121;
}

/// Debuff dispel type, determines border color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DebuffType {
    #[default]
    None,
    Magic,
    Curse,
    Disease,
    Poison,
}

impl DebuffType {
    /// RGBA border color for this debuff type.
    pub fn border_color(self) -> &'static str {
        match self {
            Self::None => "0.5,0.0,0.0,1.0",
            Self::Magic => "0.2,0.6,1.0,1.0",
            Self::Curse => "0.6,0.0,1.0,1.0",
            Self::Disease => "0.6,0.4,0.0,1.0",
            Self::Poison => "0.0,0.6,0.0,1.0",
        }
    }
}

/// A single active buff or debuff.
#[derive(Clone, Debug, PartialEq)]
pub struct AuraInstance {
    pub spell_id: u32,
    pub name: String,
    pub description: String,
    pub icon_fdid: u32,
    pub source: String,
    /// Total duration in seconds (0 = permanent).
    pub duration: f32,
    /// Remaining time in seconds.
    pub remaining: f32,
    pub stacks: u32,
    pub is_debuff: bool,
    pub debuff_type: DebuffType,
}

impl AuraInstance {
    pub fn is_permanent(&self) -> bool {
        self.duration <= 0.0
    }

    /// Timer display text (e.g. "5m", "30s", "" for permanent).
    pub fn timer_text(&self) -> String {
        if self.is_permanent() {
            return String::new();
        }
        let secs = self.remaining.ceil() as u32;
        if secs >= 3600 {
            format!("{}h", secs / 3600)
        } else if secs >= 60 {
            format!("{}m", secs / 60)
        } else {
            format!("{secs}s")
        }
    }
}

/// Runtime aura state for the local player.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct AuraState {
    pub auras: Vec<AuraInstance>,
}

/// Optional aura state attached to any world entity for UI consumers like target frames.
#[derive(Component, Clone, Debug, PartialEq, Default)]
pub struct UnitAuraState {
    pub auras: Vec<AuraInstance>,
}

impl AuraState {
    pub fn buffs(&self) -> impl Iterator<Item = &AuraInstance> {
        self.auras.iter().filter(|a| !a.is_debuff)
    }

    pub fn debuffs(&self) -> impl Iterator<Item = &AuraInstance> {
        self.auras.iter().filter(|a| a.is_debuff)
    }

    pub fn tick(&mut self, dt: f32) {
        for aura in &mut self.auras {
            if aura.remaining > 0.0 {
                aura.remaining = (aura.remaining - dt).max(0.0);
            }
        }
    }

    /// Remove expired non-permanent auras.
    pub fn remove_expired(&mut self) {
        self.auras.retain(|a| a.is_permanent() || a.remaining > 0.0);
    }
}

impl UnitAuraState {
    pub fn buffs(&self) -> impl Iterator<Item = &AuraInstance> {
        self.auras.iter().filter(|a| !a.is_debuff)
    }

    pub fn debuffs(&self) -> impl Iterator<Item = &AuraInstance> {
        self.auras.iter().filter(|a| a.is_debuff)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_buff(name: &str, duration: f32, remaining: f32) -> AuraInstance {
        AuraInstance {
            spell_id: 1,
            name: name.into(),
            description: String::new(),
            icon_fdid: 12345,
            source: "Player".into(),
            duration,
            remaining,
            stacks: 1,
            is_debuff: false,
            debuff_type: DebuffType::None,
        }
    }

    fn make_debuff(name: &str, debuff_type: DebuffType) -> AuraInstance {
        AuraInstance {
            is_debuff: true,
            debuff_type,
            ..make_buff(name, 30.0, 30.0)
        }
    }

    #[test]
    fn timer_text_formats() {
        let perm = make_buff("Perm", 0.0, 0.0);
        assert_eq!(perm.timer_text(), "");

        let secs = make_buff("Short", 10.0, 5.3);
        assert_eq!(secs.timer_text(), "6s");

        let mins = make_buff("Med", 300.0, 125.0);
        assert_eq!(mins.timer_text(), "2m");

        let hours = make_buff("Long", 7200.0, 3700.0);
        assert_eq!(hours.timer_text(), "1h");
    }

    #[test]
    fn buffs_and_debuffs_filtered() {
        let mut state = AuraState::default();
        state.auras.push(make_buff("Fort", 3600.0, 3600.0));
        state
            .auras
            .push(make_debuff("Curse of Agony", DebuffType::Curse));
        state.auras.push(make_buff("MotW", 3600.0, 3600.0));

        assert_eq!(state.buffs().count(), 2);
        assert_eq!(state.debuffs().count(), 1);
    }

    #[test]
    fn tick_decrements_remaining() {
        let mut state = AuraState::default();
        state.auras.push(make_buff("Test", 10.0, 5.0));
        state.tick(2.0);
        assert!((state.auras[0].remaining - 3.0).abs() < 0.01);
    }

    #[test]
    fn tick_clamps_at_zero() {
        let mut state = AuraState::default();
        state.auras.push(make_buff("Test", 10.0, 1.0));
        state.tick(5.0);
        assert_eq!(state.auras[0].remaining, 0.0);
    }

    #[test]
    fn remove_expired_keeps_permanent() {
        let mut state = AuraState::default();
        state.auras.push(make_buff("Perm", 0.0, 0.0));
        state.auras.push(make_buff("Expired", 10.0, 0.0));
        state.auras.push(make_buff("Active", 10.0, 5.0));
        state.remove_expired();
        assert_eq!(state.auras.len(), 2);
        assert_eq!(state.auras[0].name, "Perm");
        assert_eq!(state.auras[1].name, "Active");
    }

    #[test]
    fn debuff_type_border_colors_are_distinct() {
        let colors: Vec<&str> = [
            DebuffType::None,
            DebuffType::Magic,
            DebuffType::Curse,
            DebuffType::Disease,
            DebuffType::Poison,
        ]
        .iter()
        .map(|t| t.border_color())
        .collect();
        for (i, a) in colors.iter().enumerate() {
            for (j, b) in colors.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "types {i} and {j} should have different colors");
                }
            }
        }
    }

    #[test]
    fn texture_fdids_are_nonzero() {
        assert_ne!(textures::FORTITUDE, 0);
        assert_ne!(textures::PW_SHIELD, 0);
        assert_ne!(textures::SHADOW_WORD_PAIN, 0);
        assert_ne!(textures::NULLIFY_POISON, 0);
        assert_ne!(textures::ANTI_SHADOW, 0);
    }

    // --- Stack count tests ---

    #[test]
    fn stack_count_preserved_through_tick() {
        let mut state = AuraState::default();
        let mut aura = make_buff("Lifebloom", 10.0, 10.0);
        aura.stacks = 3;
        state.auras.push(aura);
        state.tick(5.0);
        assert_eq!(state.auras[0].stacks, 3);
        assert!((state.auras[0].remaining - 5.0).abs() < 0.01);
    }

    #[test]
    fn zero_stacks_treated_as_no_display() {
        let aura = make_buff("Fort", 3600.0, 3600.0);
        // Default stack count is 1, which means "don't show count"
        assert_eq!(aura.stacks, 1);
    }

    #[test]
    fn high_stack_count() {
        let mut aura = make_buff("Sunder Armor", 30.0, 30.0);
        aura.stacks = 5;
        aura.is_debuff = true;
        aura.debuff_type = DebuffType::None;
        assert_eq!(aura.stacks, 5);
        assert!(aura.is_debuff);
    }

    // --- Debuff classification ---

    #[test]
    fn debuff_type_classification() {
        let magic = make_debuff("Polymorph", DebuffType::Magic);
        assert_eq!(magic.debuff_type, DebuffType::Magic);
        assert!(magic.is_debuff);

        let poison = make_debuff("Deadly Poison", DebuffType::Poison);
        assert_eq!(poison.debuff_type, DebuffType::Poison);

        let disease = make_debuff("Plague", DebuffType::Disease);
        assert_eq!(disease.debuff_type, DebuffType::Disease);

        let curse = make_debuff("Curse of Weakness", DebuffType::Curse);
        assert_eq!(curse.debuff_type, DebuffType::Curse);
    }

    // --- Duration countdown edge cases ---

    #[test]
    fn tick_permanent_aura_stays_at_zero() {
        let mut state = AuraState::default();
        state.auras.push(make_buff("Perm", 0.0, 0.0));
        state.tick(100.0);
        assert_eq!(state.auras[0].remaining, 0.0);
    }

    #[test]
    fn timer_text_boundary_at_60_seconds() {
        let at_59 = make_buff("A", 60.0, 59.5);
        assert_eq!(at_59.timer_text(), "1m"); // ceil(59.5) = 60s = 1m

        let at_60 = make_buff("B", 120.0, 60.0);
        assert_eq!(at_60.timer_text(), "1m");
    }

    #[test]
    fn timer_text_boundary_at_3600_seconds() {
        let at_3599 = make_buff("A", 7200.0, 3599.5);
        assert_eq!(at_3599.timer_text(), "1h"); // ceil(3599.5) = 3600s = 1h

        let at_3600 = make_buff("B", 7200.0, 3600.0);
        assert_eq!(at_3600.timer_text(), "1h");
    }

    #[test]
    fn remove_expired_with_mixed_debuffs() {
        let mut state = AuraState::default();
        state.auras.push(make_debuff("Active", DebuffType::Magic));
        let mut expired = make_debuff("Expired", DebuffType::Curse);
        expired.remaining = 0.0;
        state.auras.push(expired);
        state.remove_expired();
        assert_eq!(state.auras.len(), 1);
        assert_eq!(state.auras[0].name, "Active");
    }
}
