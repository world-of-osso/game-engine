//! Literal DataTag values are distinct from DataTagInfo record IDs.
use super::{field, number, rows};
use rusqlite::Connection;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq)]
pub struct DataTagInfo {
    pub record_id: u32,
    pub value: u32,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DataTagGroup {
    pub name: String,
    pub values: BTreeSet<u32>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CatalogTags {
    /// All four source receipts must be exported before local actions are available.
    pub available: bool,
    pub info: Vec<DataTagInfo>,
    /// Keyed by the literal DataTagGroup record ID; no claimed Retail menu ordering.
    pub groups: BTreeMap<u32, DataTagGroup>,
    /// Literal DataTagXHouseDecorRecord.DataTagID values by decor record ID.
    pub decor: BTreeMap<u32, BTreeSet<u32>>,
}

pub(super) fn load(db: &Connection, key: &str) -> Result<CatalogTags, String> {
    let available = db
        .query_row(
            "SELECT COUNT(*) FROM housing_catalog_source WHERE build_key=? AND status='exported' AND table_name IN ('DataTagInfo','DataTagGroup','DataTagGroupXDataTag','DataTagXHouseDecorRecord')",
            [key],
            |row| row.get::<_, u32>(0),
        )
        .map_err(|error| format!("Housing tag source receipts: {error}"))?
        == 4;
    let info = rows(db, key, "DataTagInfo")?
        .into_iter()
        .map(|(record_id, fields)| {
            Ok(DataTagInfo {
                record_id,
                value: number(&fields, "DataTag")?,
                name: field(&fields, "Description_lang")?.into(),
            })
        })
        .collect::<Result<_, String>>()?;
    Ok(CatalogTags {
        available,
        info,
        groups: load_groups(db, key)?,
        decor: load_decor_tags(db, key)?,
    })
}

fn load_groups(db: &Connection, key: &str) -> Result<BTreeMap<u32, DataTagGroup>, String> {
    let mut groups: BTreeMap<_, _> = rows(db, key, "DataTagGroup")?
        .into_iter()
        .map(|(id, fields)| {
            Ok((
                id,
                DataTagGroup {
                    name: field(&fields, "Description_lang")?.into(),
                    values: BTreeSet::new(),
                },
            ))
        })
        .collect::<Result<_, String>>()?;
    for (_, fields) in rows(db, key, "DataTagGroupXDataTag")? {
        let id = number::<u32>(&fields, "DataTagGroupID")?;
        let group = groups.get_mut(&id).ok_or_else(|| {
            format!("Housing tag membership references missing DataTagGroup {id}")
        })?;
        group.values.insert(number(&fields, "DataTag")?);
    }
    Ok(groups)
}

fn load_decor_tags(db: &Connection, key: &str) -> Result<BTreeMap<u32, BTreeSet<u32>>, String> {
    let mut decor = BTreeMap::<u32, BTreeSet<u32>>::new();
    for (_, fields) in rows(db, key, "DataTagXHouseDecorRecord")? {
        let id = number(&fields, "HouseDecorID")?;
        decor
            .entry(id)
            .or_default()
            .insert(number(&fields, "DataTagID")?);
    }
    Ok(decor)
}
