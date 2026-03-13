use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;

use bevy::prelude::*;

/// Result of resolving a starter outfit for a (race, class, sex) combo.
#[derive(Debug, Clone, Default)]
pub struct OutfitResult {
    /// (ComponentSection, texture FDID) pairs for body texture compositing.
    pub item_textures: Vec<(u8, u32)>,
    /// (geoset_group_index, value) overrides from equipped items.
    pub geoset_overrides: Vec<(u16, u16)>,
    /// (ModelResourcesID, M2 FDID) for items with 3D models (weapons, shoulders, helm).
    pub model_fdids: Vec<(u32, u32)>,
}

#[derive(Debug, Clone, Default)]
struct DisplayInfoResolved {
    item_textures: Vec<(u8, u32)>,
    geoset_overrides: Vec<(u16, u16)>,
    model_resource_ids: Vec<u32>,
}

#[derive(Debug, Default)]
struct LoadedOutfitData {
    /// (race, class, sex) -> starter item ids
    outfits: HashMap<(u8, u8, u8), Vec<u32>>,
    /// ItemID -> ItemAppearanceID (first match / lowest order encountered)
    item_to_appearance: HashMap<u32, u32>,
    /// ItemAppearanceID -> ItemDisplayInfoID
    appearance_to_display: HashMap<u32, u32>,
    /// ItemDisplayInfoID -> resolved display data
    display_info: HashMap<u32, DisplayInfoResolved>,
    /// MaterialResourcesID -> first diffuse texture FileDataID
    material_to_texture: HashMap<u32, u32>,
    /// ModelResourcesID -> model FileDataID
    model_to_fdid: HashMap<u32, u32>,
}

/// Parsed outfit lookup data loaded lazily on first use.
#[derive(Resource, Debug, Default)]
pub struct OutfitData {
    data_dir: PathBuf,
    loaded: OnceLock<Result<LoadedOutfitData, String>>,
}

impl OutfitData {
    pub fn load(data_dir: &Path) -> Self {
        Self {
            data_dir: data_dir.to_path_buf(),
            loaded: OnceLock::new(),
        }
    }

    fn loaded(&self) -> Option<&LoadedOutfitData> {
        match self
            .loaded
            .get_or_init(|| match Self::try_load(&self.data_dir) {
                Ok(data) => Ok(data),
                Err(err) => {
                    warn!("Failed to load outfit data: {err}");
                    Err(err)
                }
            }) {
            Ok(data) => Some(data),
            Err(_) => None,
        }
    }

    fn try_load(data_dir: &Path) -> Result<LoadedOutfitData, String> {
        let outfits = parse_char_start_outfits(&data_dir.join("CharStartOutfit.csv"))?;
        let item_to_appearance =
            parse_item_modified_appearance(&data_dir.join("ItemModifiedAppearance.csv"))?;
        let appearance_to_display = parse_item_appearance(&data_dir.join("ItemAppearance.csv"))?;
        let base_display_info = parse_item_display_info(&data_dir.join("ItemDisplayInfo.csv"))?;
        let material_to_texture = parse_texture_file_data(&data_dir.join("TextureFileData.csv"))?;
        let model_to_fdid = parse_model_file_data(&data_dir.join("ModelFileData.csv"))?;
        let display_materials = parse_item_display_info_material_res(
            &data_dir.join("ItemDisplayInfoMaterialRes.csv"),
            &material_to_texture,
        )?;

        let mut display_info = base_display_info;
        for (display_id, textures) in display_materials {
            if let Some(entry) = display_info.get_mut(&display_id) {
                entry.item_textures.extend(textures);
            } else {
                display_info.insert(
                    display_id,
                    DisplayInfoResolved {
                        item_textures: textures,
                        ..Default::default()
                    },
                );
            }
        }

        let data = LoadedOutfitData {
            outfits,
            item_to_appearance,
            appearance_to_display,
            display_info,
            material_to_texture,
            model_to_fdid,
        };
        info!(
            "OutfitData loaded: {} outfits, {} item appearances, {} display infos",
            data.outfits.len(),
            data.item_to_appearance.len(),
            data.display_info.len()
        );
        Ok(data)
    }

    pub fn resolve_outfit(&self, race: u8, class: u8, sex: u8) -> OutfitResult {
        let Some(data) = self.loaded() else {
            return OutfitResult::default();
        };
        let Some(item_ids) = data.outfits.get(&(race, class, sex)) else {
            return OutfitResult::default();
        };

        let mut result = OutfitResult::default();
        let mut seen_textures = HashSet::new();
        let mut seen_geosets = HashSet::new();
        let mut seen_models = HashSet::new();

        for &item_id in item_ids {
            let Some(&appearance_id) = data.item_to_appearance.get(&item_id) else {
                continue;
            };
            let Some(&display_id) = data.appearance_to_display.get(&appearance_id) else {
                continue;
            };
            let Some(display) = data.display_info.get(&display_id) else {
                continue;
            };

            for &(component_section, fdid) in &display.item_textures {
                if seen_textures.insert((component_section, fdid)) {
                    result.item_textures.push((component_section, fdid));
                }
            }

            for &(group_index, value) in &display.geoset_overrides {
                if seen_geosets.insert((group_index, value)) {
                    result.geoset_overrides.push((group_index, value));
                }
            }

            for &model_resource_id in &display.model_resource_ids {
                let Some(&model_fdid) = data.model_to_fdid.get(&model_resource_id) else {
                    continue;
                };
                if seen_models.insert((model_resource_id, model_fdid)) {
                    result.model_fdids.push((model_resource_id, model_fdid));
                }
            }
        }

        result
    }

    pub fn material_texture_count(&self) -> usize {
        self.loaded()
            .map(|data| data.material_to_texture.len())
            .unwrap_or(0)
    }
}

type OutfitKey = (u8, u8, u8);

fn parse_char_start_outfits(path: &Path) -> Result<HashMap<OutfitKey, Vec<u32>>, String> {
    let (h, rows) = read_csv(path)?;
    let race_col = col(&h, "RaceID")?;
    let class_col = col(&h, "ClassID")?;
    let sex_col = col(&h, "SexID")?;

    let item_cols: Vec<_> = (0..12)
        .map(|i| col(&h, &format!("ItemID_{i}")))
        .collect::<Result<_, _>>()?;

    let mut outfits = HashMap::new();
    for row in &rows {
        let key = (
            field_u32(row, race_col) as u8,
            field_u32(row, class_col) as u8,
            field_u32(row, sex_col) as u8,
        );
        let items = item_cols
            .iter()
            .map(|&c| field_u32(row, c))
            .filter(|&item_id| item_id != 0 && item_id != 6948)
            .collect::<Vec<_>>();
        outfits.insert(key, items);
    }
    Ok(outfits)
}

fn parse_item_modified_appearance(path: &Path) -> Result<HashMap<u32, u32>, String> {
    let (h, rows) = read_csv(path)?;
    let item_col = col(&h, "ItemID")?;
    let appearance_col = col(&h, "ItemAppearanceID")?;

    let mut map = HashMap::new();
    for row in &rows {
        let item_id = field_u32(row, item_col);
        let appearance_id = field_u32(row, appearance_col);
        if item_id == 0 || appearance_id == 0 {
            continue;
        }
        map.entry(item_id).or_insert(appearance_id);
    }
    Ok(map)
}

fn parse_item_appearance(path: &Path) -> Result<HashMap<u32, u32>, String> {
    let (h, rows) = read_csv(path)?;
    let id_col = col(&h, "ID")?;
    let display_info_col = col(&h, "ItemDisplayInfoID")?;

    let mut map = HashMap::new();
    for row in &rows {
        let id = field_u32(row, id_col);
        let display_info_id = field_u32(row, display_info_col);
        if id == 0 || display_info_id == 0 {
            continue;
        }
        map.insert(id, display_info_id);
    }
    Ok(map)
}

fn parse_item_display_info(path: &Path) -> Result<HashMap<u32, DisplayInfoResolved>, String> {
    let (h, rows) = read_csv(path)?;
    let id_col = col(&h, "ID")?;
    let model_res_0_col = col(&h, "ModelResourcesID_0")?;
    let model_res_1_col = col(&h, "ModelResourcesID_1")?;

    let geoset_cols: Vec<_> = (0..6)
        .map(|i| col(&h, &format!("GeosetGroup_{i}")))
        .collect::<Result<_, _>>()?;

    let mut map = HashMap::new();
    for row in &rows {
        let id = field_u32(row, id_col);
        if id == 0 {
            continue;
        }

        let mut geoset_overrides = Vec::new();
        for (group_index, &geoset_col) in geoset_cols.iter().enumerate() {
            let value = field_u32(row, geoset_col) as u16;
            if value != 0 {
                geoset_overrides.push((group_index as u16, value));
            }
        }

        let mut model_resource_ids = Vec::new();
        for model_resource_id in [
            field_u32(row, model_res_0_col),
            field_u32(row, model_res_1_col),
        ] {
            if model_resource_id != 0 && !model_resource_ids.contains(&model_resource_id) {
                model_resource_ids.push(model_resource_id);
            }
        }

        map.insert(
            id,
            DisplayInfoResolved {
                item_textures: Vec::new(),
                geoset_overrides,
                model_resource_ids,
            },
        );
    }
    Ok(map)
}

fn parse_item_display_info_material_res(
    path: &Path,
    material_to_texture: &HashMap<u32, u32>,
) -> Result<HashMap<u32, Vec<(u8, u32)>>, String> {
    let (h, rows) = read_csv(path)?;
    let component_col = col(&h, "ComponentSection")?;
    let material_col = col(&h, "MaterialResourcesID")?;
    let display_info_col = col(&h, "ItemDisplayInfoID")?;

    let mut map: HashMap<u32, Vec<(u8, u32)>> = HashMap::new();
    let mut seen: HashSet<(u32, u8, u32)> = HashSet::new();

    for row in &rows {
        let display_info_id = field_u32(row, display_info_col);
        let component_section = field_u32(row, component_col) as u8;
        let material_resource_id = field_u32(row, material_col);
        let Some(&texture_fdid) = material_to_texture.get(&material_resource_id) else {
            continue;
        };
        if seen.insert((display_info_id, component_section, texture_fdid)) {
            map.entry(display_info_id)
                .or_default()
                .push((component_section, texture_fdid));
        }
    }

    Ok(map)
}

fn parse_texture_file_data(path: &Path) -> Result<HashMap<u32, u32>, String> {
    let (h, rows) = read_csv(path)?;
    let file_data_col = col(&h, "FileDataID")?;
    let usage_type_col = col(&h, "UsageType")?;
    let material_col = col(&h, "MaterialResourcesID")?;

    let mut preferred = HashMap::new();
    let mut fallback = HashMap::new();

    for row in &rows {
        let file_data_id = field_u32(row, file_data_col);
        let usage_type = field_u32(row, usage_type_col);
        let material_resource_id = field_u32(row, material_col);
        if file_data_id == 0 || material_resource_id == 0 {
            continue;
        }

        fallback.entry(material_resource_id).or_insert(file_data_id);
        if usage_type == 0 {
            preferred
                .entry(material_resource_id)
                .or_insert(file_data_id);
        }
    }

    for (material_resource_id, file_data_id) in fallback {
        preferred
            .entry(material_resource_id)
            .or_insert(file_data_id);
    }

    Ok(preferred)
}

fn parse_model_file_data(path: &Path) -> Result<HashMap<u32, u32>, String> {
    let (h, rows) = read_csv(path)?;
    let file_data_col = col(&h, "FileDataID")?;
    let model_resource_col = col(&h, "ModelResourcesID")?;

    let mut map = HashMap::new();
    for row in &rows {
        let file_data_id = field_u32(row, file_data_col);
        let model_resource_id = field_u32(row, model_resource_col);
        if file_data_id == 0 || model_resource_id == 0 {
            continue;
        }
        map.entry(model_resource_id).or_insert(file_data_id);
    }
    Ok(map)
}

fn read_csv(path: &Path) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let data =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut lines = data.lines();
    let header_line = lines.next().ok_or("empty CSV")?;
    let headers = parse_csv_line(header_line);
    let rows: Vec<_> = lines.map(parse_csv_line).collect();
    Ok((headers, rows))
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    current.push('"');
                } else {
                    in_quotes = false;
                }
            } else {
                current.push(ch);
            }
        } else if ch == '"' {
            in_quotes = true;
        } else if ch == ',' {
            fields.push(std::mem::take(&mut current));
        } else {
            current.push(ch);
        }
    }
    fields.push(current);
    fields
}

fn col(headers: &[String], name: &str) -> Result<usize, String> {
    headers
        .iter()
        .position(|h| h == name)
        .ok_or_else(|| format!("Column '{name}' not found"))
}

fn field_u32(row: &[String], col: usize) -> u32 {
    row.get(col)
        .and_then(|s| {
            s.parse::<u32>()
                .ok()
                .or_else(|| s.parse::<i32>().ok().map(|v| v as u32))
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_outfit_data_lazily() {
        let data = OutfitData::load(Path::new("data"));
        assert!(data.loaded.get().is_none());

        let result = data.resolve_outfit(1, 1, 0);
        assert!(data.loaded.get().is_some());
        assert!(
            !result.item_textures.is_empty() || !result.geoset_overrides.is_empty(),
            "expected starter outfit data for human warrior male"
        );
    }
}
