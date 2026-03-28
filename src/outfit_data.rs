use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;

use bevy::prelude::*;
use crate::helmet_geoset_data::{HelmetGeosetRule, load_helmet_geoset_rules};

/// Result of resolving a starter outfit for a (race, class, sex) combo.
#[derive(Debug, Clone, Default)]
pub struct OutfitResult {
    /// (ComponentSection, texture FDID) pairs for body texture compositing.
    pub item_textures: Vec<(u8, u32)>,
    /// (geoset_group_index, value) overrides from equipped items.
    /// Currently unused because ItemDisplayInfo::GeosetGroup_* is not a raw M2
    /// geoset group id; it needs item-slot-aware mapping first.
    pub geoset_overrides: Vec<(u16, u16)>,
    /// (ModelResourcesID, M2 FDID) for items with 3D models (weapons, shoulders, helm).
    pub model_fdids: Vec<(u32, u32)>,
}

#[derive(Debug, Clone, Default)]
struct DisplayInfoResolved {
    item_textures: Vec<(u8, u32)>,
    geoset_overrides: Vec<(u16, u16)>,
    model_resource_ids: Vec<u32>,
    model_material_resource_ids: Vec<u32>,
    helmet_geoset_vis_ids: Vec<u32>,
    head_geoset_groups: [i16; 2],
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
    /// ItemDisplayInfoID -> raw hand/glove geoset selector from GeosetGroup_0.
    hand_geoset_group: HashMap<u32, u16>,
    /// MaterialResourcesID -> first diffuse texture FileDataID
    material_to_texture: HashMap<u32, u32>,
    /// ModelResourcesID -> model FileDataIDs
    model_to_fdids: HashMap<u32, Vec<u32>>,
    /// RaceID -> model filename token prefix (for example `hu`, `be`).
    race_prefix: HashMap<u8, String>,
    /// HelmetGeosetVisDataID -> race-specific hide rules.
    helmet_geoset_rules: HashMap<u32, Vec<HelmetGeosetRule>>,
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
        self.loaded
            .get_or_init(|| match Self::try_load(&self.data_dir) {
                Ok(data) => Ok(data),
                Err(err) => {
                    warn!("Failed to load outfit data: {err}");
                    Err(err)
                }
            })
            .as_ref()
            .ok()
    }

    fn try_load(data_dir: &Path) -> Result<LoadedOutfitData, String> {
        let outfits = parse_char_start_outfits(&data_dir.join("CharStartOutfit.csv"))?;
        let item_to_appearance =
            parse_item_modified_appearance(&data_dir.join("ItemModifiedAppearance.csv"))?;
        let appearance_to_display = parse_item_appearance(&data_dir.join("ItemAppearance.csv"))?;
        let display_resources = load_display_resources(data_dir)?;
        let data = LoadedOutfitData {
            outfits,
            item_to_appearance,
            appearance_to_display,
            display_info: display_resources.display_info,
            hand_geoset_group: display_resources.hand_geoset_group,
            material_to_texture: display_resources.material_to_texture,
            model_to_fdids: display_resources.model_to_fdids,
            race_prefix: display_resources.race_prefix,
            helmet_geoset_rules: display_resources.helmet_geoset_rules,
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

        self.resolve_item_ids(data, item_ids)
    }

    pub fn resolve_display_info(&self, display_info_id: u32) -> OutfitResult {
        let Some(data) = self.loaded() else {
            return OutfitResult::default();
        };
        self.resolve_display_infos(data, [display_info_id])
    }

    pub fn hand_geoset_variant(&self, display_info_id: u32) -> Option<u16> {
        let data = self.loaded()?;
        let raw = *data.hand_geoset_group.get(&display_info_id)?;
        // Human glove geosets use 401 as bare wrists and item variants start at 402.
        raw.checked_add(1)
    }

    pub fn head_geoset_overrides(&self, display_info_id: u32) -> Vec<(u16, u16)> {
        let Some(data) = self.loaded() else {
            return Vec::new();
        };
        let Some(display) = data.display_info.get(&display_info_id) else {
            return Vec::new();
        };
        collect_head_geoset_overrides(display)
    }

    pub fn resolve_runtime_model(
        &self,
        display_info_id: u32,
        race: u8,
        sex: u8,
    ) -> Option<(u32, [u32; 3])> {
        let data = self.loaded()?;
        let display = data.display_info.get(&display_info_id)?;
        let model_resource_id = *display.model_resource_ids.first()?;
        let model_fdid = select_model_fdid(data, model_resource_id, race, sex)?;
        let mut skin_fdids = [0; 3];
        for (idx, material_resource_id) in display.model_material_resource_ids.iter().take(3).enumerate() {
            skin_fdids[idx] = data
                .material_to_texture
                .get(material_resource_id)
                .copied()
                .unwrap_or(0);
        }
        Some((model_fdid, skin_fdids))
    }

    pub fn helmet_hide_geoset_groups(&self, display_info_id: u32, race: u8) -> Vec<u16> {
        let Some(data) = self.loaded() else {
            return Vec::new();
        };
        let Some(display) = data.display_info.get(&display_info_id) else {
            return Vec::new();
        };
        collect_helmet_hide_geoset_groups(data, display, race)
    }

    fn resolve_item_ids(&self, data: &LoadedOutfitData, item_ids: &[u32]) -> OutfitResult {
        let display_ids = item_ids
            .iter()
            .filter_map(|item_id| data.item_to_appearance.get(item_id))
            .filter_map(|appearance_id| data.appearance_to_display.get(appearance_id))
            .copied()
            .collect::<Vec<_>>();
        self.resolve_display_infos(data, display_ids)
    }

    fn resolve_display_infos(
        &self,
        data: &LoadedOutfitData,
        display_ids: impl IntoIterator<Item = u32>,
    ) -> OutfitResult {
        let mut result = OutfitResult::default();
        let mut seen_textures = HashSet::new();
        let mut seen_geosets = HashSet::new();
        let mut seen_models = HashSet::new();

        for display_id in display_ids {
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
                let Some(&model_fdid) = data
                    .model_to_fdids
                    .get(&model_resource_id)
                    .and_then(|fdids| fdids.first())
                else {
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

struct DisplayResources {
    display_info: HashMap<u32, DisplayInfoResolved>,
    hand_geoset_group: HashMap<u32, u16>,
    material_to_texture: HashMap<u32, u32>,
    model_to_fdids: HashMap<u32, Vec<u32>>,
    race_prefix: HashMap<u8, String>,
    helmet_geoset_rules: HashMap<u32, Vec<HelmetGeosetRule>>,
}

fn load_display_resources(data_dir: &Path) -> Result<DisplayResources, String> {
    let (base_display_info, hand_geoset_group) =
        parse_item_display_info(&data_dir.join("ItemDisplayInfo.csv"))?;
    let material_to_texture = parse_texture_file_data(&data_dir.join("TextureFileData.csv"))?;
    let display_materials = parse_item_display_info_material_res(
        &data_dir.join("ItemDisplayInfoMaterialRes.csv"),
        &material_to_texture,
    )?;
    Ok(DisplayResources {
        display_info: merge_display_materials(base_display_info, display_materials),
        hand_geoset_group,
        material_to_texture,
        model_to_fdids: parse_model_file_data(&data_dir.join("ModelFileData.csv"))?,
        race_prefix: parse_race_prefix(&data_dir.join("ChrRaces.csv"))?,
        helmet_geoset_rules: load_helmet_geoset_rules(data_dir)?,
    })
}

fn merge_display_materials(
    mut display_info: HashMap<u32, DisplayInfoResolved>,
    display_materials: HashMap<u32, Vec<(u8, u32)>>,
) -> HashMap<u32, DisplayInfoResolved> {
    for (display_id, textures) in display_materials {
        if let Some(entry) = display_info.get_mut(&display_id) {
            entry.item_textures.extend(textures);
            continue;
        }
        display_info.insert(
            display_id,
            DisplayInfoResolved {
                item_textures: textures,
                ..Default::default()
            },
        );
    }
    display_info
}

fn collect_helmet_hide_geoset_groups(
    data: &LoadedOutfitData,
    display: &DisplayInfoResolved,
    race: u8,
) -> Vec<u16> {
    let race_bit = playable_race_bit_selection(race);
    let mut hidden = Vec::new();
    for vis_id in &display.helmet_geoset_vis_ids {
        let Some(rules) = data.helmet_geoset_rules.get(vis_id) else {
            continue;
        };
        for rule in rules {
            if helmet_geoset_rule_matches(*rule, race, race_bit)
                && !hidden.contains(&rule.hide_geoset_group)
            {
                hidden.push(rule.hide_geoset_group);
            }
        }
    }
    hidden
}

fn helmet_geoset_rule_matches(rule: HelmetGeosetRule, race: u8, race_bit: u32) -> bool {
    rule.race_id == race
        || (rule.race_id == 0
            && rule.race_bit_selection != 0
            && rule.race_bit_selection == race_bit)
}

fn playable_race_bit_selection(race: u8) -> u32 {
    if matches!(race, 1 | 3 | 4 | 7 | 11 | 22 | 25 | 29 | 30 | 34 | 37) {
        1
    } else if matches!(race, 2 | 5 | 6 | 8 | 9 | 10 | 27 | 28 | 31 | 35 | 36) {
        2
    } else {
        3
    }
}

fn select_model_fdid(
    data: &LoadedOutfitData,
    model_resource_id: u32,
    race: u8,
    sex: u8,
) -> Option<u32> {
    let candidates = data.model_to_fdids.get(&model_resource_id)?;
    let suffixes = race_model_suffixes(data, race, sex);
    if !suffixes.is_empty() {
        for suffix in &suffixes {
            for &fdid in candidates {
                let Some(path) = game_engine::listfile::lookup_fdid(fdid) else {
                    continue;
                };
                if path.ends_with(&format!("_{suffix}.m2")) {
                    return Some(fdid);
                }
            }
        }
        return None;
    }
    candidates.first().copied()
}

fn race_model_suffixes(data: &LoadedOutfitData, race: u8, sex: u8) -> Vec<String> {
    let Some(prefix) = data.race_prefix.get(&race) else {
        return Vec::new();
    };
    let sex_suffix = match sex {
        0 => "m",
        1 => "f",
        _ => return Vec::new(),
    };
    let mut suffixes = vec![format!("{prefix}{sex_suffix}")];
    if prefix.len() == 2 {
        suffixes.push(format!("{prefix}_{sex_suffix}"));
    }
    suffixes
}

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

fn parse_item_display_info(
    path: &Path,
) -> Result<(HashMap<u32, DisplayInfoResolved>, HashMap<u32, u16>), String> {
    let (h, rows) = read_csv(path)?;
    let columns = ItemDisplayInfoColumns::from_headers(&h)?;

    let mut map = HashMap::new();
    let mut hand_geoset_group = HashMap::new();
    for row in &rows {
        insert_display_info_row(&mut map, &mut hand_geoset_group, row, columns);
    }
    Ok((map, hand_geoset_group))
}

fn insert_display_info_row(
    map: &mut HashMap<u32, DisplayInfoResolved>,
    hand_geoset_group: &mut HashMap<u32, u16>,
    row: &[String],
    columns: ItemDisplayInfoColumns,
) {
    let id = field_u32(row, columns.id);
    if id == 0 {
        return;
    }
    let glove_geoset = field_u32(row, columns.glove_geoset) as u16;
    if glove_geoset != 0 {
        hand_geoset_group.insert(id, glove_geoset);
    }
    map.insert(id, parse_display_info_row(row, columns));
}

fn parse_display_info_row(row: &[String], columns: ItemDisplayInfoColumns) -> DisplayInfoResolved {
    DisplayInfoResolved {
        item_textures: Vec::new(),
        geoset_overrides: Vec::new(),
        model_resource_ids: collect_unique_u32(row, &[columns.model_res_0, columns.model_res_1]),
        model_material_resource_ids: collect_unique_u32(
            row,
            &[columns.model_mat_res_0, columns.model_mat_res_1],
        ),
        helmet_geoset_vis_ids: collect_unique_u32(row, &[columns.helmet_vis_0, columns.helmet_vis_1]),
        head_geoset_groups: [
            field_i32(row, columns.geoset_group_0) as i16,
            field_i32(row, columns.geoset_group_1) as i16,
        ],
    }
}

#[derive(Clone, Copy)]
struct ItemDisplayInfoColumns {
    id: usize,
    model_res_0: usize,
    model_res_1: usize,
    model_mat_res_0: usize,
    model_mat_res_1: usize,
    glove_geoset: usize,
    geoset_group_0: usize,
    geoset_group_1: usize,
    helmet_vis_0: usize,
    helmet_vis_1: usize,
}

impl ItemDisplayInfoColumns {
    fn from_headers(headers: &[String]) -> Result<Self, String> {
        Ok(Self {
            id: col(headers, "ID")?,
            model_res_0: col(headers, "ModelResourcesID_0")?,
            model_res_1: col(headers, "ModelResourcesID_1")?,
            model_mat_res_0: col(headers, "ModelMaterialResourcesID_0")?,
            model_mat_res_1: col(headers, "ModelMaterialResourcesID_1")?,
            glove_geoset: col(headers, "GeosetGroup_0")?,
            geoset_group_0: col(headers, "GeosetGroup_0")?,
            geoset_group_1: col(headers, "GeosetGroup_1")?,
            helmet_vis_0: col(headers, "HelmetGeosetVis_0")?,
            helmet_vis_1: col(headers, "HelmetGeosetVis_1")?,
        })
    }
}

fn collect_head_geoset_overrides(display: &DisplayInfoResolved) -> Vec<(u16, u16)> {
    let mut overrides = Vec::new();
    if let Some(primary_variant) = head_geoset_primary_variant(display.head_geoset_groups[0]) {
        overrides.push((27, primary_variant));
    }
    if let Some(secondary_variant) = head_geoset_secondary_variant(display.head_geoset_groups[1]) {
        overrides.push((21, secondary_variant));
    }
    overrides
}

fn head_geoset_primary_variant(raw_value: i16) -> Option<u16> {
    match raw_value {
        value if value < 0 => None,
        0 => Some(2),
        value => Some(value as u16),
    }
}

fn head_geoset_secondary_variant(raw_value: i16) -> Option<u16> {
    match raw_value {
        value if value <= 0 => None,
        value => Some(value as u16),
    }
}

fn collect_unique_u32(row: &[String], columns: &[usize]) -> Vec<u32> {
    let mut values = Vec::new();
    for &column in columns {
        let value = field_u32(row, column);
        if value != 0 && !values.contains(&value) {
            values.push(value);
        }
    }
    values
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

fn parse_model_file_data(path: &Path) -> Result<HashMap<u32, Vec<u32>>, String> {
    let (h, rows) = read_csv(path)?;
    let file_data_col = col(&h, "FileDataID")?;
    let model_resource_col = col(&h, "ModelResourcesID")?;

    let mut map: HashMap<u32, Vec<u32>> = HashMap::new();
    for row in &rows {
        let file_data_id = field_u32(row, file_data_col);
        let model_resource_id = field_u32(row, model_resource_col);
        if file_data_id == 0 || model_resource_id == 0 {
            continue;
        }
        let entry = map.entry(model_resource_id).or_default();
        if !entry.contains(&file_data_id) {
            entry.push(file_data_id);
        }
    }
    Ok(map)
}

fn parse_race_prefix(path: &Path) -> Result<HashMap<u8, String>, String> {
    let (h, rows) = read_csv(path)?;
    let id_col = col(&h, "ID")?;
    let prefix_col = col(&h, "ClientPrefix")?;

    let mut map = HashMap::new();
    for row in &rows {
        let id = field_u32(row, id_col) as u8;
        let prefix = row
            .get(prefix_col)
            .map(|s| s.trim().to_ascii_lowercase())
            .unwrap_or_default();
        if id != 0 && !prefix.is_empty() {
            map.insert(id, prefix);
        }
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

fn field_i32(row: &[String], col: usize) -> i32 {
    row.get(col)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_outfit_data_eagerly() {
        let data = OutfitData::load(Path::new("data"));
        assert!(data.loaded.get().is_none());

        let result = data.resolve_outfit(1, 1, 0);
        assert!(data.loaded.get().is_some());
        assert!(
            !result.item_textures.is_empty() || !result.model_fdids.is_empty(),
            "expected starter outfit data for human warrior male"
        );
        assert!(
            result.geoset_overrides.is_empty(),
            "raw ItemDisplayInfo geoset columns should not be applied directly"
        );
    }

    #[test]
    fn live_mask_display_resolves_head_geoset_defaults() {
        let data = OutfitData::load(Path::new("data"));

        assert_eq!(data.head_geoset_overrides(720086), vec![(27, 2)]);
    }
}
