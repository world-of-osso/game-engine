use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use crate::helmet_geoset_data::{HelmetGeosetRule, load_helmet_geoset_rules};
use bevy::prelude::*;

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
pub(crate) struct DisplayInfoResolved {
    pub item_textures: Vec<(u8, u32)>,
    pub geoset_overrides: Vec<(u16, u16)>,
    pub model_resource_ids: Vec<u32>,
    pub model_material_resource_ids: Vec<u32>,
    pub model_resource_columns: [u32; 2],
    pub model_material_resource_columns: [u32; 2],
    pub helmet_geoset_vis_ids: Vec<u32>,
    pub geoset_groups: [i16; 6],
}

#[cfg(test)]
pub(crate) struct DisplayMaterialTextures {
    pub direct: HashMap<u32, Vec<(u8, u32)>>,
}

#[derive(Debug, Default)]
struct LoadedOutfitData {
    cache_path: PathBuf,
    display_info_cache: Mutex<HashMap<u32, Option<DisplayInfoResolved>>>,
    material_to_texture_cache: Mutex<HashMap<u32, Option<u32>>>,
    model_to_fdids_cache: Mutex<HashMap<u32, Vec<u32>>>,
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
        let cache_path = crate::world_db::import_outfit_links_cache(data_dir)?;
        let data = LoadedOutfitData {
            cache_path,
            display_info_cache: Mutex::new(HashMap::new()),
            material_to_texture_cache: Mutex::new(HashMap::new()),
            model_to_fdids_cache: Mutex::new(HashMap::new()),
            race_prefix: crate::world_db::load_chr_race_prefixes()?,
            helmet_geoset_rules: load_helmet_geoset_rules(data_dir)?,
        };
        info!(
            "OutfitData loaded: cache={}, race prefixes={}, helmet geoset rules={}",
            data.cache_path.display(),
            data.race_prefix.len(),
            data.helmet_geoset_rules.len()
        );
        Ok(data)
    }

    pub fn resolve_outfit(&self, race: u8, class: u8, sex: u8) -> OutfitResult {
        let Some(data) = self.loaded() else {
            return OutfitResult::default();
        };
        let Ok(display_ids) =
            crate::world_db::resolve_cached_outfit_display_ids(&self.data_dir, race, class, sex)
        else {
            return OutfitResult::default();
        };
        if display_ids.is_empty() {
            return OutfitResult::default();
        }
        self.resolve_display_infos(data, display_ids)
    }

    pub fn resolve_display_info(&self, display_info_id: u32) -> OutfitResult {
        let Some(data) = self.loaded() else {
            return OutfitResult::default();
        };
        self.resolve_display_infos(data, [display_info_id])
    }

    pub fn hand_geoset_variant(&self, display_info_id: u32) -> Option<u16> {
        self.display_geoset_variant(display_info_id, 0)
    }

    pub fn cape_geoset_variant(&self, display_info_id: u32) -> Option<u16> {
        self.display_geoset_variant(display_info_id, 0)
    }

    pub fn chest_geoset_variant(&self, display_info_id: u32) -> Option<u16> {
        let data = self.loaded()?;
        let display = self.display_info(data, display_info_id)?;
        let raw = *display.geoset_groups.first()?;
        (raw > 0).then_some(2)
    }

    pub fn waist_geoset_variant(&self, display_info_id: u32) -> Option<u16> {
        self.display_geoset_variant(display_info_id, 0)
    }

    pub fn pants_geoset_variant(&self, display_info_id: u32) -> Option<u16> {
        self.display_geoset_variant(display_info_id, 0)
    }

    pub fn kneepad_geoset_variant(&self, display_info_id: u32) -> Option<u16> {
        self.display_geoset_variant(display_info_id, 1)
    }

    pub fn boot_geoset_variant(&self, display_info_id: u32) -> Option<u16> {
        self.display_geoset_variant(display_info_id, 0)
    }

    pub fn trouser_geoset_variant(&self, display_info_id: u32) -> Option<u16> {
        self.display_geoset_variant(display_info_id, 2)
    }

    pub fn display_material_texture_fdids(&self, display_info_id: u32) -> Vec<u32> {
        let Some(data) = self.loaded() else {
            return Vec::new();
        };
        let Some(display) = self.display_info(data, display_info_id) else {
            return Vec::new();
        };
        display
            .model_material_resource_ids
            .iter()
            .filter_map(|material_resource_id| {
                self.material_texture_fdid(data, *material_resource_id)
            })
            .filter(|fdid| *fdid != 0)
            .collect()
    }

    pub fn cape_texture_fdid(&self, display_info_id: u32) -> Option<u32> {
        self.display_material_texture_fdids(display_info_id)
            .into_iter()
            .next()
    }

    pub fn head_geoset_overrides(&self, display_info_id: u32) -> Vec<(u16, u16)> {
        let Some(data) = self.loaded() else {
            return Vec::new();
        };
        let Some(display) = self.display_info(data, display_info_id) else {
            return Vec::new();
        };
        collect_head_geoset_overrides(&display)
    }

    pub fn resolve_runtime_model(
        &self,
        display_info_id: u32,
        race: u8,
        sex: u8,
    ) -> Option<(u32, [u32; 3])> {
        let data = self.loaded()?;
        let display = self.display_info(data, display_info_id)?;
        let model_resource_id = *display.model_resource_ids.first()?;
        let model_fdid = self.select_model_fdid(data, model_resource_id, race, sex)?;
        let mut skin_fdids = [0; 3];
        for (idx, material_resource_id) in display
            .model_material_resource_ids
            .iter()
            .take(3)
            .enumerate()
        {
            skin_fdids[idx] = self
                .material_texture_fdid(data, *material_resource_id)
                .unwrap_or(0);
        }
        Some((model_fdid, skin_fdids))
    }

    pub fn resolve_item_model_skin_fdids_for_model_path(
        &self,
        model_path: &Path,
    ) -> Option<[u32; 3]> {
        let stem = model_path.file_stem()?.to_str()?;
        if let Ok(model_fdid) = stem.parse::<u32>() {
            return crate::world_db::resolve_cached_skin_fdids_for_model_fdid(
                &self.data_dir,
                model_fdid,
            )
            .ok()
            .flatten();
        }
        let model_name = model_path.file_name()?.to_str()?;
        crate::world_db::resolve_cached_skin_fdids_for_model_name(&self.data_dir, model_name)
            .ok()
            .flatten()
    }

    pub fn resolve_shoulder_runtime_model(
        &self,
        display_info_id: u32,
        shoulder_index: usize,
        race: u8,
        sex: u8,
    ) -> Option<(u32, [u32; 3])> {
        let data = self.loaded()?;
        let display = self.display_info(data, display_info_id)?;
        let column_index = shoulder_model_column_index(&display, shoulder_index)?;
        let model_resource_id = display.model_resource_columns[column_index];
        let model_fdid =
            self.select_shoulder_model_fdid(data, model_resource_id, shoulder_index, race, sex)?;
        let mut skin_fdids = [0; 3];
        let material_resource_id = display.model_material_resource_columns[column_index];
        if material_resource_id != 0 {
            skin_fdids[0] = self
                .material_texture_fdid(data, material_resource_id)
                .unwrap_or(0);
        }
        Some((model_fdid, skin_fdids))
    }

    pub fn has_helmet_geoset_vis_data(&self, display_info_id: u32) -> bool {
        let Some(data) = self.loaded() else {
            return false;
        };
        let Some(display) = self.display_info(data, display_info_id) else {
            return false;
        };
        !display.helmet_geoset_vis_ids.is_empty()
    }

    pub fn helmet_hide_geoset_groups(&self, display_info_id: u32, race: u8) -> Vec<u16> {
        let Some(data) = self.loaded() else {
            return Vec::new();
        };
        let Some(display) = self.display_info(data, display_info_id) else {
            return Vec::new();
        };
        collect_helmet_hide_geoset_groups(data, &display, race)
    }

    fn resolve_display_infos(
        &self,
        data: &LoadedOutfitData,
        display_ids: impl IntoIterator<Item = u32>,
    ) -> OutfitResult {
        let mut result = OutfitResult::default();
        for display_id in display_ids {
            let Some(display) = self.display_info(data, display_id) else {
                continue;
            };
            self.merge_display_into_result(&mut result, data, &display);
        }
        result
    }

    pub fn material_texture_count(&self) -> usize {
        self.loaded()
            .map(|data| data.material_to_texture_cache.lock().unwrap().len())
            .unwrap_or(0)
    }

    fn display_geoset_variant(&self, display_info_id: u32, group_index: usize) -> Option<u16> {
        let data = self.loaded()?;
        let display = self.display_info(data, display_info_id)?;
        let raw = *display.geoset_groups.get(group_index)?;
        let raw = u16::try_from(raw).ok()?;
        (raw != 0).then_some(raw + 1)
    }
    fn display_info(
        &self,
        data: &LoadedOutfitData,
        display_info_id: u32,
    ) -> Option<DisplayInfoResolved> {
        if let Some(cached) = data
            .display_info_cache
            .lock()
            .unwrap()
            .get(&display_info_id)
            .cloned()
        {
            return cached;
        }
        let resolved = crate::world_db::load_cached_display_info(&self.data_dir, display_info_id)
            .map_err(|err| warn!("Failed display_info lookup for {display_info_id}: {err}"))
            .ok()
            .flatten();
        data.display_info_cache
            .lock()
            .unwrap()
            .insert(display_info_id, resolved.clone());
        resolved
    }

    fn material_texture_fdid(
        &self,
        data: &LoadedOutfitData,
        material_resource_id: u32,
    ) -> Option<u32> {
        if let Some(cached) = data
            .material_to_texture_cache
            .lock()
            .unwrap()
            .get(&material_resource_id)
            .copied()
        {
            return cached;
        }
        let resolved = crate::world_db::load_cached_material_texture_fdid(
            &self.data_dir,
            material_resource_id,
        )
        .map_err(|err| warn!("Failed material_to_texture lookup for {material_resource_id}: {err}"))
        .ok()
        .flatten();
        data.material_to_texture_cache
            .lock()
            .unwrap()
            .insert(material_resource_id, resolved);
        resolved
    }

    fn model_fdids(&self, data: &LoadedOutfitData, model_resource_id: u32) -> Vec<u32> {
        if let Some(cached) = data
            .model_to_fdids_cache
            .lock()
            .unwrap()
            .get(&model_resource_id)
            .cloned()
        {
            return cached;
        }
        let resolved = crate::world_db::load_cached_model_fdids(&self.data_dir, model_resource_id)
            .map_err(|err| warn!("Failed model_to_fdid lookup for {model_resource_id}: {err}"))
            .unwrap_or_default();
        data.model_to_fdids_cache
            .lock()
            .unwrap()
            .insert(model_resource_id, resolved.clone());
        resolved
    }

    fn merge_display_into_result(
        &self,
        result: &mut OutfitResult,
        data: &LoadedOutfitData,
        display: &DisplayInfoResolved,
    ) {
        for &(component_section, fdid) in &display.item_textures {
            if !result.item_textures.contains(&(component_section, fdid)) {
                result.item_textures.push((component_section, fdid));
            }
        }
        for &pair in &display.geoset_overrides {
            if !result.geoset_overrides.contains(&pair) {
                result.geoset_overrides.push(pair);
            }
        }
        for &model_resource_id in &display.model_resource_ids {
            let Some(model_fdid) = self.model_fdids(data, model_resource_id).first().copied()
            else {
                continue;
            };
            let pair = (model_resource_id, model_fdid);
            if !result.model_fdids.contains(&pair) {
                result.model_fdids.push(pair);
            }
        }
    }

    fn select_model_fdid(
        &self,
        data: &LoadedOutfitData,
        model_resource_id: u32,
        race: u8,
        sex: u8,
    ) -> Option<u32> {
        let candidates = self.model_fdids(data, model_resource_id);
        select_candidate_fdid(data, &candidates, race, sex)
    }

    fn select_shoulder_model_fdid(
        &self,
        data: &LoadedOutfitData,
        model_resource_id: u32,
        shoulder_index: usize,
        race: u8,
        sex: u8,
    ) -> Option<u32> {
        let candidates = self.model_fdids(data, model_resource_id);
        let side_candidates = candidates
            .iter()
            .copied()
            .filter(|fdid| shoulder_model_matches_side(*fdid, shoulder_index))
            .collect::<Vec<_>>();
        select_candidate_fdid(data, &side_candidates, race, sex)
            .or_else(|| select_candidate_fdid(data, &candidates, race, sex))
    }
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

fn collect_head_geoset_overrides(display: &DisplayInfoResolved) -> Vec<(u16, u16)> {
    let mut overrides = Vec::new();
    if let Some(primary_variant) = head_geoset_primary_variant(display.geoset_groups[0]) {
        overrides.push((27, primary_variant));
    }
    if let Some(secondary_variant) = head_geoset_secondary_variant(display.geoset_groups[1]) {
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

fn shoulder_model_column_index(
    display: &DisplayInfoResolved,
    shoulder_index: usize,
) -> Option<usize> {
    match shoulder_index {
        0 => {
            if display.model_resource_columns[0] != 0 {
                Some(0)
            } else if display.model_resource_columns[1] != 0 {
                Some(1)
            } else {
                None
            }
        }
        1 => {
            if display.model_resource_columns[1] != 0 {
                Some(1)
            } else if display.model_resource_columns[0] != 0 {
                Some(0)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn shoulder_model_matches_side(fdid: u32, shoulder_index: usize) -> bool {
    let Some(path) = game_engine::listfile::lookup_fdid(fdid) else {
        return false;
    };
    let lower = path.to_ascii_lowercase();
    match shoulder_index {
        0 => lower.contains("/lshoulder_") || lower.ends_with("_l.m2"),
        1 => lower.contains("/rshoulder_") || lower.ends_with("_r.m2"),
        _ => false,
    }
}

fn select_candidate_fdid(
    data: &LoadedOutfitData,
    candidates: &[u32],
    race: u8,
    sex: u8,
) -> Option<u32> {
    if candidates.is_empty() {
        return None;
    }
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
    }
    (candidates.len() == 1).then(|| candidates[0])
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

    #[test]
    fn waist_display_without_material_rows_has_no_item_textures() {
        let data = OutfitData::load(Path::new("data"));

        let resolved = data.resolve_display_info(15040);

        assert!(
            resolved.item_textures.is_empty(),
            "display 15040 has no material rows, should have no item textures, got {:?}",
            resolved.item_textures
        );
    }
}
