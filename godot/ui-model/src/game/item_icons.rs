//! Item icon FileDataIDs are indexed from `Item.csv`, `ItemModifiedAppearance` (item →
//! appearance, lowest `OrderIndex`) and `ItemAppearance.DefaultIconFileDataID`. Items without
//! an appearance (trade goods, consumables) use `Item.IconFileDataID`. This small index loads
//! before the full background `ItemSparse` catalog, so equipped icons do not wait for names.

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::OnceLock;

use shared::item_data::ItemDefinitionSource;

use crate::csv_util::{header_index, parse_csv_line};
use crate::spell_catalog::SPELL_DB2_BUILD;

#[derive(Debug)]
struct SourceItemIconCatalogs {
    retail: HashMap<u32, u32>,
    forever: Result<HashMap<u32, u32>, String>,
}

static ICON_CATALOG: OnceLock<Result<SourceItemIconCatalogs, String>> = OnceLock::new();

/// Load item icon tables before the full catalog's background `ItemSparse` parse.
pub(crate) fn warm_item_icon_catalog() {
    let _ = source_item_icon_catalog();
}

/// The item's Retail icon FileDataID, independent of the full item catalog.
pub fn item_icon_fdid(item_id: u32) -> Option<u32> {
    item_icon_fdid_for(ItemDefinitionSource::Retail, item_id)
}

pub fn item_icon_fdid_for(source: ItemDefinitionSource, item_id: u32) -> Option<u32> {
    let catalog = source_item_icon_catalog()?;
    match source {
        ItemDefinitionSource::Retail => catalog.retail.get(&item_id).copied(),
        ItemDefinitionSource::Forever70205 => catalog.forever.as_ref().ok()?.get(&item_id).copied(),
    }
}

fn source_item_icon_catalog() -> Option<&'static SourceItemIconCatalogs> {
    ICON_CATALOG
        .get_or_init(|| {
            load_source_item_icon_catalog().map_err(|error| {
                eprintln!("item icon catalog unavailable: {error}");
                error
            })
        })
        .as_ref()
        .ok()
}

fn load_source_item_icon_catalog() -> Result<SourceItemIconCatalogs, String> {
    let data_dir = crate::paths::resolve_data_path("");
    let retail_item_dir = data_dir.join("db2").join(SPELL_DB2_BUILD);
    let retail = load_item_icon_map_from(&retail_item_dir, &data_dir)?;
    let forever_dir = data_dir.join(crate::item_catalog::FOREVER_ITEM_DIR);
    let forever = load_item_icon_map_from(&forever_dir, &forever_dir);
    if let Err(error) = &forever {
        eprintln!("Forever70205 item icon catalog unavailable: {error}");
    }
    Ok(SourceItemIconCatalogs { retail, forever })
}

pub(crate) fn load_item_icons() -> Result<HashMap<u32, u32>, String> {
    load_item_icons_from(&crate::paths::resolve_data_path(""))
}

pub(crate) fn load_item_icons_from(dir: &Path) -> Result<HashMap<u32, u32>, String> {
    let appearance_icons = read_columns(
        &dir.join("ItemAppearance.csv"),
        ["ID", "DefaultIconFileDataID", "ID"],
    )?;
    let appearance_icon: HashMap<u32, u32> = appearance_icons
        .into_iter()
        .map(|[id, icon, _]| (id, icon))
        .collect();
    let modified = read_columns(
        &dir.join("ItemModifiedAppearance.csv"),
        ["ItemID", "ItemAppearanceID", "OrderIndex"],
    )?;
    Ok(item_icons(modified, &appearance_icon))
}

fn load_item_icon_map_from(
    item_dir: &Path,
    appearance_dir: &Path,
) -> Result<HashMap<u32, u32>, String> {
    let item_rows = read_columns(&item_dir.join("Item.csv"), ["ID", "IconFileDataID", "ID"])?;
    let mut icons: HashMap<u32, u32> = item_rows
        .into_iter()
        .filter_map(|[item_id, icon, _]| (icon != 0).then_some((item_id, icon)))
        .collect();
    icons.extend(load_item_icons_from(appearance_dir)?);
    Ok(icons)
}

/// Keeps each item's lowest-`OrderIndex` appearance that has an icon.
fn item_icons(modified: Vec<[u32; 3]>, appearance_icon: &HashMap<u32, u32>) -> HashMap<u32, u32> {
    let mut best: HashMap<u32, (u32, u32)> = HashMap::new();
    for [item_id, appearance_id, order] in modified {
        let Some(&icon) = appearance_icon
            .get(&appearance_id)
            .filter(|icon| **icon != 0)
        else {
            continue;
        };
        let entry = best.entry(item_id).or_insert((order, icon));
        if order < entry.0 {
            *entry = (order, icon);
        }
    }
    best.into_iter()
        .map(|(item, (_, icon))| (item, icon))
        .collect()
}

fn read_columns(path: &Path, columns: [&str; 3]) -> Result<Vec<[u32; 3]>, String> {
    let file =
        std::fs::File::open(path).map_err(|err| format!("open {}: {err}", path.display()))?;
    let mut lines = BufReader::new(file).lines();
    let header = lines
        .next()
        .ok_or_else(|| format!("{} is empty", path.display()))?
        .map_err(|err| format!("read {}: {err}", path.display()))?;
    let headers = parse_csv_line(&header);
    let indexes = [
        header_index(&headers, columns[0], path)?,
        header_index(&headers, columns[1], path)?,
        header_index(&headers, columns[2], path)?,
    ];
    let mut rows = Vec::new();
    for (line_number, line) in lines.enumerate() {
        let line = line.map_err(|err| format!("read {}: {err}", path.display()))?;
        let fields = parse_csv_line(&line);
        let value = |index| {
            game_engine_core::csv_util::parse_field::<u32>(&fields, index)
                .map_err(|error| format!("{}:{}: {error}", path.display(), line_number + 2))
        };
        rows.push([value(indexes[0])?, value(indexes[1])?, value(indexes[2])?]);
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowest_order_index_with_an_icon_wins() {
        let appearance_icon = HashMap::from([(10, 135_274), (11, 0), (12, 134_948)]);
        let icons = item_icons(
            vec![[2055, 12, 1], [2055, 10, 0], [2057, 11, 0], [2057, 12, 2]],
            &appearance_icon,
        );
        assert_eq!(icons.get(&2055), Some(&135_274));
        assert_eq!(
            icons.get(&2057),
            Some(&134_948),
            "icon-less appearance skipped"
        );
    }

    #[test]
    fn brotherhood_of_thieves_reward_resolves_from_retail_tables() {
        // Brotherhood of Thieves (18) choice reward 5580.
        // ItemModifiedAppearance 2132 → ItemAppearance 1885.
        assert_eq!(
            crate::item_catalog::wait_for_item_catalog().icon_fdid(5580),
            Some(133_057)
        );
    }

    #[test]
    fn items_without_an_appearance_use_the_item_icon() {
        // Linen Cloth 2589 has no ItemModifiedAppearance row; Item.IconFileDataID 132889.
        assert_eq!(
            crate::item_catalog::wait_for_item_catalog().icon_fdid(2589),
            Some(132_889)
        );
    }
}

#[cfg(test)]
mod db2_header_tests {
    use super::*;
    #[test]
    fn db2_header_item_icons_reject_invalid_canonical_row() {
        let fixture = crate::db2_test_support::ExportFixture::copy("ItemAppearance", Some("ID"));
        let error = read_columns(&fixture.0, ["ID", "DefaultIconFileDataID", "ID"]).unwrap_err();
        assert!(error.contains("ItemAppearance.csv:2:"), "{error}");
    }
    #[test]
    fn db2_header_item_icons_read_canonical_count() {
        let fixture = crate::db2_test_support::ExportFixture::copy("ItemAppearance", None);
        let rows = read_columns(&fixture.0, ["ID", "DefaultIconFileDataID", "ID"]).unwrap();
        assert_eq!(
            rows.len(),
            std::fs::read_to_string(&fixture.0).unwrap().lines().count() - 1
        );
    }
}
