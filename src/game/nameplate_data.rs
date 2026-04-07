//! Nameplate visual data model.
//!
//! Describes the visual elements of a WoW-style nameplate: backdrop, border,
//! health bar, cast bar, name text, and level indicator. Colors are driven
//! by unit reaction (hostile/friendly/neutral) and class.

use bevy::prelude::*;

/// Unit reaction toward the local player, determines nameplate color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum UnitReaction {
    Hostile,
    #[default]
    Neutral,
    Friendly,
}

impl UnitReaction {
    /// Health bar color for this reaction.
    pub fn health_bar_color(self) -> [f32; 4] {
        match self {
            Self::Hostile => [0.8, 0.0, 0.0, 1.0],
            Self::Neutral => [0.9, 0.9, 0.0, 1.0],
            Self::Friendly => [0.0, 0.8, 0.0, 1.0],
        }
    }

    /// Name text color for this reaction.
    pub fn name_color(self) -> [f32; 4] {
        match self {
            Self::Hostile => [1.0, 0.2, 0.2, 1.0],
            Self::Neutral => [1.0, 1.0, 0.0, 1.0],
            Self::Friendly => [0.2, 1.0, 0.2, 1.0],
        }
    }
}

/// WoW class IDs for class-colored player nameplates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassColor {
    Warrior,
    Paladin,
    Hunter,
    Rogue,
    Priest,
    DeathKnight,
    Shaman,
    Mage,
    Warlock,
    Monk,
    Druid,
    DemonHunter,
    Evoker,
}

impl ClassColor {
    /// RGB class color (WoW standard).
    pub fn rgb(self) -> [f32; 3] {
        match self {
            Self::Warrior => [0.78, 0.61, 0.43],
            Self::Paladin => [0.96, 0.55, 0.73],
            Self::Hunter => [0.67, 0.83, 0.45],
            Self::Rogue => [1.00, 0.96, 0.41],
            Self::Priest => [1.00, 1.00, 1.00],
            Self::DeathKnight => [0.77, 0.12, 0.23],
            Self::Shaman => [0.00, 0.44, 0.87],
            Self::Mage => [0.25, 0.78, 0.92],
            Self::Warlock => [0.53, 0.53, 0.93],
            Self::Monk => [0.00, 1.00, 0.60],
            Self::Druid => [1.00, 0.49, 0.04],
            Self::DemonHunter => [0.64, 0.19, 0.79],
            Self::Evoker => [0.20, 0.58, 0.50],
        }
    }

    /// From WoW class ID (1-based).
    pub fn from_class_id(id: u8) -> Option<Self> {
        match id {
            1 => Some(Self::Warrior),
            2 => Some(Self::Paladin),
            3 => Some(Self::Hunter),
            4 => Some(Self::Rogue),
            5 => Some(Self::Priest),
            6 => Some(Self::DeathKnight),
            7 => Some(Self::Shaman),
            8 => Some(Self::Mage),
            9 => Some(Self::Warlock),
            10 => Some(Self::Monk),
            11 => Some(Self::Druid),
            12 => Some(Self::DemonHunter),
            13 => Some(Self::Evoker),
            _ => None,
        }
    }
}

/// Visual state of a nameplate.
#[derive(Clone, Debug, PartialEq)]
pub struct NameplateVisuals {
    pub name: String,
    pub level: u32,
    pub reaction: UnitReaction,
    pub class_color: Option<ClassColor>,
    pub health_current: f32,
    pub health_max: f32,
    /// Active cast bar (None = not casting).
    pub cast: Option<NameplateCastBar>,
    /// Whether this unit has threat on the player.
    pub has_threat: bool,
}

impl Default for NameplateVisuals {
    fn default() -> Self {
        Self {
            name: String::new(),
            level: 1,
            reaction: UnitReaction::Neutral,
            class_color: None,
            health_current: 1.0,
            health_max: 1.0,
            cast: None,
            has_threat: false,
        }
    }
}

impl NameplateVisuals {
    /// Health bar fill fraction (0.0–1.0).
    pub fn health_fraction(&self) -> f32 {
        if self.health_max <= 0.0 {
            return 0.0;
        }
        (self.health_current / self.health_max).clamp(0.0, 1.0)
    }

    /// Health bar color — class color for friendly players, reaction color otherwise.
    pub fn health_bar_color(&self) -> [f32; 4] {
        if self.reaction == UnitReaction::Friendly
            && let Some(class) = self.class_color
        {
            let [r, g, b] = class.rgb();
            return [r, g, b, 1.0];
        }
        self.reaction.health_bar_color()
    }

    /// Name text color — class color for players, reaction color for NPCs.
    pub fn name_text_color(&self) -> [f32; 4] {
        if let Some(class) = self.class_color {
            let [r, g, b] = class.rgb();
            return [r, g, b, 1.0];
        }
        self.reaction.name_color()
    }
}

/// Cast bar state within a nameplate.
#[derive(Clone, Debug, PartialEq)]
pub struct NameplateCastBar {
    pub spell_name: String,
    pub progress: f32,
    pub interruptible: bool,
}

impl NameplateCastBar {
    /// Cast bar fill color — gray if uninterruptible, orange otherwise.
    pub fn fill_color(&self) -> [f32; 4] {
        if self.interruptible {
            [1.0, 0.7, 0.0, 1.0]
        } else {
            [0.6, 0.6, 0.6, 1.0]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- UnitReaction ---

    #[test]
    fn reaction_colors_distinct() {
        assert_ne!(
            UnitReaction::Hostile.health_bar_color(),
            UnitReaction::Friendly.health_bar_color()
        );
        assert_ne!(
            UnitReaction::Hostile.health_bar_color(),
            UnitReaction::Neutral.health_bar_color()
        );
    }

    #[test]
    fn reaction_name_colors_distinct() {
        assert_ne!(
            UnitReaction::Hostile.name_color(),
            UnitReaction::Friendly.name_color()
        );
    }

    // --- ClassColor ---

    #[test]
    fn class_from_id_valid() {
        assert_eq!(ClassColor::from_class_id(1), Some(ClassColor::Warrior));
        assert_eq!(ClassColor::from_class_id(2), Some(ClassColor::Paladin));
        assert_eq!(ClassColor::from_class_id(13), Some(ClassColor::Evoker));
    }

    #[test]
    fn class_from_id_invalid() {
        assert!(ClassColor::from_class_id(0).is_none());
        assert!(ClassColor::from_class_id(14).is_none());
        assert!(ClassColor::from_class_id(255).is_none());
    }

    #[test]
    fn all_class_colors_unique() {
        let classes = [
            ClassColor::Warrior,
            ClassColor::Paladin,
            ClassColor::Hunter,
            ClassColor::Rogue,
            ClassColor::Priest,
            ClassColor::DeathKnight,
            ClassColor::Shaman,
            ClassColor::Mage,
            ClassColor::Warlock,
            ClassColor::Monk,
            ClassColor::Druid,
            ClassColor::DemonHunter,
            ClassColor::Evoker,
        ];
        for (i, a) in classes.iter().enumerate() {
            for (j, b) in classes.iter().enumerate() {
                if i != j {
                    assert_ne!(a.rgb(), b.rgb(), "{a:?} and {b:?} have same color");
                }
            }
        }
    }

    // --- NameplateVisuals ---

    #[test]
    fn health_fraction() {
        let np = NameplateVisuals {
            health_current: 50.0,
            health_max: 100.0,
            ..Default::default()
        };
        assert!((np.health_fraction() - 0.5).abs() < 0.01);
    }

    #[test]
    fn health_fraction_zero_max() {
        let np = NameplateVisuals {
            health_max: 0.0,
            ..Default::default()
        };
        assert_eq!(np.health_fraction(), 0.0);
    }

    #[test]
    fn health_bar_uses_class_color_for_friendly_player() {
        let np = NameplateVisuals {
            reaction: UnitReaction::Friendly,
            class_color: Some(ClassColor::Paladin),
            ..Default::default()
        };
        let color = np.health_bar_color();
        let [r, g, b] = ClassColor::Paladin.rgb();
        assert!((color[0] - r).abs() < 0.01);
        assert!((color[1] - g).abs() < 0.01);
        assert!((color[2] - b).abs() < 0.01);
    }

    #[test]
    fn health_bar_uses_reaction_for_hostile() {
        let np = NameplateVisuals {
            reaction: UnitReaction::Hostile,
            class_color: Some(ClassColor::Mage),
            ..Default::default()
        };
        assert_eq!(
            np.health_bar_color(),
            UnitReaction::Hostile.health_bar_color()
        );
    }

    #[test]
    fn name_color_uses_class_for_players() {
        let np = NameplateVisuals {
            class_color: Some(ClassColor::Shaman),
            ..Default::default()
        };
        let [r, g, b] = ClassColor::Shaman.rgb();
        let color = np.name_text_color();
        assert!((color[0] - r).abs() < 0.01);
    }

    #[test]
    fn name_color_uses_reaction_for_npcs() {
        let np = NameplateVisuals {
            reaction: UnitReaction::Hostile,
            class_color: None,
            ..Default::default()
        };
        assert_eq!(np.name_text_color(), UnitReaction::Hostile.name_color());
    }

    // --- NameplateCastBar ---

    #[test]
    fn cast_bar_interruptible_color() {
        let bar = NameplateCastBar {
            spell_name: "Fireball".into(),
            progress: 0.5,
            interruptible: true,
        };
        let color = bar.fill_color();
        assert!((color[0] - 1.0).abs() < 0.01); // orange-ish
    }

    #[test]
    fn cast_bar_uninterruptible_color() {
        let bar = NameplateCastBar {
            spell_name: "Hearthstone".into(),
            progress: 0.3,
            interruptible: false,
        };
        let color = bar.fill_color();
        assert!((color[0] - 0.6).abs() < 0.01); // gray
    }
}
