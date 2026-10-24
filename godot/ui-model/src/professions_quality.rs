//! Authored item crafting tier and inventory overlay; never an output-quality formula.
use super::professions_catalog::{number, read_rows};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CraftingQuality {
    pub tier: u32,
    pub inventory_atlas: String,
    pub mixed_inventory_atlas: String,
}

fn load_inventory_atlases(dir: &Path) -> Result<BTreeMap<u32, (String, String)>, String> {
    let mut names = BTreeMap::new();
    read_rows(dir, "UiTextureAtlasElement", &["ID", "Name"], |row| {
        names.insert(number(row[0])?, row[1].to_string());
        Ok(())
    })?;
    let mut sets = BTreeMap::new();
    read_rows(
        dir,
        "CraftingQualityAtlasSet",
        &["ID", "IconInv", "IconMixedInv"],
        |row| {
            let icon = number(row[1])?;
            let name = names
                .get(&icon)
                .ok_or_else(|| format!("Crafting atlas set {}: missing element {icon}", row[0]))?;
            let mixed_id = number(row[2])?;
            let mixed = names.get(&mixed_id).ok_or_else(|| {
                format!(
                    "Crafting atlas set {}: missing mixed element {mixed_id}",
                    row[0]
                )
            })?;
            sets.insert(number(row[0])?, (name.clone(), mixed.clone()));
            Ok(())
        },
    )?;
    Ok(sets)
}

fn load_quality_types(dir: &Path) -> Result<BTreeMap<u32, CraftingQuality>, String> {
    let atlases = load_inventory_atlases(dir)?;
    let mut qualities = BTreeMap::new();
    read_rows(
        dir,
        "CraftingQuality",
        &["ID", "QualityTier", "CraftingQualityAtlasSetID"],
        |row| {
            let tier = number(row[1])?;
            if tier == 0 {
                return Ok(());
            }
            let atlas_set = number(row[2])?;
            let atlas = atlases.get(&atlas_set).ok_or_else(|| {
                format!(
                    "Crafting quality {}: missing inventory atlas set {atlas_set}",
                    row[0]
                )
            })?;
            qualities.insert(
                number(row[0])?,
                CraftingQuality {
                    tier,
                    inventory_atlas: atlas.0.clone(),
                    mixed_inventory_atlas: atlas.1.clone(),
                },
            );
            Ok(())
        },
    )?;
    Ok(qualities)
}

pub fn load(dir: &Path) -> Result<BTreeMap<u32, CraftingQuality>, String> {
    let qualities = load_quality_types(dir)?;
    let mut items = BTreeMap::new();
    read_rows(dir, "Item", &["ID", "CraftingQualityID"], |row| {
        let quality_id = number(row[1])?;
        if quality_id == 0 {
            return Ok(());
        }
        let quality = qualities
            .get(&quality_id)
            .ok_or_else(|| format!("Item {}: unknown crafting quality {quality_id}", row[0]))?;
        items.insert(number(row[0])?, quality.clone());
        Ok(())
    })?;
    Ok(items)
}
