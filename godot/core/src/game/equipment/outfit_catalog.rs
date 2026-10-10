use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use crate::component_file_data::ComponentFileData;
use crate::helmet_geoset_data::{
    HelmetGeosetRule, load_forever_helmet_rules, load_helmet_geoset_rules, load_retail_helmet_rules,
};
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
    /// Owned namespaces are in-memory and never query the borrowed NPC cache.
    source_connection: Option<Mutex<rusqlite::Connection>>,
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
    retail_items: OnceLock<Result<Box<OutfitData>, String>>,
    forever_items: OnceLock<Result<Box<OutfitData>, String>>,
}

impl OutfitData {
    pub fn load(data_dir: &Path) -> Self {
        Self {
            data_dir: data_dir.to_path_buf(),
            loaded: OnceLock::new(),
            helmet_cache: None,
            retail_items: OnceLock::new(),
            forever_items: OnceLock::new(),
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

    /// Isolated Retail item/display/resource groups, without borrowed NPC rows.
    pub fn load_owned_retail(&self) -> Result<&Self, String> {
        self.load_owned_catalog(false)
    }

    /// Isolated Forever 70205 item/display/resource groups; no Retail substitution.
    pub fn load_owned_forever_70205(&self) -> Result<&Self, String> {
        self.load_owned_catalog(true)
    }

    fn load_owned_catalog(&self, forever: bool) -> Result<&Self, String> {
        let cell = if forever {
            &self.forever_items
        } else {
            &self.retail_items
        };
        cell.get_or_init(|| self.read_owned_catalog(forever))
            .as_ref()
            .map(Box::as_ref)
            .map_err(Clone::clone)
    }

    fn read_owned_catalog(&self, forever: bool) -> Result<Box<Self>, String> {
        let gear_dir = if forever {
            self.data_dir.join("db2/1.60.1.70205")
        } else {
            self.data_dir.clone()
        };
        let items_dir = if forever {
            gear_dir.join("items")
        } else {
            gear_dir.clone()
        };
        let components_dir = if forever {
            gear_dir.clone()
        } else {
            gear_dir.join("db2/12.1.0.69933")
        };
        let product = if forever {
            crate::asset_product::AssetProduct::Forever
        } else {
            crate::asset_product::AssetProduct::Retail
        };
        let connection =
            crate::outfit_catalog_db::load_owned_outfit_connection(&items_dir, &gear_dir, product)?;
        let helmet_geoset_rules = if forever {
            load_forever_helmet_rules(&gear_dir.join("HelmetGeosetData.csv"))?
        } else {
            load_retail_helmet_rules(&self.data_dir)?
        };
        let data = LoadedOutfitData {
            source_connection: Some(Mutex::new(connection)),
            components: ComponentFileData::load_source(&components_dir)?,
            model_materials: if forever {
                ModelMaterials::new()
            } else {
                load_model_materials(&self.data_dir)?
            },
            helmet_geoset_rules,
            ..LoadedOutfitData::default()
        };
        Ok(Box::new(Self {
            loaded: OnceLock::from(Ok(data)),
            ..Self::load(&self.data_dir)
        }))
    }

    fn load_display_info(&self, display_id: u32) -> Result<Option<DisplayInfoResolved>, String> {
        let data = self.loaded_result()?;
        if let Some(conn) = &data.source_connection {
            return crate::outfit_catalog_db::query_display_info(&conn.lock().unwrap(), display_id);
        }
        crate::outfit_catalog_db::load_cached_display_info(&self.data_dir, display_id)
    }

    fn load_material_fdids(&self, resource: u32) -> Result<Vec<u32>, String> {
        let data = self.loaded_result()?;
        if let Some(conn) = &data.source_connection {
            return crate::outfit_catalog_db::query_material_texture_fdids(
                &conn.lock().unwrap(),
                resource,
            );
        }
        crate::outfit_catalog_db::load_cached_material_texture_fdids(&self.data_dir, resource)
    }

    fn load_model_fdids(&self, resource: u32) -> Result<Vec<u32>, String> {
        let data = self.loaded_result()?;
        if let Some(conn) = &data.source_connection {
            return crate::outfit_catalog_db::query_model_fdids(&conn.lock().unwrap(), resource);
        }
        crate::outfit_catalog_db::load_cached_model_fdids(&self.data_dir, resource)
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
            source_connection: None,
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
        let Some(display) = self.load_display_info(display_id)? else {
            return Ok(None);
        };
        self.check_display_resources(&display)?;
        let mut result = OutfitResult::default();
        self.merge_display_into_result(&mut result, data, &display, race, sex);
        Ok(Some(result))
    }

    /// Resolve models and geosets when an authored NPC bake supplies body pixels.
    /// Component item textures are neither required nor returned; model resources
    /// and their materials remain required in this catalog's source namespace.
    pub fn try_load_baked_display_info(
        &self,
        display_id: u32,
        race: u8,
        sex: u8,
    ) -> Result<Option<OutfitResult>, String> {
        let data = self.loaded_result()?;
        let Some(mut display) = self.load_display_info(display_id)? else {
            return Ok(None);
        };
        // Clear only this owned display's body overlays: the authored bake replaces
        // those pixels, not attached models or their material textures.
        display.item_materials.clear();
        self.check_display_resources(&display)?;
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
            let display = self
                .load_display_info(id)?
                .ok_or_else(|| format!("outfit display {id} missing"))?;
            self.check_display_resources(&display)?;
            self.merge_display_into_result(&mut result, data, &display, race, sex);
        }
        Ok(result)
    }

    fn check_display_resources(&self, display: &DisplayInfoResolved) -> Result<(), String> {
        self.load_required_model_resources(display)?;
        for &(_, id) in &display.item_materials {
            if self.load_material_fdids(id)?.is_empty() {
                return Err(format!("missing TextureFileData material resource {id}"));
            }
        }
        Ok(())
    }

    fn load_required_model_resources(&self, display: &DisplayInfoResolved) -> Result<(), String> {
        for &id in &display.model_resource_ids {
            let fdids = self.load_model_fdids(id)?;
            if fdids.is_empty() {
                return Err(format!("missing ModelFileData model resource {id}"));
            }
        }
        for &id in &display.model_material_resource_ids {
            let fdids = self.load_material_fdids(id)?;
            if fdids.is_empty() {
                return Err(format!("missing TextureFileData material resource {id}"));
            }
        }
        Ok(())
    }

    pub fn resolve_item_display_id(&self, item_id: u32) -> Result<u32, String> {
        let data = self.loaded_result()?;
        if let Some(conn) = &data.source_connection {
            return crate::outfit_catalog_db::query_item_display_id(&conn.lock().unwrap(), item_id);
        }
        let conn = crate::cache_sqlite::open_read_only(&data.cache_path)?;
        crate::outfit_catalog_db::query_item_display_id(&conn, item_id)
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
        let Some(display) = self.load_display_info(display_info_id)? else {
            return Ok(None);
        };
        self.load_required_model_resources(&display)?;
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

    pub fn load_column_products(
        &self,
        display_info_id: u32,
        model_index: usize,
    ) -> Result<
        (
            crate::asset_product::AssetProduct,
            Option<crate::asset_product::AssetProduct>,
        ),
        String,
    > {
        let display = self
            .load_display_info(display_info_id)?
            .ok_or_else(|| format!("Display {display_info_id} missing"))?;
        let model = *display
            .model_resource_columns
            .get(model_index)
            .ok_or_else(|| {
                format!("Display {display_info_id} has no model column {model_index}")
            })?;
        let material = display.model_material_resource_columns[model_index];
        let product = self.load_resource_product("model_to_fdid", model)?;
        let material_product = if material == 0 {
            None
        } else {
            Some(self.load_resource_product("material_textures", material)?)
        };
        Ok((product, material_product))
    }

    pub fn load_resource_product(
        &self,
        table: &str,
        resource: u32,
    ) -> Result<crate::asset_product::AssetProduct, String> {
        let data = self.loaded_result()?;
        if let Some(conn) = &data.source_connection {
            return query_resource_product(
                &conn.lock().expect("owned outfit namespace"),
                table,
                resource,
            );
        }
        let conn = crate::cache_sqlite::open_read_only(&data.cache_path)?;
        query_resource_product(&conn, table, resource)
    }

    pub fn try_resolve_column_models(
        &self,
        display_info_id: u32,
        race: u8,
        sex: u8,
    ) -> Result<Vec<(usize, u32, [u32; 3])>, String> {
        let data = self.loaded_result()?;
        let Some(display) = self.load_display_info(display_info_id)? else {
            return Ok(Vec::new());
        };
        self.load_required_model_resources(&display)?;
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
        let resolved = self.load_display_info(display_info_id).ok().flatten();
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
                let loaded = self
                    .load_material_fdids(material_resource_id)
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
        let resolved = self.load_model_fdids(model_resource_id).unwrap_or_default();
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

fn query_resource_product(
    conn: &rusqlite::Connection,
    table: &str,
    resource: u32,
) -> Result<crate::asset_product::AssetProduct, String> {
    conn.query_row(
        "SELECT source_product FROM asset_resource_sources WHERE table_name = ?1 AND resource_id = ?2",
        rusqlite::params![table, resource], |row| row.get(0),
    ).map_err(|error| format!("Metadata source for {table} resource {resource}: {error}"))
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
