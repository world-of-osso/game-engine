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

/// Threat level for aggro indicators on nameplates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ThreatLevel {
    /// No threat — no glow.
    #[default]
    None,
    /// Low threat — gaining aggro (yellow glow).
    Low,
    /// High threat — about to pull aggro (orange glow).
    High,
    /// Tanking — currently has aggro (red glow).
    Tanking,
}

impl ThreatLevel {
    /// Glow border color for this threat level.
    pub fn glow_color(self) -> Option<[f32; 4]> {
        match self {
            Self::None => Option::None,
            Self::Low => Some([1.0, 1.0, 0.0, 0.5]),
            Self::High => Some([1.0, 0.5, 0.0, 0.7]),
            Self::Tanking => Some([1.0, 0.0, 0.0, 0.9]),
        }
    }

    /// Whether the glow indicator should be shown.
    pub fn has_glow(self) -> bool {
        !matches!(self, Self::None)
    }

    /// Determine threat level from threat percentage (0–100+).
    pub fn from_pct(pct: f32) -> Self {
        if pct >= 100.0 {
            Self::Tanking
        } else if pct >= 80.0 {
            Self::High
        } else if pct >= 50.0 {
            Self::Low
        } else {
            Self::None
        }
    }
}

/// Quest giver indicator above a nameplate (! or ?).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum QuestIndicator {
    #[default]
    None,
    /// Yellow ! — NPC has a new quest available.
    Available,
    /// Yellow ? — NPC can accept a completed quest turn-in.
    TurnIn,
    /// Silver ! — NPC has a quest available but requirements not met.
    Unavailable,
    /// Blue ! — Daily quest available.
    DailyAvailable,
    /// Blue ? — Daily quest turn-in.
    DailyTurnIn,
    /// Orange ! — Campaign/legendary quest.
    CampaignAvailable,
}

impl QuestIndicator {
    /// The display glyph (! or ?).
    pub fn glyph(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Available
            | Self::Unavailable
            | Self::DailyAvailable
            | Self::CampaignAvailable => "!",
            Self::TurnIn | Self::DailyTurnIn => "?",
        }
    }

    /// RGBA color for the indicator.
    pub fn color(self) -> [f32; 4] {
        match self {
            Self::None => [0.0; 4],
            Self::Available | Self::TurnIn => [1.0, 0.82, 0.0, 1.0],
            Self::Unavailable => [0.7, 0.7, 0.7, 1.0],
            Self::DailyAvailable | Self::DailyTurnIn => [0.3, 0.5, 1.0, 1.0],
            Self::CampaignAvailable => [1.0, 0.5, 0.0, 1.0],
        }
    }

    /// Whether any indicator should be shown.
    pub fn is_visible(self) -> bool {
        !matches!(self, Self::None)
    }

    /// Texture FDID for the indicator icon (from quest_data textures).
    pub fn icon_fdid(self) -> u32 {
        match self {
            Self::None => 0,
            Self::Available => crate::quest_data::textures::QUEST_BANG_NORMAL,
            Self::TurnIn => crate::quest_data::textures::QUEST_TURNIN,
            Self::DailyAvailable | Self::DailyTurnIn => {
                crate::quest_data::textures::QUEST_BANG_DAILY
            }
            Self::CampaignAvailable => crate::quest_data::textures::QUEST_BANG_CAMPAIGN,
            Self::Unavailable => crate::quest_data::textures::QUEST_BANG_NORMAL,
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
    /// Threat/aggro level for glow indicator.
    pub threat: ThreatLevel,
    /// Quest giver indicator (! or ?).
    pub quest_indicator: QuestIndicator,
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
            threat: ThreatLevel::None,
            quest_indicator: QuestIndicator::None,
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

    // --- ThreatLevel ---

    #[test]
    fn threat_none_has_no_glow() {
        assert!(!ThreatLevel::None.has_glow());
        assert!(ThreatLevel::None.glow_color().is_none());
    }

    #[test]
    fn threat_levels_have_glow() {
        assert!(ThreatLevel::Low.has_glow());
        assert!(ThreatLevel::High.has_glow());
        assert!(ThreatLevel::Tanking.has_glow());
    }

    #[test]
    fn threat_glow_colors_distinct() {
        let low = ThreatLevel::Low.glow_color().unwrap();
        let high = ThreatLevel::High.glow_color().unwrap();
        let tanking = ThreatLevel::Tanking.glow_color().unwrap();
        assert_ne!(low, high);
        assert_ne!(high, tanking);
    }

    #[test]
    fn threat_from_pct_thresholds() {
        assert_eq!(ThreatLevel::from_pct(0.0), ThreatLevel::None);
        assert_eq!(ThreatLevel::from_pct(49.0), ThreatLevel::None);
        assert_eq!(ThreatLevel::from_pct(50.0), ThreatLevel::Low);
        assert_eq!(ThreatLevel::from_pct(79.0), ThreatLevel::Low);
        assert_eq!(ThreatLevel::from_pct(80.0), ThreatLevel::High);
        assert_eq!(ThreatLevel::from_pct(99.0), ThreatLevel::High);
        assert_eq!(ThreatLevel::from_pct(100.0), ThreatLevel::Tanking);
        assert_eq!(ThreatLevel::from_pct(150.0), ThreatLevel::Tanking);
    }

    #[test]
    fn threat_tanking_glow_is_reddest() {
        let tanking = ThreatLevel::Tanking.glow_color().unwrap();
        assert!((tanking[0] - 1.0).abs() < 0.01); // red channel max
        assert!(tanking[3] > 0.8); // high alpha
    }

    #[test]
    fn nameplate_with_threat() {
        let np = NameplateVisuals {
            threat: ThreatLevel::Tanking,
            ..Default::default()
        };
        assert!(np.threat.has_glow());
    }

    // --- Quest indicators ---

    #[test]
    fn quest_indicator_none_not_visible() {
        assert!(!QuestIndicator::None.is_visible());
        assert_eq!(QuestIndicator::None.glyph(), "");
    }

    #[test]
    fn quest_available_shows_exclamation() {
        let qi = QuestIndicator::Available;
        assert!(qi.is_visible());
        assert_eq!(qi.glyph(), "!");
        // Yellow color
        assert!((qi.color()[0] - 1.0).abs() < 0.01);
    }

    #[test]
    fn quest_turnin_shows_question() {
        let qi = QuestIndicator::TurnIn;
        assert_eq!(qi.glyph(), "?");
        assert!(qi.is_visible());
    }

    #[test]
    fn daily_indicators_are_blue() {
        let avail = QuestIndicator::DailyAvailable;
        let turnin = QuestIndicator::DailyTurnIn;
        assert_eq!(avail.glyph(), "!");
        assert_eq!(turnin.glyph(), "?");
        // Blue-ish color
        assert!(avail.color()[2] > 0.8);
        assert!(turnin.color()[2] > 0.8);
    }

    #[test]
    fn campaign_is_orange() {
        let qi = QuestIndicator::CampaignAvailable;
        assert_eq!(qi.glyph(), "!");
        assert!((qi.color()[0] - 1.0).abs() < 0.01);
        assert!((qi.color()[1] - 0.5).abs() < 0.01);
    }

    #[test]
    fn unavailable_is_gray() {
        let qi = QuestIndicator::Unavailable;
        assert_eq!(qi.glyph(), "!");
        assert!((qi.color()[0] - 0.7).abs() < 0.01);
    }

    #[test]
    fn quest_icon_fdids_nonzero() {
        assert_ne!(QuestIndicator::Available.icon_fdid(), 0);
        assert_ne!(QuestIndicator::TurnIn.icon_fdid(), 0);
        assert_ne!(QuestIndicator::DailyAvailable.icon_fdid(), 0);
        assert_ne!(QuestIndicator::CampaignAvailable.icon_fdid(), 0);
        assert_eq!(QuestIndicator::None.icon_fdid(), 0);
    }

    #[test]
    fn nameplate_with_quest_indicator() {
        let np = NameplateVisuals {
            quest_indicator: QuestIndicator::Available,
            ..Default::default()
        };
        assert!(np.quest_indicator.is_visible());
    }
}
