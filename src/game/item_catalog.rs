//! The client's item catalog, one entry per item ID from the build-pinned Retail
//! DB2 exports: `Item.csv` (class, subclass, icon) joined with `ItemSparse.csv`
//! (name, quality, stack size, sell price, binding, level, inventory type), and the
//! subclass names of `ItemSubClass.csv`. The
//! server sends item IDs and counts only, so bags, the auction house and item
//! tooltips resolve everything else here, as Retail's client resolves item data
//! from its DB2 cache (`C_Item.GetItemInfo`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::spell_catalog::SPELL_DB2_BUILD;
use crate::spell_catalog::csv_records::CsvTable;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ItemCatalogEntry {
    pub class_id: u8,
    pub subclass_id: u8,
    pub icon_fdid: u32,
    /// `SheatheType`: where the item sits when sheathed (0 none).
    pub sheathe_type: u8,
    /// `Display_lang`.
    pub name: String,
    /// `OverallQualityID` (Retail `Enum.ItemQuality`, 0 Poor … 7 Heirloom).
    pub quality: u8,
    /// `Stackable`: the largest stack.
    pub stackable: u32,
    /// `SellPrice` in copper per item; 0 means vendors don't buy it.
    pub sell_price: u32,
    /// `Bonding`: 1 on pickup, 2 on equip, 3 on use, 4 quest.
    pub bonding: u8,
    pub required_level: u16,
    /// `ExpansionID`.
    pub expansion_id: u8,
    /// `InventoryType` (Retail `Enum.InventoryType`).
    pub inventory_type: u8,
    pub item_level: u16,
    /// `MaxCount`: 1 is "Unique".
    pub max_count: u32,
    /// `Description_lang`, the yellow flavor text.
    pub description: String,
    /// `ContainerSlots` of a bag.
    pub container_slots: u8,
    /// `ItemDelay`: a weapon's swing time in milliseconds.
    pub delay_ms: u32,
    /// `DmgVariance`: the damage spread around the average hit.
    pub damage_variance: f32,
    /// `Flags_1 & ITEM_FLAG2_CASTER_WEAPON` (0x200): the weapon reads the caster damage tables.
    pub caster_weapon: bool,
    /// `StatModifier_bonusStat_N` and `StatPercentEditor_N` of the used stat slots.
    pub stats: Vec<(i8, i32)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ItemCatalog {
    items: HashMap<u32, ItemCatalogEntry>,
    /// `ItemSubClass.DisplayName_lang` by (ClassID, SubClassID).
    subclass_names: HashMap<(u8, u8), String>,
}

impl ItemCatalog {
    pub fn get(&self, item_id: u32) -> Option<&ItemCatalogEntry> {
        self.items.get(&item_id)
    }

    pub fn subclass_name(&self, class_id: u8, subclass_id: u8) -> Option<&str> {
        self.subclass_names
            .get(&(class_id, subclass_id))
            .map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

fn catalog() -> &'static ItemCatalog {
    static CATALOG: OnceLock<ItemCatalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let dir = crate::paths::resolve_data_path(db2_dir(Path::new("")));
        load_item_catalog(&dir).unwrap_or_else(|err| {
            #[cfg(not(godot_host))]
            bevy::log::error!("item catalog unavailable: {err}");
            #[cfg(godot_host)]
            eprintln!("item catalog unavailable: {err}");
            ItemCatalog::default()
        })
    })
}

/// Catalog entry of `item_id`; the catalog loads on first use.
pub fn item_catalog_entry(item_id: u32) -> Option<&'static ItemCatalogEntry> {
    catalog().get(item_id)
}

/// The item's subclass name (`GetItemInfo` itemSubType): "Sword", "Cloth".
pub fn item_subclass_name(entry: &ItemCatalogEntry) -> Option<&'static str> {
    catalog().subclass_name(entry.class_id, entry.subclass_id)
}

/// Load the catalog on a background thread so the first bag or tooltip that needs
/// it does not stall a frame on the ~175k-row ItemSparse parse.
pub fn warm_item_catalog() {
    std::thread::spawn(|| {
        catalog();
    });
}

fn db2_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("db2").join(SPELL_DB2_BUILD)
}

/// `Item.csv`, `ItemSparse.csv` and `ItemSubClass.csv` from one DB2 export directory.
pub fn load_item_catalog(dir: &Path) -> Result<ItemCatalog, String> {
    let mut catalog = parse_item_catalog(&CsvTable::read(&dir.join("Item.csv"))?)?;
    let sparse = CsvTable::read(&dir.join("ItemSparse.csv"))?;
    apply_item_sparse(&mut catalog, &sparse)?;
    apply_item_sparse_stats(&mut catalog, &sparse)?;
    apply_subclass_names(
        &mut catalog,
        &CsvTable::read(&dir.join("ItemSubClass.csv"))?,
    )?;
    Ok(catalog)
}

pub(crate) fn apply_subclass_names(
    catalog: &mut ItemCatalog,
    table: &CsvTable,
) -> Result<(), String> {
    let path = table.path();
    csv_rows(
        table,
        ["ClassID", "SubClassID", "DisplayName_lang"],
        |[class, subclass, name]| {
            catalog.subclass_names.insert(
                (number(class, path)? as u8, number(subclass, path)? as u8),
                name.to_string(),
            );
            Ok(())
        },
    )
}

/// Rows of `table` as the values of `columns`, by header name. ItemSparse
/// descriptions hold quoted newlines, so records follow RFC 4180, not lines.
fn csv_rows<const N: usize>(
    table: &CsvTable,
    columns: [&str; N],
    mut row: impl FnMut([&str; N]) -> Result<(), String>,
) -> Result<(), String> {
    let mut indexes = [0; N];
    for (index, column) in indexes.iter_mut().zip(columns) {
        *index = table.column(column)?;
    }
    for record in table.records() {
        let mut values = [""; N];
        for (value, index) in values.iter_mut().zip(indexes) {
            *value = record
                .get(index)
                .ok_or_else(|| format!("{}: short row {record:?}", table.path().display()))?;
        }
        row(values)?;
    }
    Ok(())
}

fn number(value: &str, path: &Path) -> Result<i64, String> {
    value
        .parse()
        .map_err(|err| format!("{}: bad value {value:?}: {err}", path.display()))
}

pub(crate) fn parse_item_catalog(table: &CsvTable) -> Result<ItemCatalog, String> {
    let path = table.path();
    let mut items = HashMap::new();
    csv_rows(
        table,
        [
            "ID",
            "ClassID",
            "SubclassID",
            "IconFileDataID",
            "SheatheType",
        ],
        |[id, class, subclass, icon, sheathe]| {
            items.insert(
                number(id, path)? as u32,
                ItemCatalogEntry {
                    class_id: number(class, path)? as u8,
                    subclass_id: number(subclass, path)? as u8,
                    icon_fdid: number(icon, path)?.max(0) as u32,
                    sheathe_type: number(sheathe, path)? as u8,
                    ..Default::default()
                },
            );
            Ok(())
        },
    )?;
    Ok(ItemCatalog {
        items,
        subclass_names: HashMap::new(),
    })
}

/// Fill the ItemSparse fields of the catalog's items; sparse rows without an
/// `Item` row are skipped (the client cannot show an item without its icon).
pub(crate) fn apply_item_sparse(catalog: &mut ItemCatalog, table: &CsvTable) -> Result<(), String> {
    const COLUMNS: [&str; 13] = [
        "ID",
        "Display_lang",
        "OverallQualityID",
        "Stackable",
        "SellPrice",
        "Bonding",
        "RequiredLevel",
        "InventoryType",
        "ItemLevel",
        "MaxCount",
        "Description_lang",
        "ContainerSlots",
        "ExpansionID",
    ];
    let path = table.path();
    csv_rows(table, COLUMNS, |values| {
        let [
            id,
            name,
            quality,
            stackable,
            sell,
            bonding,
            level,
            inventory_type,
            ilvl,
            max,
            desc,
            slots,
            expansion,
        ] = values;
        let Some(entry) = catalog.items.get_mut(&(number(id, path)? as u32)) else {
            return Ok(());
        };
        entry.name = name.to_string();
        entry.quality = number(quality, path)?.clamp(0, 8) as u8;
        entry.stackable = number(stackable, path)?.max(1) as u32;
        entry.sell_price = number(sell, path)?.max(0) as u32;
        entry.bonding = number(bonding, path)? as u8;
        entry.required_level = number(level, path)?.max(0) as u16;
        entry.expansion_id = number(expansion, path)?.max(0) as u8;
        entry.inventory_type = number(inventory_type, path)? as u8;
        entry.item_level = number(ilvl, path)?.max(0) as u16;
        entry.max_count = number(max, path)?.max(0) as u32;
        entry.description = desc.to_string();
        entry.container_slots = number(slots, path)?.clamp(0, 255) as u8;
        Ok(())
    })
}

/// `ITEM_FLAG2_CASTER_WEAPON`.
const CASTER_WEAPON_FLAG: i64 = 0x200;
/// `MAX_ITEM_PROTO_STATS`.
const STAT_SLOTS: usize = 10;

/// Fill the weapon and stat ItemSparse fields the tooltip's armor, damage and stat lines read
/// (`item_stats`).
pub(crate) fn apply_item_sparse_stats(
    catalog: &mut ItemCatalog,
    table: &CsvTable,
) -> Result<(), String> {
    let path = table.path();
    let column = |name: &str| table.column(name);
    let (id, delay, variance, flags) = (
        column("ID")?,
        column("ItemDelay")?,
        column("DmgVariance")?,
        column("Flags_1")?,
    );
    let mut stat_columns = [(0, 0); STAT_SLOTS];
    for (slot, columns) in stat_columns.iter_mut().enumerate() {
        *columns = (
            column(&format!("StatModifier_bonusStat_{slot}"))?,
            column(&format!("StatPercentEditor_{slot}"))?,
        );
    }
    for record in table.records() {
        let field = |index: usize| {
            record
                .get(index)
                .ok_or_else(|| format!("{}: short row {record:?}", path.display()))
        };
        let Some(entry) = catalog.items.get_mut(&(number(field(id)?, path)? as u32)) else {
            continue;
        };
        entry.delay_ms = number(field(delay)?, path)?.max(0) as u32;
        entry.damage_variance = field(variance)?
            .parse()
            .map_err(|err| format!("{}: bad DmgVariance: {err}", path.display()))?;
        entry.caster_weapon = number(field(flags)?, path)? & CASTER_WEAPON_FLAG != 0;
        entry.stats.clear();
        for (stat, percent) in stat_columns {
            let (stat, percent) = (number(field(stat)?, path)?, number(field(percent)?, path)?);
            if stat >= 0 && percent != 0 {
                entry.stats.push((stat as i8, percent as i32));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "item_catalog_tests.rs"]
mod tests;
