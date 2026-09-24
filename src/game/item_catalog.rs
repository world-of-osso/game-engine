//! Item class and icon per item ID from `Item.csv` (Retail `Item` DB2): the auction
//! house protocol carries only item ID, name and quality, so the client resolves the
//! icon (`IconFileDataID`) and the browse category (`ClassID`) itself.

use std::collections::HashMap;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::csv_util::{header_index, parse_csv_line};
use crate::spell_catalog::SPELL_DB2_BUILD;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemCatalogEntry {
    pub class_id: u8,
    pub subclass_id: u8,
    pub icon_fdid: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ItemCatalog {
    items: HashMap<u32, ItemCatalogEntry>,
}

impl ItemCatalog {
    pub fn get(&self, item_id: u32) -> Option<ItemCatalogEntry> {
        self.items.get(&item_id).copied()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// Catalog entry of `item_id`; the catalog loads on first use.
pub fn item_catalog_entry(item_id: u32) -> Option<ItemCatalogEntry> {
    static CATALOG: OnceLock<ItemCatalog> = OnceLock::new();
    CATALOG
        .get_or_init(|| {
            let path = crate::paths::resolve_data_path(item_csv_path(Path::new("")));
            load_item_catalog(&path).unwrap_or_else(|err| {
                bevy::log::error!("item catalog unavailable: {err}");
                ItemCatalog::default()
            })
        })
        .get(item_id)
}

pub fn item_csv_path(data_dir: &Path) -> PathBuf {
    data_dir.join("db2").join(SPELL_DB2_BUILD).join("Item.csv")
}

pub fn load_item_catalog(path: &Path) -> Result<ItemCatalog, String> {
    let file =
        std::fs::File::open(path).map_err(|err| format!("open {}: {err}", path.display()))?;
    parse_item_catalog(std::io::BufReader::new(file), path)
}

pub fn parse_item_catalog<R: BufRead>(reader: R, path: &Path) -> Result<ItemCatalog, String> {
    let mut lines = reader.lines();
    let header = lines
        .next()
        .ok_or_else(|| format!("{} is empty", path.display()))?
        .map_err(|err| format!("read {} header: {err}", path.display()))?;
    let headers = parse_csv_line(&header);
    let columns = [
        header_index(&headers, "ID", path)?,
        header_index(&headers, "ClassID", path)?,
        header_index(&headers, "SubclassID", path)?,
        header_index(&headers, "IconFileDataID", path)?,
    ];
    let mut items = HashMap::new();
    for line in lines {
        let line = line.map_err(|err| format!("read {}: {err}", path.display()))?;
        let fields = parse_csv_line(&line);
        let field = |index: usize| {
            fields
                .get(columns[index])
                .ok_or_else(|| format!("{}: short row {line:?}", path.display()))
        };
        let parse = |index: usize| -> Result<i64, String> {
            let value = field(index)?;
            value
                .parse()
                .map_err(|err| format!("{}: bad value {value:?}: {err}", path.display()))
        };
        items.insert(
            parse(0)? as u32,
            ItemCatalogEntry {
                class_id: parse(1)? as u8,
                subclass_id: parse(2)? as u8,
                icon_fdid: parse(3)?.max(0) as u32,
            },
        );
    }
    Ok(ItemCatalog { items })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ITEM_CSV: &str = "ID,ClassID,SubclassID,Material,InventoryType,SheatheType,Sound_override_subclassID,IconFileDataID,ItemGroupSoundsID\n\
        2447,7,9,7,0,0,-1,133939,23\n\
        2589,7,5,8,0,0,-1,132889,7\n\
        25,2,7,1,21,3,-1,135274,0\n";

    #[test]
    fn parses_class_and_icon_by_item_id() {
        let catalog = parse_item_catalog(ITEM_CSV.as_bytes(), Path::new("Item.csv")).unwrap();

        assert_eq!(catalog.len(), 3);
        assert_eq!(
            catalog.get(2589),
            Some(ItemCatalogEntry {
                class_id: 7,
                subclass_id: 5,
                icon_fdid: 132889,
            })
        );
        assert_eq!(catalog.get(25).map(|item| item.class_id), Some(2));
        assert_eq!(catalog.get(9999), None);
    }

    #[test]
    fn missing_icon_column_is_an_error() {
        let error = parse_item_catalog("ID,ClassID,SubclassID\n1,2,3\n".as_bytes(), Path::new("x"))
            .unwrap_err();

        assert!(error.contains("IconFileDataID"), "{error}");
    }
}
