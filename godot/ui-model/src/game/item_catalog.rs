//! Independent Retail and Forever70205 item catalogs, keyed by authored item ID
//! within each source. Item/ItemSparse, appearance icons and subclass names come
//! from source-local CSVs. Owned stacks carry source and ID; no race-based lookup.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Once, OnceLock};

use shared::item_data::ItemDefinitionSource;

use crate::spell_catalog::SPELL_DB2_BUILD;

pub const FOREVER_ITEM_DIR: &str = "db2/1.60.1.70205/items";
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
    /// Retail Item.ItemSquishEraID; pinned Forever has no modern squish.
    pub squish_era: u8,
    /// Socket penalties aligned to the used stat allocations.
    pub stat_socket_multipliers: [f32; 10],
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
    /// Each item's base-appearance icon (`item_icons`).
    appearance_icons: HashMap<u32, u32>,
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

    /// `C_Item.GetItemIconByID`: the base appearance's icon; items without an appearance
    /// (trade goods, consumables) use `Item.IconFileDataID`.
    pub fn icon_fdid(&self, item_id: u32) -> Option<u32> {
        self.appearance_icons.get(&item_id).copied().or_else(|| {
            self.get(item_id)
                .map(|entry| entry.icon_fdid)
                .filter(|icon| *icon != 0)
        })
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// Independent namespaces; a failed Forever load never selects Retail.
#[derive(Debug)]
pub struct SourceItemCatalogs {
    retail: ItemCatalog,
    forever: Result<ItemCatalog, String>,
}

impl SourceItemCatalogs {
    pub fn new(retail: ItemCatalog, forever: Result<ItemCatalog, String>) -> Self {
        Self { retail, forever }
    }

    pub fn catalog(&self, source: ItemDefinitionSource) -> Result<&ItemCatalog, &str> {
        match source {
            ItemDefinitionSource::Retail => Ok(&self.retail),
            ItemDefinitionSource::Forever70205 => self.forever.as_ref().map_err(String::as_str),
        }
    }

    pub fn entry(
        &self,
        source: ItemDefinitionSource,
        item_id: u32,
    ) -> Result<&ItemCatalogEntry, String> {
        let catalog = self
            .catalog(source)
            .map_err(|error| format!("{source:?} item {item_id}: {error}"))?;
        catalog
            .get(item_id)
            .ok_or_else(|| format!("{source:?} item {item_id}: definition absent"))
    }
}

static CATALOG: OnceLock<SourceItemCatalogs> = OnceLock::new();

/// The catalog once its background load is done, `None` until then: as Retail's
/// `C_Item.GetItemInfo` returns nil until `GET_ITEM_INFO_RECEIVED`, no caller waits for
/// the ~175k-row ItemSparse parse. The first call starts the load.
pub fn item_catalog() -> Option<&'static ItemCatalog> {
    warm_item_catalog();
    item_catalog_for(ItemDefinitionSource::Retail)
}

pub fn item_catalogs() -> Option<&'static SourceItemCatalogs> {
    warm_item_catalog();
    CATALOG.get()
}

pub fn item_catalog_for(source: ItemDefinitionSource) -> Option<&'static ItemCatalog> {
    item_catalogs()?.catalog(source).ok()
}

pub fn item_catalog_entry_for(
    source: ItemDefinitionSource,
    item_id: u32,
) -> Option<&'static ItemCatalogEntry> {
    item_catalog_for(source)?.get(item_id)
}

pub fn require_item_catalog_entry(
    source: ItemDefinitionSource,
    item_id: u32,
) -> Result<&'static ItemCatalogEntry, String> {
    warm_item_catalog();
    let catalogs = CATALOG
        .get()
        .ok_or_else(|| format!("{source:?} item {item_id}: catalog still loading"))?;
    catalogs.entry(source, item_id)
}

/// Catalog entry of `item_id`; `None` while the catalog loads.
pub fn item_catalog_entry(item_id: u32) -> Option<&'static ItemCatalogEntry> {
    item_catalog()?.get(item_id)
}

/// The item's subclass name (`GetItemInfo` itemSubType): "Sword", "Cloth".
pub fn item_subclass_name(entry: &ItemCatalogEntry) -> Option<&'static str> {
    item_subclass_name_for(ItemDefinitionSource::Retail, entry)
}

pub fn item_subclass_name_for(
    source: ItemDefinitionSource,
    entry: &ItemCatalogEntry,
) -> Option<&'static str> {
    item_catalog_for(source)?.subclass_name(entry.class_id, entry.subclass_id)
}

/// Start loading the catalog on a background thread, once.
pub fn warm_item_catalog() {
    static STARTED: Once = Once::new();
    STARTED.call_once(|| {
        std::thread::Builder::new()
            .name("item-catalog".into())
            .spawn(|| {
                CATALOG.get_or_init(|| {
                    let retail = load_client_catalog();
                    let dir = crate::paths::resolve_data_path(FOREVER_ITEM_DIR);
                    let forever = load_item_catalog_for(&dir, ItemDefinitionSource::Forever70205)
                        .and_then(|mut catalog| {
                            catalog.appearance_icons =
                                crate::item_icons::load_item_icons_from(&dir)?;
                            validate_named_items(&catalog, &dir)?;
                            Ok(catalog)
                        });
                    if let Err(error) = &forever {
                        eprintln!("Forever70205 item catalog unavailable: {error}");
                    }
                    SourceItemCatalogs::new(retail, forever)
                });
            })
            .expect("spawn the item catalog loader");
    });
}

/// The catalog, waiting for its load. For tests and offline tools: a frame never waits.
pub fn wait_for_item_catalog() -> &'static ItemCatalog {
    warm_item_catalog();
    &CATALOG.wait().retail
}

fn validate_named_items(catalog: &ItemCatalog, dir: &Path) -> Result<(), String> {
    for (id, entry) in &catalog.items {
        if entry.name.is_empty() {
            return Err(format!(
                "{}: Forever70205 item {id} has no ItemSparse definition",
                dir.display()
            ));
        }
    }
    Ok(())
}

fn load_client_catalog() -> ItemCatalog {
    let started = std::time::Instant::now();
    let dir = crate::paths::resolve_data_path(db2_dir(Path::new("")));
    let loaded = load_item_catalog(&dir).and_then(|mut catalog| {
        catalog.appearance_icons = crate::item_icons::load_item_icons()?;
        Ok(catalog)
    });
    if let Ok(catalog) = &loaded {
        let (items, seconds) = (catalog.len(), started.elapsed().as_secs_f32());
        println!("item catalog: {items} items loaded in {seconds:.1} s");
    }
    loaded.unwrap_or_else(|err| {
        eprintln!("item catalog unavailable: {err}");
        ItemCatalog::default()
    })
}

fn db2_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("db2").join(SPELL_DB2_BUILD)
}

/// `Item.csv`, `ItemSparse.csv` and `ItemSubClass.csv` from one DB2 export directory.
pub fn load_item_catalog(dir: &Path) -> Result<ItemCatalog, String> {
    load_item_catalog_for(dir, ItemDefinitionSource::Retail)
}

fn load_item_catalog_for(dir: &Path, source: ItemDefinitionSource) -> Result<ItemCatalog, String> {
    let items = CsvTable::read(&dir.join("Item.csv"))?;
    let mut catalog = parse_item_catalog(&items)?;
    let sparse = CsvTable::read(&dir.join("ItemSparse.csv"))?;
    apply_item_sparse(&mut catalog, &sparse)?;
    apply_item_sparse_stats(&mut catalog, &sparse)?;
    if source == ItemDefinitionSource::Retail {
        apply_retail_scaling_fields(&mut catalog, &items, &sparse)?;
    }
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
        ..Default::default()
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

/// Modern scaling columns are mandatory for Retail, absent in pinned Forever.
fn apply_retail_scaling_fields(
    catalog: &mut ItemCatalog,
    items: &CsvTable,
    sparse: &CsvTable,
) -> Result<(), String> {
    csv_rows(items, ["ID", "ItemSquishEraID"], |[id, era]| {
        if let Some(entry) = catalog.items.get_mut(&(number(id, items.path())? as u32)) {
            entry.squish_era = number(era, items.path())? as u8;
        }
        Ok(())
    })?;
    let columns: [String; 31] = std::array::from_fn(|index| {
        if index == 0 {
            return "ID".into();
        }
        let slot = (index - 1) / 3;
        let prefix = [
            "StatModifier_bonusStat",
            "StatPercentEditor",
            "StatPercentageOfSocket",
        ][(index - 1) % 3];
        format!("{prefix}_{slot}")
    });
    csv_rows(sparse, columns.each_ref().map(String::as_str), |values| {
        let Some(entry) = catalog
            .items
            .get_mut(&(number(values[0], sparse.path())? as u32))
        else {
            return Ok(());
        };
        let mut used = 0;
        for triple in values[1..].chunks_exact(3) {
            if number(triple[0], sparse.path())? >= 0 && number(triple[1], sparse.path())? != 0 {
                entry.stat_socket_multipliers[used] = triple[2].parse().map_err(|error| {
                    format!(
                        "{}: bad socket multiplier: {error}",
                        sparse.path().display()
                    )
                })?;
                used += 1;
            }
        }
        Ok(())
    })
}

#[cfg(test)]
#[path = "item_catalog_tests.rs"]
mod tests;
