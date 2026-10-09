//! Tooltip scaling delegates to the authoritative shared formula (SimC 3036108,
//! TrinityCore a352b1fa). Source-local DB2 CSVs supply budgets, armor, weapon DPS
//! and the newest Retail squish curve. Local CASC game tables supply socket cost
//! and stamina/rating multipliers, matching the server's generated game tables.

use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use crate::item_catalog::{FOREVER_ITEM_DIR, ItemCatalogEntry};
use crate::spell_catalog::SPELL_DB2_BUILD;
use crate::spell_catalog::csv_records::CsvTable;
use shared::item_data::ItemDefinitionSource;
use shared::item_scaling::{
    self, FLAG2_CASTER_WEAPON, ItemScaling, ItemScalingTables, ItemStatAllocation,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ItemStatTables(ItemScalingTables);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeaponDamage {
    pub min: f32,
    pub max: f32,
    pub speed: f32,
    pub dps: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemStat {
    pub stat: i8,
    pub value: i32,
}

fn tables() -> &'static ItemStatTables {
    static TABLES: OnceLock<ItemStatTables> = OnceLock::new();
    TABLES.get_or_init(|| {
        let dir = crate::paths::resolve_data_path(Path::new("db2").join(SPELL_DB2_BUILD));
        load_item_stat_tables(&dir)
            .unwrap_or_else(|error| panic!("item stat tables unavailable: {error}"))
    })
}

fn tables_for(source: ItemDefinitionSource) -> Option<&'static ItemStatTables> {
    match source {
        ItemDefinitionSource::Retail => Some(tables()),
        ItemDefinitionSource::Forever70205 => {
            static TABLES: OnceLock<Result<ItemStatTables, String>> = OnceLock::new();
            TABLES
                .get_or_init(|| {
                    let dir = crate::paths::resolve_data_path(FOREVER_ITEM_DIR);
                    let result = load_source_tables(&dir, None);
                    if let Err(error) = &result {
                        eprintln!("Forever70205 item stat tables unavailable: {error}");
                    }
                    result
                })
                .as_ref()
                .ok()
        }
    }
}

pub fn item_level_for(source: ItemDefinitionSource, entry: &ItemCatalogEntry) -> u16 {
    tables_for(source).map_or(0, |tables| tables.0.item_level(&scaling(entry)))
}

pub fn item_armor_for(source: ItemDefinitionSource, entry: &ItemCatalogEntry) -> u32 {
    tables_for(source).map_or(0, |tables| tables.armor(entry))
}

pub fn weapon_damage_for(
    source: ItemDefinitionSource,
    entry: &ItemCatalogEntry,
) -> Option<WeaponDamage> {
    tables_for(source)?.damage(entry)
}

pub fn item_stats_for(source: ItemDefinitionSource, entry: &ItemCatalogEntry) -> Vec<ItemStat> {
    tables_for(source).map_or_else(Vec::new, |tables| tables.stats(entry))
}

pub fn item_armor(entry: &ItemCatalogEntry) -> u32 {
    tables().armor(entry)
}
pub fn weapon_damage(entry: &ItemCatalogEntry) -> Option<WeaponDamage> {
    tables().damage(entry)
}
pub fn item_stats(entry: &ItemCatalogEntry) -> Vec<ItemStat> {
    tables().stats(entry)
}

fn scaling(entry: &ItemCatalogEntry) -> ItemScaling {
    let mut stats = [ItemStatAllocation::default(); 10];
    for (index, (&(stat, allocation), target)) in
        entry.stats.iter().zip(stats.iter_mut()).enumerate()
    {
        *target = ItemStatAllocation {
            stat,
            allocation,
            socket_multiplier: entry.stat_socket_multipliers[index],
        };
    }
    ItemScaling {
        class_id: entry.class_id,
        subclass_id: entry.subclass_id,
        quality: entry.quality,
        item_level: entry.item_level,
        squish_era: entry.squish_era,
        delay_ms: entry.delay_ms as u16,
        damage_variance: entry.damage_variance,
        flags2: if entry.caster_weapon {
            FLAG2_CASTER_WEAPON
        } else {
            0
        },
        stats,
    }
}

impl ItemStatTables {
    pub fn armor(&self, entry: &ItemCatalogEntry) -> u32 {
        let item = scaling(entry);
        item_scaling::armor(
            &self.0,
            &item,
            entry.inventory_type,
            self.0.item_level(&item),
        ) as u32
    }

    pub fn damage(&self, entry: &ItemCatalogEntry) -> Option<WeaponDamage> {
        let item = scaling(entry);
        let ilvl = self.0.item_level(&item);
        let damage = item_scaling::weapon(&self.0, &item, entry.inventory_type, ilvl)?;
        Some(WeaponDamage {
            min: damage.min,
            max: damage.max,
            speed: damage.speed,
            dps: item_scaling::weapon_dps(&self.0, &item, entry.inventory_type, ilvl)? as f32,
        })
    }

    pub fn stats(&self, entry: &ItemCatalogEntry) -> Vec<ItemStat> {
        let item = scaling(entry);
        item_scaling::stat_values(&self.0, &item, entry.inventory_type)
            .filter_map(|(stat, value)| {
                (value != 0.0).then_some(ItemStat {
                    stat,
                    value: value as i32,
                })
            })
            .collect()
    }
}

pub fn load_item_stat_tables(dir: &Path) -> Result<ItemStatTables, String> {
    load_source_tables(dir, load_squish(dir)?)
}

fn load_source_tables(
    dir: &Path,
    squish: Option<(u8, Vec<(f64, f64)>)>,
) -> Result<ItemStatTables, String> {
    let read = |name: &str| CsvTable::read(&dir.join(name));
    let quality = |name: &str, key: &str, prefix: &str| quality_rows(&read(name)?, key, prefix);
    let game_tables = crate::paths::resolve_data_path("gametables");
    let modifiers = [
        "Clothmodifier",
        "Leathermodifier",
        "Chainmodifier",
        "Platemodifier",
    ];
    Ok(ItemStatTables(ItemScalingTables {
        armor_quality: quality("ItemArmorQuality.csv", "ID", "Qualitymod")?,
        armor_shield: quality("ItemArmorShield.csv", "ItemLevel", "Quality")?,
        damage: [
            quality("ItemDamageOneHand.csv", "ItemLevel", "Quality")?,
            quality("ItemDamageOneHandCaster.csv", "ItemLevel", "Quality")?,
            quality("ItemDamageTwoHand.csv", "ItemLevel", "Quality")?,
            quality("ItemDamageTwoHandCaster.csv", "ItemLevel", "Quality")?,
        ],
        rand_prop_points: rand_prop_rows(&read("RandPropPoints.csv")?)?,
        armor_total: level_rows(
            &read("ItemArmorTotal.csv")?,
            "ItemLevel",
            ["Cloth", "Leather", "Mail", "Plate"],
        )?,
        armor_location: level_rows(&read("ArmorLocation.csv")?, "ID", modifiers)?
            .into_iter()
            .map(|(id, row)| (id as u8, row))
            .collect(),
        squish,
        socket_cost: load_game_table::<1>(
            &game_tables.join("itemsocketcostperlevel.txt"),
            &["Socket Cost"],
        )?
        .into_iter()
        .map(|row| row[0])
        .collect(),
        stamina_multiplier: load_game_multipliers(&game_tables.join("staminamultbyilvl.txt"))?,
        rating_multiplier: load_game_multipliers(&game_tables.join("combatratingsmultbyilvl.txt"))?,
        ..Default::default()
    }))
}

fn load_game_multipliers(path: &Path) -> Result<Vec<[f32; 4]>, String> {
    load_game_table(
        path,
        &[
            "Armor Multiplier",
            "Weapon Multiplier",
            "Trinket Multiplier",
            "Jewelry Multiplier",
        ],
    )
}

fn load_game_table<const N: usize>(
    path: &Path,
    columns: &[&str; N],
) -> Result<Vec<[f32; N]>, String> {
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut lines = text.lines();
    let headers: Vec<_> = lines
        .next()
        .ok_or_else(|| format!("{}: empty game table", path.display()))?
        .split('\t')
        .collect();
    let mut indexes = [0; N];
    for (index, name) in indexes.iter_mut().zip(columns) {
        *index = headers
            .iter()
            .position(|column| column == name)
            .ok_or_else(|| format!("{}: missing {name}", path.display()))?;
    }
    lines
        .enumerate()
        .map(|(row_index, line)| {
            let fields: Vec<_> = line.split('\t').collect();
            let level = fields[0]
                .parse::<usize>()
                .map_err(|error| format!("{}: bad level: {error}", path.display()))?;
            if level != row_index + 1 {
                return Err(format!("{}: non-contiguous level {level}", path.display()));
            }
            let mut row = [0.0; N];
            for (value, index) in row.iter_mut().zip(indexes) {
                let field = fields
                    .get(index)
                    .ok_or_else(|| format!("{}: short row {level}", path.display()))?;
                *value = field
                    .parse()
                    .map_err(|error| format!("{}: bad value: {error}", path.display()))?;
            }
            Ok(row)
        })
        .collect()
}

fn load_squish(dir: &Path) -> Result<Option<(u8, Vec<(f64, f64)>)>, String> {
    let eras = level_rows(
        &CsvTable::read(&dir.join("ItemSquishEra.csv"))?,
        "ID",
        ["CurveID"],
    )?;
    let Some((era, curve)) = eras
        .into_iter()
        .filter(|(_, row)| row[0] > 0.0)
        .max_by_key(|(era, _)| *era)
    else {
        return Ok(None);
    };
    let table = CsvTable::read(&dir.join("CurvePoint.csv"))?;
    let indexes = [
        table.column("CurveID")?,
        table.column("OrderIndex")?,
        table.column("Pos_0")?,
        table.column("Pos_1")?,
    ];
    let mut points = Vec::new();
    for record in table.records() {
        let field = |index| {
            record
                .get(index)
                .ok_or_else(|| format!("{}: short curve row", table.path().display()))
        };
        if float(field(indexes[0])?, table.path())? != curve[0] {
            continue;
        }
        points.push((
            float(field(indexes[1])?, table.path())? as u32,
            (
                float(field(indexes[2])?, table.path())?,
                float(field(indexes[3])?, table.path())?,
            ),
        ));
    }
    points.sort_by_key(|&(order, _)| order);
    Ok(Some((
        era as u8,
        points.into_iter().map(|(_, point)| point).collect(),
    )))
}

fn float(value: &str, path: &Path) -> Result<f64, String> {
    value
        .parse()
        .map_err(|err| format!("{}: bad value {value:?}: {err}", path.display()))
}

/// Rows keyed by `key`, the values of `columns` in order.
fn level_rows<const N: usize>(
    table: &CsvTable,
    key: &str,
    columns: [&str; N],
) -> Result<HashMap<u16, [f64; N]>, String> {
    let path = table.path();
    let key = table.column(key)?;
    let mut indexes = [0; N];
    for (index, column) in indexes.iter_mut().zip(columns) {
        *index = table.column(column)?;
    }
    let mut rows = HashMap::new();
    for record in table.records() {
        let field = |index: usize| {
            record
                .get(index)
                .ok_or_else(|| format!("{}: short row {record:?}", path.display()))
        };
        let level = float(field(key)?, path)? as u16;
        let mut row = [0.0; N];
        for (value, index) in row.iter_mut().zip(indexes) {
            *value = float(field(index)?, path)?;
        }
        rows.insert(level, row);
    }
    Ok(rows)
}

/// Rows of `{prefix}_0` … `{prefix}_6`, one per item quality.
fn quality_rows(
    table: &CsvTable,
    key: &str,
    prefix: &str,
) -> Result<HashMap<u16, [f64; 7]>, String> {
    let names: [String; 7] = std::array::from_fn(|quality| format!("{prefix}_{quality}"));
    level_rows(table, key, names.each_ref().map(String::as_str))
}

fn rand_prop_rows(table: &CsvTable) -> Result<HashMap<u16, [[f64; 5]; 3]>, String> {
    let names: [String; 15] = std::array::from_fn(|index| {
        let group = ["EpicF", "SuperiorF", "GoodF"][index / 5];
        format!("{group}_{}", index % 5)
    });
    let flat = level_rows(table, "ID", names.each_ref().map(String::as_str))?;
    Ok(flat
        .into_iter()
        .map(|(level, row)| {
            (
                level,
                std::array::from_fn(|group| std::array::from_fn(|index| row[group * 5 + index])),
            )
        })
        .collect())
}

#[cfg(test)]
#[path = "item_stats_tests.rs"]
mod tests;
