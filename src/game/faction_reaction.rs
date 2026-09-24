//! FactionTemplate reaction rules as pure functions over DB2 rows. Dependency-free so the same
//! code can move into shared-protocol and serve both client and server.
//!
//! `reaction` is the template part of TrinityCore `Unit::GetFactionReactionTo`; reputation
//! (forced ranks, at-war) is not consulted.

use std::collections::HashMap;

/// `FACTION_TEMPLATE_FLAG_HOSTILE_BY_DEFAULT` (hates everyone it is not friendly with).
const HOSTILE_BY_DEFAULT: u32 = 0x2000;

/// One `FactionTemplate.csv` row.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FactionTemplateRow {
    pub id: u32,
    pub faction: u32,
    pub flags: u32,
    pub faction_group: u32,
    pub friend_group: u32,
    pub enemy_group: u32,
    pub enemies: [u32; 8],
    pub friends: [u32; 8],
}

impl FactionTemplateRow {
    /// TrinityCore `FactionTemplateEntry::IsFriendlyTo`.
    fn is_friendly_to(&self, other: &Self) -> bool {
        if self.id == other.id {
            return true;
        }
        if other.faction != 0 {
            if self.enemies.contains(&other.faction) {
                return false;
            }
            if self.friends.contains(&other.faction) {
                return true;
            }
        }
        self.friend_group & other.faction_group != 0 || self.faction_group & other.friend_group != 0
    }

    /// TrinityCore `FactionTemplateEntry::IsHostileTo`.
    fn is_hostile_to(&self, other: &Self) -> bool {
        if self.id == other.id {
            return false;
        }
        if other.faction != 0 {
            if self.enemies.contains(&other.faction) {
                return true;
            }
            if self.friends.contains(&other.faction) {
                return false;
            }
        }
        self.enemy_group & other.faction_group != 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reaction {
    Hostile,
    Neutral,
    Friendly,
}

/// How a unit with template `own` regards a unit with template `other`.
pub fn reaction(own: &FactionTemplateRow, other: &FactionTemplateRow) -> Reaction {
    if own.is_hostile_to(other) {
        Reaction::Hostile
    } else if own.is_friendly_to(other) || other.is_friendly_to(own) {
        Reaction::Friendly
    } else if own.flags & HOSTILE_BY_DEFAULT != 0 {
        Reaction::Hostile
    } else {
        Reaction::Neutral
    }
}

/// Parses wago.tools `FactionTemplate.csv` (all-numeric columns, no quoting) by header name.
pub fn parse_faction_template_csv(text: &str) -> Result<HashMap<u32, FactionTemplateRow>, String> {
    let mut lines = text.lines();
    let header: Vec<&str> = lines
        .next()
        .ok_or("FactionTemplate.csv is empty")?
        .split(',')
        .collect();
    let column = |name: &str| {
        header
            .iter()
            .position(|column| *column == name)
            .ok_or_else(|| format!("FactionTemplate.csv has no {name} column"))
    };
    let scalar = [
        "ID",
        "Faction",
        "Flags",
        "FactionGroup",
        "FriendGroup",
        "EnemyGroup",
    ]
    .map(column)
    .into_iter()
    .collect::<Result<Vec<_>, _>>()?;
    let enemies = std::array::from_fn::<_, 8, _>(|i| column(&format!("Enemies_{i}")));
    let friends = std::array::from_fn::<_, 8, _>(|i| column(&format!("Friend_{i}")));
    let enemies = enemies.into_iter().collect::<Result<Vec<_>, _>>()?;
    let friends = friends.into_iter().collect::<Result<Vec<_>, _>>()?;
    let mut rows = HashMap::new();
    for (line_no, line) in lines.enumerate().filter(|(_, line)| !line.is_empty()) {
        let fields: Vec<&str> = line.split(',').collect();
        let value = |index: usize| -> Result<u32, String> {
            let field = fields.get(index).copied().unwrap_or_default();
            field
                .parse::<i64>()
                .map(|value| value as u32)
                .map_err(|err| {
                    format!("FactionTemplate.csv line {}: {field:?}: {err}", line_no + 2)
                })
        };
        let list = |columns: &[usize]| -> Result<[u32; 8], String> {
            let mut out = [0; 8];
            for (slot, &index) in out.iter_mut().zip(columns) {
                *slot = value(index)?;
            }
            Ok(out)
        };
        let row = FactionTemplateRow {
            id: value(scalar[0])?,
            faction: value(scalar[1])?,
            flags: value(scalar[2])?,
            faction_group: value(scalar[3])?,
            friend_group: value(scalar[4])?,
            enemy_group: value(scalar[5])?,
            enemies: list(&enemies)?,
            friends: list(&friends)?,
        };
        rows.insert(row.id, row);
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn templates() -> HashMap<u32, FactionTemplateRow> {
        let text = std::fs::read_to_string("data/db2/12.1.0.69933/FactionTemplate.csv")
            .expect("FactionTemplate.csv from wago.tools build 12.1.0.69933");
        parse_faction_template_csv(&text).expect("parse FactionTemplate.csv")
    }

    // Template ids (FactionTemplate.csv, build 12.1.0.69933): 1 Human player (Faction 1),
    // 2 Orc player (Faction 2), 11 Stormwind guard (Faction 72), 14 Monster (Faction 14).
    #[test]
    fn human_player_is_friendly_to_stormwind_guard_and_hostile_to_orc() {
        let rows = templates();
        let (human, orc, guard, monster) = (&rows[&1], &rows[&2], &rows[&11], &rows[&14]);
        assert_eq!(reaction(guard, human), Reaction::Friendly);
        assert_eq!(reaction(human, guard), Reaction::Friendly);
        assert_eq!(reaction(human, orc), Reaction::Hostile);
        assert_eq!(reaction(guard, orc), Reaction::Hostile);
        assert_eq!(reaction(monster, human), Reaction::Hostile);
    }

    #[test]
    fn explicit_enemy_list_overrides_group_friendship() {
        let own = FactionTemplateRow {
            id: 100,
            faction_group: 1,
            friend_group: 1,
            enemies: [7, 0, 0, 0, 0, 0, 0, 0],
            ..Default::default()
        };
        let other = FactionTemplateRow {
            id: 101,
            faction: 7,
            faction_group: 1,
            ..Default::default()
        };
        assert_eq!(reaction(&own, &other), Reaction::Hostile);
    }

    #[test]
    fn unrelated_templates_are_neutral_unless_hostile_by_default() {
        let own = FactionTemplateRow {
            id: 1,
            faction: 1,
            ..Default::default()
        };
        let other = FactionTemplateRow {
            id: 2,
            faction: 2,
            ..Default::default()
        };
        assert_eq!(reaction(&own, &other), Reaction::Neutral);
        let hater = FactionTemplateRow {
            flags: HOSTILE_BY_DEFAULT,
            ..own
        };
        assert_eq!(reaction(&hater, &other), Reaction::Hostile);
    }
}
