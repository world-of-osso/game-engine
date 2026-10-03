//! Engine-free display health of a level-scaled creature. TrinityCore replicates a
//! tuned creature's one health pool at its native level (the top of its ContentTuning
//! range) and scales what each unit does to it by `GetHealthMultiplierForTarget`
//! (Creature.cpp:3181-3189: max health at the unit's level over the native max health),
//! so a viewer sees that pool times the same multiplier: the Blackrock Spy's 4379
//! (level 30) is 377 to a level-10 player.

use std::collections::HashMap;

/// ExpectedStat `ExpansionID` whose rows apply to every expansion without its own row
/// (DB2Manager::EvaluateExpectedStat).
const DEFAULT_EXPANSION: &str = "-2";

/// ExpectedStat `CreatureHealth` by `Lvl` from the default-expansion rows. The creature's
/// `HealthScalingExpansion` is not replicated, so the other expansions' rows (levels 26-53
/// of expansions 0-7, 70-85 of later ones in 12.1.0) are not used; ContentTuning and class
/// ExpectedStatMods multiply both levels alike and cancel in the ratio.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CreatureHealthByLevel(HashMap<u8, f32>);

impl CreatureHealthByLevel {
    pub fn parse_expected_stat_csv(text: &str) -> Result<Self, String> {
        let mut lines = text.lines();
        let header: Vec<&str> = lines
            .next()
            .ok_or("ExpectedStat.csv is empty")?
            .split(',')
            .collect();
        let column = |name: &str| {
            header
                .iter()
                .position(|column| *column == name)
                .ok_or_else(|| format!("ExpectedStat.csv has no {name} column"))
        };
        let (expansion, health, level) = (
            column("ExpansionID")?,
            column("CreatureHealth")?,
            column("Lvl")?,
        );
        let mut by_level = HashMap::new();
        for line in lines.filter(|line| !line.is_empty()) {
            let fields: Vec<&str> = line.split(',').collect();
            if fields.get(expansion) != Some(&DEFAULT_EXPANSION) {
                continue;
            }
            let parse = |index: usize| fields.get(index).map(|field| field.trim());
            let level: u8 = parse(level)
                .and_then(|field| field.parse().ok())
                .ok_or_else(|| format!("ExpectedStat.csv bad Lvl in {line}"))?;
            let health: f32 = parse(health)
                .and_then(|field| field.parse().ok())
                .ok_or_else(|| format!("ExpectedStat.csv bad CreatureHealth in {line}"))?;
            by_level.insert(level, health);
        }
        Ok(Self(by_level))
    }

    /// `GetHealthMultiplierForTarget` for a creature of `native_level` fought at
    /// `level_for_target`; 1 when either level has no row.
    pub fn health_multiplier(&self, native_level: u8, level_for_target: u8) -> f32 {
        match (self.0.get(&level_for_target), self.0.get(&native_level)) {
            (Some(target), Some(native)) if *native > 0.0 => target / native,
            _ => 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ExpectedStat 12.1.0.69933 rows 10 and 30 (default expansion) and an expansion-8 row.
    const CSV: &str = "ID,ExpansionID,CreatureHealth,PlayerHealth,CreatureAutoAttackDps,CreatureArmor,PlayerMana,PlayerPrimaryStat,PlayerSecondaryStat,ArmorConstant,CreatureSpellDamage,ContentSetID,Lvl
10,-2,377.3422546386719,1115.8746337890625,17.05213165283203,90.0,1000.0,41.1169548034668,3.5507280826568604,210.0,1048.3651123046875,0,10
30,-2,4378.77490234375,8459.818359375,162.6229248046875,155.0,3870.0,168.60362243652344,194.5824737548828,361.0,8211.373046875,0,30
900,8,99999.0,1.0,1.0,1.0,1.0,1.0,1.0,1.0,1.0,0,30
";

    #[test]
    fn a_level_30_spy_shows_level_10_health_to_a_level_10_player() {
        let table = CreatureHealthByLevel::parse_expected_stat_csv(CSV).unwrap();
        let multiplier = table.health_multiplier(30, 10);
        assert!(
            (multiplier - 377.342_25 / 4378.775).abs() < 1e-6,
            "{multiplier}"
        );
        // The spy's replicated 4379 native health.
        assert_eq!((4379.0 * multiplier).round(), 377.0);
    }

    #[test]
    fn untuned_levels_and_missing_rows_leave_health_unscaled() {
        let table = CreatureHealthByLevel::parse_expected_stat_csv(CSV).unwrap();
        assert_eq!(table.health_multiplier(30, 30), 1.0);
        assert_eq!(table.health_multiplier(30, 55), 1.0);
    }
}
