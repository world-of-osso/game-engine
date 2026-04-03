use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use bevy::prelude::*;

#[path = "creature_display_cache.rs"]
mod cache;
#[path = "creature_named_model_cache.rs"]
mod named_cache;

const NAMED_MODEL_CACHE_PATH: &str = "cache/named-model-lookups.sqlite";

static NAMED_MODEL_FDID_CACHE: OnceLock<Mutex<HashMap<String, u32>>> = OnceLock::new();
static NAMED_MODEL_SKIN_CACHE: OnceLock<Mutex<HashMap<String, [u32; 3]>>> = OnceLock::new();

/// Per-display creature data: M2 model FDID and up to 3 skin texture FDIDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureDisplay {
    pub model_fdid: u32,
    pub skin_fdids: [u32; 3],
    pub scale_milli: u32,
}

/// Bevy resource for creature display lookups.
///
/// Queries `cache/creature_display.sqlite` on demand instead of loading all
/// entries into memory. Build the cache with:
/// `cargo run --bin creature_display_cache_import`
#[derive(Resource, Default)]
pub struct CreatureDisplayMap;

impl CreatureDisplayMap {
    /// Look up the M2 FDID for a creature display_id.
    pub fn get_fdid(&self, display_id: u32) -> Option<u32> {
        cache::query_display(display_id).map(|e| e.model_fdid)
    }

    /// Look up the skin texture FDIDs for a creature display_id.
    pub fn get_skin_fdids(&self, display_id: u32) -> Option<[u32; 3]> {
        cache::query_display(display_id).map(|e| e.skin_fdids)
    }

    /// Look up the final creature display scale.
    pub fn get_scale(&self, display_id: u32) -> Option<f32> {
        cache::query_display(display_id).map(|e| e.scale_milli as f32 / 1000.0)
    }

    /// Pick a preferred non-empty skin set for a model FDID.
    ///
    /// Uses a precomputed table built during cache import: for each model FDID,
    /// the entry with the most populated texture slots (lowest display ID for ties).
    pub fn get_skin_fdids_for_model_fdid(&self, model_fdid: u32) -> Option<[u32; 3]> {
        cache::query_preferred_skins(model_fdid)
    }

    /// Resolve preferred skin texture FDIDs from a local model path.
    ///
    /// Supports both FDID-named files (`123456.m2`) and named files (`boar.m2`).
    /// Named files are resolved via `community-listfile.csv` filename matching.
    pub fn resolve_skin_fdids_for_model_path(&self, model_path: &Path) -> Option<[u32; 3]> {
        if let Some(skin_fdids) = resolve_numeric_model_skin_fdids(self, model_path) {
            return Some(skin_fdids);
        }
        resolve_named_model_skin_fdids(self, model_path)
    }
}

/// Import creature display cache from CSVs. Called by the import binary.
pub fn import_creature_display_cache() -> Result<std::path::PathBuf, String> {
    cache::import_creature_display_cache()
}

fn resolve_numeric_model_skin_fdids(
    map: &CreatureDisplayMap,
    model_path: &Path,
) -> Option<[u32; 3]> {
    let stem = model_path.file_stem()?.to_str()?;
    let model_fdid = stem.parse::<u32>().ok()?;
    map.get_skin_fdids_for_model_fdid(model_fdid)
}

fn resolve_named_model_skin_fdids(map: &CreatureDisplayMap, model_path: &Path) -> Option<[u32; 3]> {
    if let Some(skin_fdids) = cached_named_model_skin_resolution(model_path) {
        return (skin_fdids != [0, 0, 0]).then_some(skin_fdids);
    }
    if let Some(skin_fdids) = resolve_cached_named_model_skin_fdids(map, model_path) {
        return Some(skin_fdids);
    }
    if let Some(skin_fdids) = resolve_known_wow_path_skin_fdids(map, model_path) {
        return Some(skin_fdids);
    }
    if let Some(skin_fdids) = resolve_named_model_skin_fdids_via_listfile(map, model_path) {
        return Some(skin_fdids);
    }
    remember_named_model_skin_fdids(model_path, [0, 0, 0]);
    None
}

fn resolve_cached_named_model_skin_fdids(
    map: &CreatureDisplayMap,
    model_path: &Path,
) -> Option<[u32; 3]> {
    let model_fdid = cached_named_model_fdid(model_path)?;
    let skin_fdids = map
        .get_skin_fdids_for_model_fdid(model_fdid)
        .unwrap_or([0, 0, 0]);
    remember_named_model_skin_fdids(model_path, skin_fdids);
    (skin_fdids != [0, 0, 0]).then_some(skin_fdids)
}

fn resolve_known_wow_path_skin_fdids(
    map: &CreatureDisplayMap,
    model_path: &Path,
) -> Option<[u32; 3]> {
    let wow_path = crate::character_models::known_wow_path_for_local_model(model_path)?;
    let model_fdid = game_engine::listfile::lookup_path(wow_path)?;
    remember_named_model_fdid(model_path, model_fdid);
    let skin_fdids = map
        .get_skin_fdids_for_model_fdid(model_fdid)
        .unwrap_or([0, 0, 0]);
    remember_named_model_skin_fdids(model_path, skin_fdids);
    (skin_fdids != [0, 0, 0]).then_some(skin_fdids)
}

/// Fallback: scan all distinct model FDIDs via listfile to find one whose
/// filename matches the local model path. Only called once per unique model
/// name (result is cached in named-model-lookups.sqlite).
fn resolve_named_model_skin_fdids_via_listfile(
    map: &CreatureDisplayMap,
    model_path: &Path,
) -> Option<[u32; 3]> {
    let model_name = model_path.file_name()?.to_str()?.to_ascii_lowercase();
    let model_fdids = cache::query_distinct_model_fdids();
    let matched_fdid = model_fdids.into_iter().find(|&fdid| {
        game_engine::listfile::lookup_fdid(fdid)
            .and_then(|wow_path| {
                Path::new(wow_path)
                    .file_name()?
                    .to_str()
                    .map(|name| name.to_ascii_lowercase())
            })
            .is_some_and(|name| name == model_name)
    })?;
    remember_named_model_fdid(model_path, matched_fdid);
    let skin_fdids = map
        .get_skin_fdids_for_model_fdid(matched_fdid)
        .unwrap_or([0, 0, 0]);
    remember_named_model_skin_fdids(model_path, skin_fdids);
    (skin_fdids != [0, 0, 0]).then_some(skin_fdids)
}

pub(crate) fn remember_named_model_fdid_for_wow_path(wow_path: &str, fdid: u32) {
    let Some(file_name) = Path::new(wow_path)
        .file_name()
        .and_then(|name| name.to_str())
    else {
        return;
    };
    remember_named_model_fdid_by_name(file_name, fdid);
}

pub(crate) fn cached_named_model_fdid_for_wow_path(wow_path: &str) -> Option<u32> {
    let file_name = Path::new(wow_path).file_name()?.to_str()?;
    named_model_fdid_cache()
        .lock()
        .unwrap()
        .get(&file_name.to_ascii_lowercase())
        .copied()
}

fn cached_named_model_fdid(model_path: &Path) -> Option<u32> {
    let key = named_model_cache_key(model_path)?;
    named_model_fdid_cache().lock().unwrap().get(&key).copied()
}

fn remember_named_model_fdid(model_path: &Path, fdid: u32) {
    let Some(key) = named_model_cache_key(model_path) else {
        return;
    };
    remember_named_model_fdid_by_name(&key, fdid);
}

fn remember_named_model_fdid_by_name(name: &str, fdid: u32) {
    let key = name.to_ascii_lowercase();
    let mut cache = named_model_fdid_cache().lock().unwrap();
    if cache.contains_key(&key) {
        return;
    }
    cache.insert(key.clone(), fdid);
    if let Err(err) = append_named_model_fdid_cache_entry(&key, fdid) {
        eprintln!("Failed to persist named model FDID cache entry {key}: {err}");
    }
}

fn cached_named_model_skin_resolution(model_path: &Path) -> Option<[u32; 3]> {
    let key = named_model_cache_key(model_path)?;
    named_model_skin_cache().lock().unwrap().get(&key).copied()
}

fn remember_named_model_skin_fdids(model_path: &Path, skin_fdids: [u32; 3]) {
    let Some(key) = named_model_cache_key(model_path) else {
        return;
    };
    let mut cache = named_model_skin_cache().lock().unwrap();
    if cache.contains_key(&key) {
        return;
    }
    cache.insert(key.clone(), skin_fdids);
    if let Err(err) = append_named_model_skin_cache_entry(&key, skin_fdids) {
        eprintln!("Failed to persist named model skin cache entry {key}: {err}");
    }
}

fn named_model_cache_key(model_path: &Path) -> Option<String> {
    model_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_ascii_lowercase())
}

fn named_model_fdid_cache() -> &'static Mutex<HashMap<String, u32>> {
    NAMED_MODEL_FDID_CACHE.get_or_init(|| Mutex::new(load_named_model_fdid_cache()))
}

fn named_model_skin_cache() -> &'static Mutex<HashMap<String, [u32; 3]>> {
    NAMED_MODEL_SKIN_CACHE.get_or_init(|| Mutex::new(load_named_model_skin_cache()))
}

fn load_named_model_fdid_cache() -> HashMap<String, u32> {
    named_cache::load_named_model_fdid_cache(Path::new(NAMED_MODEL_CACHE_PATH)).unwrap_or_default()
}

fn load_named_model_skin_cache() -> HashMap<String, [u32; 3]> {
    named_cache::load_named_model_skin_cache(Path::new(NAMED_MODEL_CACHE_PATH)).unwrap_or_default()
}

fn append_named_model_fdid_cache_entry(name: &str, fdid: u32) -> Result<(), String> {
    named_cache::remember_named_model_fdid(Path::new(NAMED_MODEL_CACHE_PATH), name, fdid)
}

fn append_named_model_skin_cache_entry(name: &str, skin_fdids: [u32; 3]) -> Result<(), String> {
    named_cache::remember_named_model_skin(Path::new(NAMED_MODEL_CACHE_PATH), name, skin_fdids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_returns_none() {
        let map = CreatureDisplayMap;
        assert_eq!(map.get_fdid(123), None);
        assert_eq!(map.get_skin_fdids(123), None);
        assert_eq!(map.get_scale(123), None);
    }

    #[test]
    fn resolve_skin_fdids_for_local_boar_model() {
        let boar_model = Path::new("data/models/boar.m2");
        if !boar_model.exists() || !cache::creature_display_cache_path().exists() {
            return;
        }
        let map = CreatureDisplayMap;
        let skin_fdids = map.resolve_skin_fdids_for_model_path(boar_model);
        assert!(
            skin_fdids.is_some_and(|fdids| fdids.iter().any(|fdid| *fdid != 0)),
            "expected non-empty skin FDIDs for boar.m2"
        );
    }

    #[test]
    fn cached_named_model_fdid_for_wow_path_uses_filename_cache() {
        remember_named_model_fdid_for_wow_path("character/human/male/humanmale_hd.m2", 1011653);
        remember_named_model_fdid_for_wow_path("character/human/male/humanmale_hd00.skin", 1012983);
        assert_eq!(
            cached_named_model_fdid_for_wow_path("character/human/male/humanmale_hd.m2"),
            Some(1011653)
        );
        assert_eq!(
            cached_named_model_fdid_for_wow_path("character/human/male/humanmale_hd00.skin"),
            Some(1012983)
        );
    }
}
