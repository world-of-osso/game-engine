//! Item icon FileDataIDs: `ItemModifiedAppearance` (item → appearance, lowest
//! `OrderIndex`) joined with `ItemAppearance.DefaultIconFileDataID`, as Retail
//! `C_Item.GetItemIconByID` resolves the base appearance. Items without an
//! appearance (trade goods, consumables) use `Item.IconFileDataID`.

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::csv_util::{header_index, parse_csv_line};

/// The item's icon; `None` while the item catalog loads (see `ItemCatalog::icon_fdid`).
pub fn item_icon_fdid(item_id: u32) -> Option<u32> {
    crate::item_catalog::item_catalog()?.icon_fdid(item_id)
}

pub fn item_icon_fdid_for(
    source: shared::item_data::ItemDefinitionSource,
    item_id: u32,
) -> Option<u32> {
    crate::item_catalog::item_catalog_for(source)?.icon_fdid(item_id)
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
    for line in lines {
        let line = line.map_err(|err| format!("read {}: {err}", path.display()))?;
        let fields = parse_csv_line(&line);
        let value = |index: usize| fields.get(index).and_then(|v| v.parse::<u32>().ok());
        if let (Some(a), Some(b), Some(c)) =
            (value(indexes[0]), value(indexes[1]), value(indexes[2]))
        {
            rows.push([a, b, c]);
        }
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
