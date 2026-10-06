use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use crate::component_file_data::ComponentFileData;
use crate::helmet_geoset_data::{HelmetGeosetRule, load_helmet_geoset_rules};
#[path = "item_model_material_data.rs"]
mod item_model_material_data;
use item_model_material_data::{ModelMaterials, load_model_materials};

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
pub struct DisplayInfoResolved {
    /// (ComponentSection, MaterialResourcesID) pairs; the texture of each material a
    /// character wears depends on its race and sex.
    pub item_materials: Vec<(u8, u32)>,
    pub geoset_overrides: Vec<(u16, u16)>,
    pub model_resource_ids: Vec<u32>,
    pub model_material_resource_ids: Vec<u32>,
    pub model_resource_columns: [u32; 2],
    pub model_material_resource_columns: [u32; 2],
    pub helmet_geoset_vis_ids: Vec<u32>,
    pub geoset_groups: [i16; 6],
}

#[derive(Debug, Default)]
struct LoadedOutfitData {
    cache_path: PathBuf,
    display_info_cache: Mutex<HashMap<u32, Option<DisplayInfoResolved>>>,
    /// MaterialResourcesID -> its texture files.
    material_textures_cache: Mutex<HashMap<u32, Vec<u32>>>,
    model_to_fdids_cache: Mutex<HashMap<u32, Vec<u32>>>,
    /// Which race, sex and side each texture and model file is for.
    components: ComponentFileData,
    model_materials: ModelMaterials,
    /// HelmetGeosetVisDataID -> race-specific hide rules.
    helmet_geoset_rules: HashMap<u32, Vec<HelmetGeosetRule>>,
}

/// Parsed outfit lookup data loaded lazily on first use.
#[derive(Debug, Default)]
pub struct OutfitData {
    data_dir: PathBuf,
    loaded: OnceLock<Result<LoadedOutfitData, String>>,
    helmet_cache: Option<fn(&Path) -> Result<(), String>>,
}

impl OutfitData {
    pub fn load(data_dir: &Path) -> Self {
        Self {
            data_dir: data_dir.to_path_buf(),
            loaded: OnceLock::new(),
            helmet_cache: None,
        }
    }

    pub fn with_root_loaders(
        data_dir: &Path,
        helmet_cache: fn(&Path) -> Result<(), String>,
    ) -> Self {
        let mut catalog = Self::load(data_dir);
        catalog.helmet_cache = Some(helmet_cache);
        catalog
    }

    fn loaded(&self) -> Option<&LoadedOutfitData> {
        self.loaded_result().ok()
    }

    fn loaded_result(&self) -> Result<&LoadedOutfitData, String> {
        self.loaded
            .get_or_init(|| self.try_load())
            .as_ref()
            .map_err(Clone::clone)
    }

    fn try_load(&self) -> Result<LoadedOutfitData, String> {
        let data_dir = &self.data_dir;
        let cache_path = crate::outfit_catalog_db::import_outfit_links_cache(data_dir)?;
        if let Some(cache_helmet) = self.helmet_cache {
            cache_helmet(data_dir)?;
        }
        let data = LoadedOutfitData {
            cache_path,
            display_info_cache: Mutex::new(HashMap::new()),
            material_textures_cache: Mutex::new(HashMap::new()),
            model_to_fdids_cache: Mutex::new(HashMap::new()),
            components: ComponentFileData::load(&data_dir.join("db2/12.1.0.69933"))?,
            model_materials: load_model_materials(data_dir)?,
            helmet_geoset_rules: load_helmet_geoset_rules(data_dir)?,
        };
        Ok(data)
    }

    pub fn resolve_outfit(&self, race: u8, class: u8, sex: u8) -> OutfitResult {
        let Some(data) = self.loaded() else {
            return OutfitResult::default();
        };
        let Ok(display_ids) = crate::outfit_catalog_db::resolve_cached_outfit_display_ids(
            &self.data_dir,
            race,
            class,
            sex,
        ) else {
            return OutfitResult::default();
        };
        if display_ids.is_empty() {
            return OutfitResult::default();
        }
        self.resolve_display_infos(data, display_ids, race, sex)
    }

    /// Resolve a starter outfit without hiding catalog or cache errors.
    pub fn try_resolve_outfit(&self, race: u8, class: u8, sex: u8) -> Result<OutfitResult, String> {
        let data = self.loaded_result()?;
        let ids = crate::outfit_catalog_db::resolve_cached_outfit_display_ids(
            &self.data_dir,
            race,
            class,
            sex,
        )?;
        self.resolve_display_infos_checked(data, ids, race, sex)
    }

    /// Display `display_id` as race `race`/sex `sex` wears it.
    pub fn try_resolve_display_info(
        &self,
        display_id: u32,
        race: u8,
        sex: u8,
    ) -> Result<Option<OutfitResult>, String> {
        let data = self.loaded_result()?;
        let Some(display) =
            crate::outfit_catalog_db::load_cached_display_info(&self.data_dir, display_id)?
        else {
            return Ok(None);
        };
        self.check_model_resources(&display)?;
        let mut result = OutfitResult::default();
        self.merge_display_into_result(&mut result, data, &display, race, sex);
        Ok(Some(result))
    }

    fn resolve_display_infos_checked(
        &self,
        data: &LoadedOutfitData,
        ids: impl IntoIterator<Item = u32>,
        race: u8,
        sex: u8,
    ) -> Result<OutfitResult, String> {
        let mut result = OutfitResult::default();
        for id in ids {
            let display = crate::outfit_catalog_db::load_cached_display_info(&self.data_dir, id)?
                .ok_or_else(|| format!("outfit display {id} missing"))?;
            self.check_model_resources(&display)?;
            self.merge_display_into_result(&mut result, data, &display, race, sex);
        }
        Ok(result)
    }

    fn check_model_resources(&self, display: &DisplayInfoResolved) -> Result<(), String> {
        for &id in &display.model_resource_ids {
            crate::outfit_catalog_db::load_cached_model_fdids(&self.data_dir, id)?;
        }
        Ok(())
    }

    pub fn resolve_item_display_id(&self, item_id: u32) -> Result<u32, String> {
        let data = self.loaded_result()?;
        let conn = crate::cache_sqlite::open_read_only(&data.cache_path)?;
        conn.query_row(
            "SELECT iam.display_info_id
             FROM item_modified_appearance_map ima
             JOIN item_appearance_map iam ON iam.appearance_id = ima.appearance_id
             WHERE ima.item_id = ?1",
            [item_id],
            |row| row.get(0),
        )
        .map_err(|err| format!("resolve item {item_id} display: {err}"))
    }

    pub fn resolve_display_info(&self, display_info_id: u32, race: u8, sex: u8) -> OutfitResult {
        let Some(data) = self.loaded() else {
            return OutfitResult::default();
        };
        self.resolve_display_infos(data, [display_info_id], race, sex)
    }

    /// The model textures of display `display_info_id` as race `race`/sex `sex` wears it.
    pub fn display_material_texture_fdids(
        &self,
        display_info_id: u32,
        race: u8,
        sex: u8,
    ) -> Vec<u32> {
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
                self.material_texture_fdid(data, *material_resource_id, race, sex)
            })
            .filter(|fdid| *fdid != 0)
            .collect()
    }

    pub fn cape_texture_fdid(&self, display_info_id: u32, race: u8, sex: u8) -> Option<u32> {
        self.display_material_texture_fdids(display_info_id, race, sex)
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

    pub fn try_resolve_runtime_model(
        &self,
        display_info_id: u32,
        race: u8,
        sex: u8,
    ) -> Result<Option<(u32, [u32; 3])>, String> {
        self.loaded_result()?;
        let Some(display) =
            crate::outfit_catalog_db::load_cached_display_info(&self.data_dir, display_info_id)?
        else {
            return Ok(None);
        };
        self.check_model_resources(&display)?;
        for &id in &display.model_material_resource_ids {
            crate::outfit_catalog_db::load_cached_material_texture_fdids(&self.data_dir, id)?;
        }
        Ok(self.resolve_runtime_model(display_info_id, race, sex))
    }

    /// Each model column of display `display_info_id` as race `race`/sex `sex` wears it,
    /// with that column's material as its texture.
    pub fn resolve_model_texture_fdids(
        &self,
        display_info_id: u32,
        model_index: usize,
        race: u8,
        sex: u8,
    ) -> Result<Vec<(u32, u32)>, String> {
        let data = self.loaded_result()?;
        data.model_materials.get(&(display_info_id, model_index))
            .into_iter().flatten()
            .map(|&(kind, material)| {
                let fdid = self.material_texture_fdid(data, material, race, sex)
                    .ok_or_else(|| format!("Display {display_info_id} model {model_index} type {kind}: material {material} has no texture"))?;
                Ok((kind, fdid))
            }).collect()
    }

    pub fn shoulder_model_column(
        &self,
        display_info_id: u32,
        shoulder_index: usize,
    ) -> Result<usize, String> {
        let data = self.loaded_result()?;
        let display = self
            .display_info(data, display_info_id)
            .ok_or_else(|| format!("Display {display_info_id} missing"))?;
        shoulder_model_column_index(&display, shoulder_index)
            .ok_or_else(|| format!("Display {display_info_id} has no shoulder {shoulder_index}"))
    }

    pub fn try_resolve_column_models(
        &self,
        display_info_id: u32,
        race: u8,
        sex: u8,
    ) -> Result<Vec<(usize, u32, [u32; 3])>, String> {
        let data = self.loaded_result()?;
        let Some(display) =
            crate::outfit_catalog_db::load_cached_display_info(&self.data_dir, display_info_id)?
        else {
            return Ok(Vec::new());
        };
        self.check_model_resources(&display)?;
        let columns = display
            .model_resource_columns
            .iter()
            .zip(display.model_material_resource_columns)
            .enumerate();
        Ok(columns
            .filter(|(_, (model, _))| **model != 0)
            .filter_map(|(column, (&model, material))| {
                let fdid = self.select_model_fdid(data, model, race, sex)?;
                let texture = (material != 0)
                    .then(|| self.material_texture_fdid(data, material, race, sex))
                    .flatten()
                    .unwrap_or(0);
                Some((column, fdid, [texture, 0, 0]))
            })
            .collect())
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
                .material_texture_fdid(data, *material_resource_id, race, sex)
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
            return crate::outfit_catalog_db::resolve_cached_skin_fdids_for_model_fdid(
                &self.data_dir,
                model_fdid,
            )
            .ok()
            .flatten();
        }
        let model_name = model_path.file_name()?.to_str()?;
        crate::outfit_catalog_db::resolve_cached_skin_fdids_for_model_name(
            &self.data_dir,
            model_name,
        )
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
                .material_texture_fdid(data, material_resource_id, race, sex)
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
        race: u8,
        sex: u8,
    ) -> OutfitResult {
        let mut result = OutfitResult::default();
        for display_id in display_ids {
            let Some(display) = self.display_info(data, display_id) else {
                continue;
            };
            self.merge_display_into_result(&mut result, data, &display, race, sex);
        }
        result
    }

    /// `ItemDisplayInfo.GeosetGroup[group_index]` + 1 (the geoset variant it selects), or
    /// none when that group is 0.
    pub fn display_geoset_variant(&self, display_info_id: u32, group_index: usize) -> Option<u16> {
        let raw = self.display_geoset_raw(display_info_id, group_index)?;
        (raw != 0).then_some(raw + 1)
    }

    /// `ItemDisplayInfo.GeosetGroup[group_index]` itself.
    pub fn display_geoset_raw(&self, display_info_id: u32, group_index: usize) -> Option<u16> {
        let data = self.loaded()?;
        let display = self.display_info(data, display_info_id)?;
        u16::try_from(*display.geoset_groups.get(group_index)?).ok()
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
        let resolved =
            crate::outfit_catalog_db::load_cached_display_info(&self.data_dir, display_info_id)
                .ok()
                .flatten();
        data.display_info_cache
            .lock()
            .unwrap()
            .insert(display_info_id, resolved.clone());
        resolved
    }

    /// The texture of material `material_resource_id` race `race`/sex `sex` wears.
    fn material_texture_fdid(
        &self,
        data: &LoadedOutfitData,
        material_resource_id: u32,
        race: u8,
        sex: u8,
    ) -> Option<u32> {
        let cached = data
            .material_textures_cache
            .lock()
            .unwrap()
            .get(&material_resource_id)
            .cloned();
        let candidates = match cached {
            Some(candidates) => candidates,
            None => {
                let loaded = crate::outfit_catalog_db::load_cached_material_texture_fdids(
                    &self.data_dir,
                    material_resource_id,
                )
                .unwrap_or_default();
                data.material_textures_cache
                    .lock()
                    .unwrap()
                    .insert(material_resource_id, loaded.clone());
                loaded
            }
        };
        data.components.select_texture(&candidates, race, sex)
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
        let resolved =
            crate::outfit_catalog_db::load_cached_model_fdids(&self.data_dir, model_resource_id)
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
        race: u8,
        sex: u8,
    ) {
        let mut seen_item_textures = result.item_textures.iter().copied().collect::<HashSet<_>>();
        for &(component_section, material) in &display.item_materials {
            let Some(fdid) = self.material_texture_fdid(data, material, race, sex) else {
                continue;
            };
            if seen_item_textures.insert((component_section, fdid)) {
                result.item_textures.push((component_section, fdid));
            }
        }

        let mut seen_geoset_overrides = result
            .geoset_overrides
            .iter()
            .copied()
            .collect::<HashSet<_>>();
        for &pair in &display.geoset_overrides {
            if seen_geoset_overrides.insert(pair) {
                result.geoset_overrides.push(pair);
            }
        }

        let mut seen_model_fdids = result.model_fdids.iter().copied().collect::<HashSet<_>>();
        for &model_resource_id in &display.model_resource_ids {
            let Some(model_fdid) = self.select_model_fdid(data, model_resource_id, race, sex)
            else {
                continue;
            };
            let pair = (model_resource_id, model_fdid);
            if seen_model_fdids.insert(pair) {
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
        data.components.select_model(&candidates, race, sex, None)
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
        data.components
            .select_model(&candidates, race, sex, Some(shoulder_index as u8))
    }
}

fn collect_helmet_hide_geoset_groups(
    data: &LoadedOutfitData,
    display: &DisplayInfoResolved,
    race: u8,
) -> Vec<u16> {
    let race_bit = playable_race_bit_selection(race);
    let mut hidden = Vec::new();
    let mut seen = HashSet::new();
    for vis_id in &display.helmet_geoset_vis_ids {
        let Some(rules) = data.helmet_geoset_rules.get(vis_id) else {
            continue;
        };
        for rule in rules {
            if helmet_geoset_rule_matches(*rule, race, race_bit)
                && seen.insert(rule.hide_geoset_group)
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

/// GeosetGroup[1] selects 2101 + value (wowdev.wiki DB/ItemDisplayInfo).
fn head_geoset_secondary_variant(raw_value: i16) -> Option<u16> {
    match raw_value {
        value if value <= 0 => None,
        value => Some(value as u16 + 1),
    }
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
