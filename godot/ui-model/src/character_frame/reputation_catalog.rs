//! Optional faction presentation comes from the pinned Retail Faction.db2 export.
//! Wire snapshots deliberately remain untouched; descriptions and static eligibility
//! are not substitutes for the server's current at-war state.

use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use super::reputation::ReputationRow;
use crate::csv_records::CsvTable;

struct Faction {
    description: String,
    rules: [(u64, u32, u16); 4],
}

struct Factions {
    factions: HashMap<u32, Faction>,
    race_bits: HashMap<u8, u32>,
}

static FACTIONS: OnceLock<Result<Factions, String>> = OnceLock::new();

/// Join descriptions and the player's race/class reputation rule, reporting corrupt or
/// missing tables rather than silently making eligible factions disappear.
pub fn enrich_reputation_rows(
    rows: &mut [ReputationRow],
    race: u8,
    class: u8,
) -> Result<(), String> {
    let data = FACTIONS
        .get_or_init(|| read_factions(&crate::paths::resolve_data_path("db2/12.1.0.69933")))
        .as_ref()
        .map_err(Clone::clone)?;
    let race_mask = data
        .race_bits
        .get(&race)
        .and_then(|bit| 1u64.checked_shl(*bit));
    let class_mask = class
        .checked_sub(1)
        .and_then(|bit| 1u32.checked_shl(u32::from(bit)));
    for row in rows {
        let Some(faction) = data.factions.get(&row.faction_id) else {
            continue;
        };
        row.description.clone_from(&faction.description);
        let flags = race_mask.zip(class_mask).and_then(|(race, class)| {
            faction
                .rules
                .iter()
                .find(|(races, classes, _)| {
                    (*races == 0 || races & race != 0) && (*classes == 0 || classes & class != 0)
                })
                .map(|(_, _, flags)| *flags)
        });
        // TrinityCore a352b1fa ReputationMgr.h:29-43; ReputationMgr.cpp:625-635,774-790.
        // Hidden/header/peaceful factions do not expose a user declaration of war.
        row.allows_at_war = flags.is_some_and(|flags| flags & (0x4 | 0x8 | 0x10) == 0);
    }
    Ok(())
}

fn read_factions(dir: &Path) -> Result<Factions, String> {
    let table = CsvTable::read(&dir.join("Faction.csv"))?;
    let id = table.column("ID")?;
    let description = table.column("Description_lang")?;
    let columns = (0..4)
        .map(|index| {
            Ok([
                table.column(&format!("ReputationRaceMasks{index}_0"))?,
                table.column(&format!("ReputationRaceMasks{index}_1"))?,
                table.column(&format!("ReputationClassMask_{index}"))?,
                table.column(&format!("ReputationFlags_{index}"))?,
            ])
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut factions = HashMap::new();
    for row in table.records() {
        let mut rules = [(0, 0, 0); 4];
        for (index, [lo, hi, class, flags]) in columns.iter().copied().enumerate() {
            rules[index] = (
                u64::from(parse_mask(field(&row, lo)?)?)
                    | (u64::from(parse_mask(field(&row, hi)?)?) << 32),
                parse_mask(field(&row, class)?)?,
                field(&row, flags)?
                    .parse()
                    .map_err(|e| format!("Faction flags: {e}"))?,
            );
        }
        factions.insert(
            field(&row, id)?
                .parse()
                .map_err(|e| format!("Faction ID: {e}"))?,
            Faction {
                description: field(&row, description)?.into(),
                rules,
            },
        );
    }
    let table = CsvTable::read(&dir.join("ChrRaces.csv"))?;
    let id = table.column("ID")?;
    let bit = table.column("PlayableRaceBit")?;
    let mut race_bits = HashMap::new();
    for row in table.records() {
        let bit: i32 = field(&row, bit)?
            .parse()
            .map_err(|e| format!("ChrRaces PlayableRaceBit: {e}"))?;
        if bit >= 0 {
            let id = field(&row, id)?
                .parse()
                .map_err(|e| format!("ChrRaces ID: {e}"))?;
            race_bits.insert(id, bit as u32);
        }
    }
    Ok(Factions {
        factions,
        race_bits,
    })
}

fn field<'a>(row: &'a [std::borrow::Cow<'_, str>], index: usize) -> Result<&'a str, String> {
    row.get(index)
        .map(|field| field.as_ref())
        .ok_or_else(|| format!("Faction metadata row missing column {index}"))
}

fn parse_mask(value: &str) -> Result<u32, String> {
    value
        .parse::<i64>()
        .map(|value| value as u32)
        .map_err(|e| format!("Faction mask {value:?}: {e}"))
}
