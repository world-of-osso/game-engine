//! `FactionTemplate.csv` loading for the shared reaction rules in `shared::faction_reaction`,
//! so the client colours units exactly as the server decides attack and assist.

use std::collections::HashMap;

pub use shared::faction_reaction::{FactionTemplateEntry, Reaction, reaction};

/// Parses wago.tools `FactionTemplate.csv` (all-numeric columns, no quoting) by header name.
pub fn parse_faction_template_csv(
    text: &str,
) -> Result<HashMap<u32, FactionTemplateEntry>, String> {
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
        let row = FactionTemplateEntry {
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

    fn templates() -> HashMap<u32, FactionTemplateEntry> {
        // Shared with the Godot client crate, whose manifest sits two levels deeper.
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .map(|dir| dir.join("data/db2/12.1.0.69933/FactionTemplate.csv"))
            .find(|path| path.exists())
            .expect("data/db2/12.1.0.69933/FactionTemplate.csv above the crate");
        let text = std::fs::read_to_string(path)
            .expect("FactionTemplate.csv from wago.tools build 12.1.0.69933");
        parse_faction_template_csv(&text).expect("parse FactionTemplate.csv")
    }

    // Template ids (FactionTemplate.csv, build 12.1.0.69933): 1 Human player (Faction 1),
    // 2 Orc player (Faction 2), 7 Defias Thug (Faction 7), 11 Stormwind guard (Faction 72),
    // 14 Monster (Faction 14).
    #[test]
    fn parsed_retail_templates_regard_a_human_player_as_expected() {
        let rows = templates();
        let human = rows.get(&1);
        assert_eq!(reaction(rows.get(&11), human), Reaction::Friendly);
        assert_eq!(reaction(human, rows.get(&11)), Reaction::Friendly);
        assert_eq!(reaction(human, rows.get(&2)), Reaction::Hostile);
        assert_eq!(reaction(rows.get(&11), rows.get(&2)), Reaction::Hostile);
        assert_eq!(reaction(rows.get(&14), human), Reaction::Hostile);
        assert_eq!(reaction(rows.get(&7), human), Reaction::Neutral);
    }
}
