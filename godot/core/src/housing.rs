//! Read-only, build-qualified housing catalog. Static DB2 data never grants decor.
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use rusqlite::{Connection, OpenFlags, params};

pub const RETAIL_BUILD: &str = "12.1.0.69933";
pub const RETAIL_BUILD_KEY: &str = "dcfc90fffd79ba00406ae46f5f657592";

type Fields = BTreeMap<String, String>;

#[derive(Clone, Debug, PartialEq)]
pub struct Decor {
    pub id: u32,
    pub name: String,
    pub item_id: u32,
    /// Literal DB2 ModelType; no inferred M2/WMO naming.
    pub model_type: u32,
    pub model_fdid: u32,
    pub thumbnail_fdid: u32,
    pub order: i32,
    pub subcategories: Vec<u32>,
    pub sources: Vec<DecorSource>,
}

/// One authored CollectableSourceInfo source of a decor (SourceTypeEnum 3 vendor through
/// CollectableSourceVendorSparse, 1 encounter through CollectableSourceEncounterSparse).
#[derive(Clone, Debug, PartialEq)]
pub enum DecorSource {
    /// The client DB2s do not name most vendors; the server's creature query does.
    Vendor {
        creature_id: u32,
        /// AreaTable.AreaName_lang; AreaTableID 0 names no area.
        area: Option<String>,
    },
    Encounter {
        encounter: String,
        instance: String,
        /// Difficulty.Name_lang; DifficultyID 0 names no difficulty.
        difficulty: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Category {
    pub id: u32,
    pub name: String,
    pub order: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Subcategory {
    pub id: u32,
    pub category_id: u32,
    pub name: String,
    pub order: i32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct HousingCatalog {
    pub decor: Vec<Decor>,
    pub categories: Vec<Category>,
    pub subcategories: Vec<Subcategory>,
    pub encrypted_records: u32,
    pub missing_tables: Vec<String>,
    /// Whether the cache carries the CollectableSource tables at all.
    pub sources_imported: bool,
}

impl HousingCatalog {
    pub fn load(path: &Path) -> Result<Self, String> {
        let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|error| format!("Housing catalog {}: {error}", path.display()))?;
        load_connection(&db, RETAIL_BUILD, RETAIL_BUILD_KEY)
    }
}

fn rows(db: &Connection, key: &str, table: &str) -> Result<Vec<(u32, Fields)>, String> {
    let mut query = db
        .prepare("SELECT record_id, fields_json FROM housing_catalog_record WHERE build_key=? AND table_name=? ORDER BY record_id")
        .map_err(|error| error.to_string())?;
    let records = query
        .query_map(params![key, table], |row| {
            Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| error.to_string())?;
    records
        .map(|record| {
            let (id, json) = record.map_err(|error| error.to_string())?;
            let fields: Fields =
                serde_json::from_str(&json).map_err(|error| format!("{table} {id}: {error}"))?;
            Ok((id, fields))
        })
        .collect()
}

fn field<'a>(fields: &'a Fields, name: &str) -> Result<&'a str, String> {
    fields
        .get(name)
        .map(String::as_str)
        .ok_or_else(|| format!("Housing catalog lacks {name}"))
}

fn number<T: std::str::FromStr>(fields: &Fields, name: &str) -> Result<T, String> {
    field(fields, name)?
        .parse()
        .map_err(|_| format!("Housing catalog has invalid {name}"))
}

pub(crate) fn load_connection(
    db: &Connection,
    build: &str,
    key: &str,
) -> Result<HousingCatalog, String> {
    let actual: String = db
        .query_row(
            "SELECT build FROM housing_catalog_build WHERE build_key=?",
            [key],
            |row| row.get(0),
        )
        .map_err(|error| format!("Housing build {key}: {error}"))?;
    if actual != build {
        return Err(format!("Housing catalog build {actual}, expected {build}"));
    }
    let mut catalog = HousingCatalog::default();
    let mut decor_indexes = HashMap::new();
    for (id, fields) in rows(db, key, "HouseDecor")? {
        decor_indexes.insert(id, catalog.decor.len());
        catalog.decor.push(Decor {
            id,
            name: field(&fields, "Name_lang")?.into(),
            item_id: number(&fields, "ItemID")?,
            model_type: number(&fields, "ModelType")?,
            model_fdid: number(&fields, "ModelFileDataID")?,
            thumbnail_fdid: number(&fields, "ThumbnailFileDataID")?,
            order: number(&fields, "OrderIndex")?,
            subcategories: Vec::new(),
            sources: Vec::new(),
        });
    }
    for (id, fields) in rows(db, key, "DecorCategory")? {
        catalog.categories.push(Category {
            id,
            name: field(&fields, "Name_lang")?.into(),
            order: number(&fields, "OrderIndex")?,
        });
    }
    for (id, fields) in rows(db, key, "DecorSubcategory")? {
        catalog.subcategories.push(Subcategory {
            id,
            name: field(&fields, "Name_lang")?.into(),
            category_id: number(&fields, "DecorCategoryID")?,
            order: number(&fields, "OrderIndex")?,
        });
    }
    for (_, fields) in rows(db, key, "DecorXDecorSubcategory")? {
        let decor_id = number::<u32>(&fields, "HouseDecorID")?;
        let subcategory = number::<u32>(&fields, "DecorSubcategoryID")?;
        // The export explicitly omits encrypted decor rows. Readable relationship
        // rows may still refer to them; this join cannot expose an unreadable entry.
        if let Some(&index) = decor_indexes.get(&decor_id) {
            catalog.decor[index].subcategories.push(subcategory);
        }
    }
    read_sources(db, key, &mut catalog, &decor_indexes)?;
    catalog.decor.sort_by_key(|decor| (decor.order, decor.id));
    catalog
        .categories
        .sort_by_key(|category| (category.order, category.id));
    catalog
        .subcategories
        .sort_by_key(|subcategory| (subcategory.order, subcategory.id));
    read_gaps(db, key, &mut catalog)?;
    Ok(catalog)
}

fn names(
    db: &Connection,
    key: &str,
    table: &str,
    column: &str,
) -> Result<HashMap<u32, String>, String> {
    rows(db, key, table)?
        .into_iter()
        .map(|(id, fields)| Ok((id, field(&fields, column)?.to_string())))
        .collect()
}

fn named(names: &HashMap<u32, String>, id: u32, table: &str) -> Result<String, String> {
    names
        .get(&id)
        .cloned()
        .ok_or_else(|| format!("Housing source references missing {table} {id}"))
}

/// ID 0 names nothing; any other ID must resolve.
fn optional_name(
    names: &HashMap<u32, String>,
    id: u32,
    table: &str,
) -> Result<Option<String>, String> {
    (id != 0).then(|| named(names, id, table)).transpose()
}

/// CollectableSourceInfo rows of each decor, joined to their vendor/encounter details in
/// (source info, sparse row) order. Duplicate vendor/area pairs from a second source-info
/// row are listed once. Decor rows the export could not read stay unlisted.
fn read_sources(
    db: &Connection,
    key: &str,
    catalog: &mut HousingCatalog,
    decor_indexes: &HashMap<u32, usize>,
) -> Result<(), String> {
    catalog.sources_imported = db
        .query_row(
            "SELECT COUNT(*) FROM housing_catalog_source WHERE build_key=? AND table_name='CollectableSourceInfo'",
            [key],
            |row| row.get::<_, u32>(0),
        )
        .map_err(|error| error.to_string())?
        > 0;
    if !catalog.sources_imported {
        return Ok(());
    }
    let areas = names(db, key, "AreaTable", "AreaName_lang")?;
    let instances = names(db, key, "JournalInstance", "Name_lang")?;
    let difficulties = names(db, key, "Difficulty", "Name_lang")?;
    let mut encounters = HashMap::new();
    for (id, fields) in rows(db, key, "JournalEncounter")? {
        let instance = number::<u32>(&fields, "JournalInstanceID")?;
        encounters.insert(id, (field(&fields, "Name_lang")?.to_string(), instance));
    }
    let mut decor_of_info = HashMap::new();
    for (id, fields) in rows(db, key, "CollectableSourceInfo")? {
        decor_of_info.insert(id, number::<u32>(&fields, "HouseDecorID")?);
    }
    let mut sources = Vec::new();
    for (id, fields) in rows(db, key, "CollectableSourceVendorSparse")? {
        let info = number::<u32>(&fields, "CollectableSourceInfoID")?;
        let source = DecorSource::Vendor {
            creature_id: number(&fields, "CreatureID")?,
            area: optional_name(&areas, number(&fields, "AreaTableID")?, "AreaTable")?,
        };
        sources.push((info, id, source));
    }
    for (id, fields) in rows(db, key, "CollectableSourceEncounterSparse")? {
        let info = number::<u32>(&fields, "CollectableSourceInfoID")?;
        let encounter_id = number::<u32>(&fields, "JournalEncounterID")?;
        let (encounter, instance) = encounters.get(&encounter_id).ok_or_else(|| {
            format!("Housing source references missing JournalEncounter {encounter_id}")
        })?;
        let source = DecorSource::Encounter {
            encounter: encounter.clone(),
            instance: named(&instances, *instance, "JournalInstance")?,
            difficulty: optional_name(
                &difficulties,
                number(&fields, "DifficultyID")?,
                "Difficulty",
            )?,
        };
        sources.push((info, id, source));
    }
    sources.sort_by_key(|(info, id, _)| (*info, *id));
    for (info, _, source) in sources {
        let decor_id = decor_of_info.get(&info).ok_or_else(|| {
            format!("Housing source references missing CollectableSourceInfo {info}")
        })?;
        if let Some(&index) = decor_indexes.get(decor_id) {
            let listed = &mut catalog.decor[index].sources;
            if !listed.contains(&source) {
                listed.push(source);
            }
        }
    }
    Ok(())
}

fn read_gaps(db: &Connection, key: &str, catalog: &mut HousingCatalog) -> Result<(), String> {
    let mut query = db
        .prepare("SELECT table_name, status, encrypted_records_dropped FROM housing_catalog_source WHERE build_key=? ORDER BY table_name")
        .map_err(|error| error.to_string())?;
    let rows = query
        .query_map([key], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, u32>(2)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    for row in rows {
        let (name, status, encrypted) = row.map_err(|error| error.to_string())?;
        catalog.encrypted_records += encrypted;
        if status != "exported" {
            catalog.missing_tables.push(name);
        }
    }
    Ok(())
}
