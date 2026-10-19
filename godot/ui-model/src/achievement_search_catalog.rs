//! Static installed DB2 ancestry only; completion/progress always comes from received rows.
use crate::csv_records::CsvTable;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::OnceLock;

struct SearchCatalog {
    achievements: BTreeMap<u32, u32>,
    categories: BTreeMap<u32, (i32, String)>,
}

impl SearchCatalog {
    fn load(dir: &Path) -> Result<Self, String> {
        let table = CsvTable::read(&dir.join("Achievement.csv"))?;
        let id = table.column("ID")?;
        let category = table.column("Category")?;
        let mut achievements = BTreeMap::new();
        for row in table.records() {
            achievements.insert(number(&row, id)?, number(&row, category)?);
        }
        let table = CsvTable::read(&dir.join("Achievement_Category.csv"))?;
        let id = table.column("ID")?;
        let parent = table.column("Parent")?;
        let name = table.column("Name_lang")?;
        let mut categories = BTreeMap::new();
        for row in table.records() {
            let title = row
                .get(name)
                .ok_or("Achievement_Category missing Name_lang")?;
            categories.insert(
                number(&row, id)?,
                (number(&row, parent)?, title.to_string()),
            );
        }
        Ok(Self {
            achievements,
            categories,
        })
    }

    fn path(&self, achievement: u32) -> Result<String, String> {
        let mut category = *self.achievements.get(&achievement).ok_or_else(|| {
            format!("Achievement.csv has no category for achievement {achievement}")
        })?;
        let mut names = Vec::new();
        let mut seen = BTreeSet::new();
        loop {
            if !seen.insert(category) {
                return Err(format!("Achievement_Category cycle at {category}"));
            }
            let (parent, name) = self
                .categories
                .get(&category)
                .ok_or_else(|| format!("Achievement_Category missing category {category}"))?;
            names.push(name.as_str());
            if *parent == -1 {
                break;
            }
            category = u32::try_from(*parent)
                .map_err(|_| format!("Achievement_Category {category} invalid parent {parent}"))?;
        }
        names.reverse();
        Ok(names.join(" > "))
    }
}

fn number<T: std::str::FromStr>(
    row: &[std::borrow::Cow<'_, str>],
    index: usize,
) -> Result<T, String> {
    let value = row
        .get(index)
        .ok_or_else(|| format!("Achievement metadata missing column {index}"))?;
    value
        .parse()
        .map_err(|_| format!("Achievement metadata column {index} invalid number {value:?}"))
}

pub(super) fn category_path(achievement: u32) -> String {
    static CATALOG: OnceLock<SearchCatalog> = OnceLock::new();
    CATALOG
        .get_or_init(|| {
            SearchCatalog::load(&crate::paths::resolve_data_path("db2/active"))
                .expect("load installed achievement search ancestry")
        })
        .path(achievement)
        .expect("resolve received achievement search ancestry")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn achievement_search_catalog_uses_installed_root_and_nested_ancestry() {
        let catalog = SearchCatalog::load(&crate::paths::resolve_data_path("db2/active")).unwrap();
        assert_eq!(catalog.path(6).unwrap(), "Characters");
        assert_eq!(catalog.path(633).unwrap(), "Dungeons & Raids > Classic");
        assert!(catalog.path(u32::MAX).unwrap_err().contains("no category"));
    }
}
