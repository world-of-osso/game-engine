//! Item armor, weapon damage and stat values from source-local DB2 exports, as
//! TrinityCore `ItemTemplate::GetArmor` / `GetDPS` / `GetDamage` and
//! `Item::GetItemStatValue` with `GetRandomPropertyPoints` compute them (TrinityCore
//! a352b1fa, ItemTemplate.cpp:147-269, Item.cpp:2385-2408, ItemEnchantmentMgr.cpp:107-175).
//! Not modelled: the socket stat cost (`gtItemSocketCostPerLevel`) and the item-level
//! stamina and rating multipliers (`gtStaminaMultByILvl`, `gtCombatRatingsMultByILvl`),
//! whose game tables are not exported; bonus lists and item-level scaling curves.

use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use crate::item_catalog::{FOREVER_ITEM_DIR, ItemCatalogEntry};
use crate::spell_catalog::SPELL_DB2_BUILD;
use crate::spell_catalog::csv_records::CsvTable;
use shared::item_data::ItemDefinitionSource;

const ITEM_CLASS_WEAPON: u8 = 2;
const ITEM_CLASS_ARMOR: u8 = 4;
const ARMOR_CLOTH: u8 = 1;
const ARMOR_PLATE: u8 = 4;
const ARMOR_SHIELD: u8 = 6;
const WEAPON_BOW: u8 = 2;
const WEAPON_GUN: u8 = 3;
const WEAPON_CROSSBOW: u8 = 18;
const WEAPON_WAND: u8 = 19;
const QUALITY_UNCOMMON: u8 = 2;
const QUALITY_RARE: u8 = 3;
const QUALITY_ARTIFACT: u8 = 6;
const QUALITY_HEIRLOOM: u8 = 7;

/// Per-item-level tables, each row the seven `Enum.ItemQuality` values (0 Poor … 6 Artifact).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ItemStatTables {
    armor_quality: HashMap<u16, [f32; 7]>,
    /// Cloth, Leather, Mail, Plate.
    armor_total: HashMap<u16, [f32; 4]>,
    /// Cloth, Leather, Chain, Plate modifiers by `InventoryType`.
    armor_location: HashMap<u8, [f32; 4]>,
    armor_shield: HashMap<u16, [f32; 7]>,
    damage_one_hand: HashMap<u16, [f32; 7]>,
    damage_one_hand_caster: HashMap<u16, [f32; 7]>,
    damage_two_hand: HashMap<u16, [f32; 7]>,
    damage_two_hand_caster: HashMap<u16, [f32; 7]>,
    /// `GoodF`, `SuperiorF`, `EpicF`, five columns each.
    rand_prop_points: HashMap<u16, [[f32; 5]; 3]>,
}

/// A weapon's damage range, swing time and damage per second.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeaponDamage {
    pub min: f32,
    pub max: f32,
    pub speed: f32,
    pub dps: f32,
}

/// One displayed stat: the `ITEM_MOD_*` type and its value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemStat {
    pub stat: i8,
    pub value: i32,
}

fn tables() -> &'static ItemStatTables {
    static TABLES: OnceLock<ItemStatTables> = OnceLock::new();
    TABLES.get_or_init(|| {
        let dir = crate::paths::resolve_data_path(Path::new("db2").join(SPELL_DB2_BUILD));
        load_item_stat_tables(&dir).unwrap_or_else(|err| {
            eprintln!("item stat tables unavailable: {err}");
            ItemStatTables::default()
        })
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
                    let result = load_item_stat_tables(&dir);
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

/// `ARMOR_TEMPLATE` value of `entry`; 0 for items without armor.
pub fn item_armor(entry: &ItemCatalogEntry) -> u32 {
    tables().armor(entry)
}

/// The weapon damage of `entry`; `None` for items that are not weapons.
pub fn weapon_damage(entry: &ItemCatalogEntry) -> Option<WeaponDamage> {
    tables().damage(entry)
}

/// The item's stats in their `StatModifier_bonusStat` order; zero values dropped.
pub fn item_stats(entry: &ItemCatalogEntry) -> Vec<ItemStat> {
    tables().stats(entry)
}

/// Heirlooms use the rare rows; qualities above artifact have none.
fn table_quality(quality: u8) -> Option<usize> {
    let quality = if quality == QUALITY_HEIRLOOM {
        QUALITY_RARE
    } else {
        quality
    };
    (quality <= QUALITY_ARTIFACT).then_some(usize::from(quality))
}

impl ItemStatTables {
    pub fn armor(&self, entry: &ItemCatalogEntry) -> u32 {
        let Some(quality) = table_quality(entry.quality) else {
            return 0;
        };
        if entry.class_id != ITEM_CLASS_ARMOR {
            return 0;
        }
        let level = entry.item_level;
        if entry.subclass_id == ARMOR_SHIELD {
            return self
                .armor_shield
                .get(&level)
                .map_or(0, |row| (row[quality] + 0.5) as u32);
        }
        if !(ARMOR_CLOTH..=ARMOR_PLATE).contains(&entry.subclass_id) {
            return 0;
        }
        // INVTYPE_ROBE reads the chest row.
        let inventory_type = if entry.inventory_type == 20 {
            5
        } else {
            entry.inventory_type
        };
        let (Some(quality_mod), Some(total), Some(location)) = (
            self.armor_quality.get(&level),
            self.armor_total.get(&level),
            self.armor_location.get(&inventory_type),
        ) else {
            return 0;
        };
        let kind = usize::from(entry.subclass_id - ARMOR_CLOTH);
        (quality_mod[quality] * total[kind] * location[kind] + 0.5) as u32
    }

    fn dps_table(&self, entry: &ItemCatalogEntry) -> Option<&HashMap<u16, [f32; 7]>> {
        let (one, two) = if entry.caster_weapon {
            (&self.damage_one_hand_caster, &self.damage_two_hand_caster)
        } else {
            (&self.damage_one_hand, &self.damage_two_hand)
        };
        match entry.inventory_type {
            // INVTYPE_2HWEAPON.
            17 => Some(two),
            // INVTYPE_RANGED, INVTYPE_THROWN, INVTYPE_RANGEDRIGHT.
            15 | 25 | 26 => match entry.subclass_id {
                WEAPON_WAND => Some(&self.damage_one_hand_caster),
                WEAPON_BOW | WEAPON_GUN | WEAPON_CROSSBOW => Some(two),
                _ => None,
            },
            // INVTYPE_WEAPON, INVTYPE_WEAPONMAINHAND, INVTYPE_WEAPONOFFHAND.
            13 | 21 | 22 => Some(one),
            _ => None,
        }
    }

    pub fn damage(&self, entry: &ItemCatalogEntry) -> Option<WeaponDamage> {
        let quality = table_quality(entry.quality)?;
        if entry.class_id != ITEM_CLASS_WEAPON || entry.delay_ms == 0 {
            return None;
        }
        let dps = *self
            .dps_table(entry)?
            .get(&entry.item_level)?
            .get(quality)?;
        if dps <= 0.0 {
            return None;
        }
        let speed = entry.delay_ms as f32 / 1000.0;
        let average = dps * speed;
        let variance = entry.damage_variance;
        Some(WeaponDamage {
            min: (variance * -0.5 + 1.0) * average,
            max: (average * (variance * 0.5 + 1.0) + 0.5).floor(),
            speed,
            dps,
        })
    }

    /// `GetRandomPropertyPoints` column for the item's inventory type.
    fn prop_points(&self, entry: &ItemCatalogEntry) -> f32 {
        let index = match entry.inventory_type {
            1 | 4 | 5 | 7 | 15 | 17 | 20 | 25 => 0,
            26 if entry.subclass_id == WEAPON_WAND => 3,
            26 => 0,
            13 | 21 | 22 => 3,
            3 | 6 | 8 | 10 | 12 => 1,
            2 | 9 | 11 | 14 | 16 | 23 => 2,
            28 => 4,
            _ => return 0.0,
        };
        let Some(row) = self.rand_prop_points.get(&entry.item_level) else {
            return 0.0;
        };
        match entry.quality {
            QUALITY_UNCOMMON => row[0][index],
            3 | QUALITY_HEIRLOOM => row[1][index],
            4..=QUALITY_ARTIFACT => row[2][index],
            _ => 0.0,
        }
    }

    pub fn stats(&self, entry: &ItemCatalogEntry) -> Vec<ItemStat> {
        let points = self.prop_points(entry);
        entry
            .stats
            .iter()
            .filter_map(|&(stat, percent)| {
                let value = (percent as f32 * points * 0.0001).round() as i32;
                (value != 0).then_some(ItemStat { stat, value })
            })
            .collect()
    }
}

/// The armor, damage and random-property tables of one DB2 export directory.
pub fn load_item_stat_tables(dir: &Path) -> Result<ItemStatTables, String> {
    let read = |name: &str| CsvTable::read(&dir.join(name));
    let quality = |name: &str, key: &str, prefix: &str| quality_rows(&read(name)?, key, prefix);
    let mut tables = ItemStatTables {
        armor_quality: quality("ItemArmorQuality.csv", "ID", "Qualitymod")?,
        armor_shield: quality("ItemArmorShield.csv", "ItemLevel", "Quality")?,
        damage_one_hand: quality("ItemDamageOneHand.csv", "ItemLevel", "Quality")?,
        damage_one_hand_caster: quality("ItemDamageOneHandCaster.csv", "ItemLevel", "Quality")?,
        damage_two_hand: quality("ItemDamageTwoHand.csv", "ItemLevel", "Quality")?,
        damage_two_hand_caster: quality("ItemDamageTwoHandCaster.csv", "ItemLevel", "Quality")?,
        rand_prop_points: rand_prop_rows(&read("RandPropPoints.csv")?)?,
        ..Default::default()
    };
    tables.armor_total = level_rows(
        &read("ItemArmorTotal.csv")?,
        "ItemLevel",
        ["Cloth", "Leather", "Mail", "Plate"],
    )?;
    let modifiers = [
        "Clothmodifier",
        "Leathermodifier",
        "Chainmodifier",
        "Platemodifier",
    ];
    tables.armor_location = level_rows(&read("ArmorLocation.csv")?, "ID", modifiers)?
        .into_iter()
        .map(|(id, row)| (id as u8, row))
        .collect();
    Ok(tables)
}

fn float(value: &str, path: &Path) -> Result<f32, String> {
    value
        .parse()
        .map_err(|err| format!("{}: bad value {value:?}: {err}", path.display()))
}

/// Rows keyed by `key`, the values of `columns` in order.
fn level_rows<const N: usize>(
    table: &CsvTable,
    key: &str,
    columns: [&str; N],
) -> Result<HashMap<u16, [f32; N]>, String> {
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
) -> Result<HashMap<u16, [f32; 7]>, String> {
    let names: [String; 7] = std::array::from_fn(|quality| format!("{prefix}_{quality}"));
    level_rows(table, key, names.each_ref().map(String::as_str))
}

fn rand_prop_rows(table: &CsvTable) -> Result<HashMap<u16, [[f32; 5]; 3]>, String> {
    let names: [String; 15] = std::array::from_fn(|index| {
        let group = ["GoodF", "SuperiorF", "EpicF"][index / 5];
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
