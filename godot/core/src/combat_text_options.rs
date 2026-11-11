//! Floating combat-text switches backed by Retail CVar defaults.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CombatTextOptions {
    #[serde(rename = "enableFloatingCombatText")]
    pub self_enabled: bool,
    #[serde(rename = "floatingCombatTextCombatDamage_v2")]
    pub world_damage: bool,
    #[serde(rename = "floatingCombatTextCombatHealing_v2")]
    pub world_healing: bool,
    #[serde(rename = "floatingCombatTextCombatHealingAbsorbSelf_v2")]
    pub healing_absorb_self: bool,
    #[serde(rename = "floatingCombatTextCombatHealingAbsorbTarget_v2")]
    pub healing_absorb_target: bool,
    #[serde(rename = "floatingCombatTextCombatLogPeriodicSpells_v2")]
    pub periodic: bool,
    #[serde(rename = "floatingCombatTextPetMeleeDamage_v2")]
    pub pet_melee: bool,
    #[serde(rename = "floatingCombatTextPetSpellDamage_v2")]
    pub pet_spell: bool,
    #[serde(rename = "floatingCombatTextDodgeParryMiss_v2")]
    pub avoidance: bool,
    #[serde(rename = "floatingCombatTextDamageReduction_v2")]
    pub reductions: bool,
    #[serde(rename = "floatingCombatTextFriendlyHealers_v2")]
    pub friendly_healers: bool,
    #[serde(rename = "floatingCombatTextFloatMode_v2")]
    pub float_mode: i32,
    #[serde(rename = "WorldTextScale_v2")]
    pub world_scale: f32,
    #[serde(rename = "WorldTextScreenY_v2")]
    pub screen_y: f32,
    #[serde(rename = "WorldTextCritScreenY_v2")]
    pub crit_screen_y: f32,
}

impl Default for CombatTextOptions {
    fn default() -> Self {
        Self {
            self_enabled: false,
            world_damage: true,
            world_healing: true,
            healing_absorb_self: true,
            healing_absorb_target: true,
            periodic: true,
            pet_melee: true,
            pet_spell: true,
            avoidance: false,
            reductions: false,
            friendly_healers: false,
            float_mode: 1,
            world_scale: 1.0,
            screen_y: 0.015,
            crit_screen_y: 0.0275,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_options_load_retail_combat_text_defaults() {
        let parsed: CombatTextOptions = ron::from_str("()").unwrap();
        assert_eq!(parsed, CombatTextOptions::default());
        let old: CombatTextOptions = ron::from_str("(enableFloatingCombatText:true)").unwrap();
        assert!(old.self_enabled);
        assert!(old.world_damage && old.world_healing && old.periodic);
    }

    #[test]
    fn cvar_named_fields_round_trip_without_losing_values() {
        let options = CombatTextOptions {
            self_enabled: true,
            world_damage: false,
            avoidance: true,
            float_mode: 3,
            world_scale: 1.5,
            ..Default::default()
        };
        let encoded = ron::to_string(&options).unwrap();
        assert!(encoded.contains("enableFloatingCombatText"));
        assert!(encoded.contains("floatingCombatTextDodgeParryMiss_v2"));
        assert_eq!(
            ron::from_str::<CombatTextOptions>(&encoded).unwrap(),
            options
        );
    }
}
