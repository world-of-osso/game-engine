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

/// Bevy resource mapping creature display_id → M2 model FileDataID.
///
/// Built from two wago.tools DB2 CSV exports:
/// - `CreatureDisplayInfo.csv`: display_id → ModelID + TextureVariationFileDataID[0-2]
/// - `CreatureModelData.csv`: ModelID → FileDataID
#[derive(Resource, Default)]
pub struct CreatureDisplayMap {
    entries: HashMap<u32, CreatureDisplay>,
    preferred_skins_by_model_fdid: HashMap<u32, [u32; 3]>,
}

impl CreatureDisplayMap {
    /// Load from the two CSV files, joining display_id → model_id → fdid.
    pub fn load(display_info_path: &Path, model_data_path: &Path) -> Self {
        let entries = match cache::load_creature_display_entries(display_info_path, model_data_path)
        {
            Ok(entries) => entries,
            Err(err) => {
                warn!("Failed to load creature display cache: {err}");
                cache::load_creature_display_entries_uncached(display_info_path, model_data_path)
                    .unwrap_or_default()
            }
        };
        let preferred_skins_by_model_fdid = build_preferred_skins_by_model_fdid(&entries);
        Self {
            entries,
            preferred_skins_by_model_fdid,
        }
    }

    /// Look up the M2 FDID for a creature display_id.
    pub fn get_fdid(&self, display_id: u32) -> Option<u32> {
        self.entries.get(&display_id).map(|e| e.model_fdid)
    }

    /// Look up the skin texture FDIDs for a creature display_id.
    pub fn get_skin_fdids(&self, display_id: u32) -> Option<[u32; 3]> {
        self.entries.get(&display_id).map(|e| e.skin_fdids)
    }

    /// Look up the final creature display scale.
    pub fn get_scale(&self, display_id: u32) -> Option<f32> {
        self.entries
            .get(&display_id)
            .map(|e| e.scale_milli as f32 / 1000.0)
    }

    /// Pick a preferred non-empty skin set for a model FDID.
    ///
    /// When multiple display IDs share the same model, we pick the entry with the
    /// most populated texture slots (then lowest display ID for deterministic ties).
    pub fn get_skin_fdids_for_model_fdid(&self, model_fdid: u32) -> Option<[u32; 3]> {
        self.preferred_skins_by_model_fdid
            .get(&model_fdid)
            .copied()
            .or_else(|| preferred_skin_fdids_for_model_fdid(&self.entries, model_fdid))
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

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Load from `data/` directory CSVs, logging results. Returns empty map if files missing.
    pub fn load_from_data_dir() -> Self {
        let di = Path::new("data/CreatureDisplayInfo.csv");
        let md = Path::new("data/CreatureModelData.csv");
        if !di.exists() || !md.exists() {
            warn!("Creature display CSVs not found, NPC models won't resolve");
            return Self::default();
        }
        let map = Self::load(di, md);
        info!("Loaded {} creature display→FDID mappings", map.len());
        map
    }
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
    if let Some(skin_fdids) = resolve_named_model_skin_fdids_from_entries(map, model_path) {
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

fn resolve_named_model_skin_fdids_from_entries(
    map: &CreatureDisplayMap,
    model_path: &Path,
) -> Option<[u32; 3]> {
    let model_name = model_path.file_name()?.to_str()?.to_ascii_lowercase();
    let resolved =
        select_preferred_named_model(map.entries.iter().filter_map(|(display_id, entry)| {
            let wow_path = game_engine::listfile::lookup_fdid(entry.model_fdid)?;
            let wow_name = Path::new(wow_path)
                .file_name()?
                .to_str()?
                .to_ascii_lowercase();
            (wow_name == model_name).then_some((*display_id, entry.model_fdid, entry.skin_fdids))
        }))?;
    remember_named_model_fdid(model_path, resolved.0);
    remember_named_model_skin_fdids(model_path, resolved.1);
    Some(resolved.1)
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

fn build_preferred_skins_by_model_fdid(
    entries: &HashMap<u32, CreatureDisplay>,
) -> HashMap<u32, [u32; 3]> {
    let mut best: HashMap<u32, (usize, u32, [u32; 3])> = HashMap::new();
    for (&display_id, entry) in entries {
        let filled_slots = entry.skin_fdids.iter().filter(|&&fdid| fdid != 0).count();
        if filled_slots == 0 {
            continue;
        }
        match best.get_mut(&entry.model_fdid) {
            Some((best_filled, best_display_id, best_skin_fdids)) => {
                if filled_slots > *best_filled
                    || (filled_slots == *best_filled && display_id < *best_display_id)
                {
                    *best_filled = filled_slots;
                    *best_display_id = display_id;
                    *best_skin_fdids = entry.skin_fdids;
                }
            }
            None => {
                best.insert(
                    entry.model_fdid,
                    (filled_slots, display_id, entry.skin_fdids),
                );
            }
        }
    }
    best.into_iter()
        .map(|(model_fdid, (_, _, skin_fdids))| (model_fdid, skin_fdids))
        .collect()
}

fn preferred_skin_fdids_for_model_fdid(
    entries: &HashMap<u32, CreatureDisplay>,
    model_fdid: u32,
) -> Option<[u32; 3]> {
    let mut best: Option<(usize, u32, [u32; 3])> = None;
    for (&display_id, entry) in entries {
        if entry.model_fdid != model_fdid {
            continue;
        }
        let filled_slots = entry.skin_fdids.iter().filter(|&&fdid| fdid != 0).count();
        if filled_slots == 0 {
            continue;
        }
        let better = match best {
            None => true,
            Some((best_filled, best_display_id, _)) => {
                filled_slots > best_filled
                    || (filled_slots == best_filled && display_id < best_display_id)
            }
        };
        if better {
            best = Some((filled_slots, display_id, entry.skin_fdids));
        }
    }
    best.map(|(_, _, skin_fdids)| skin_fdids)
}

fn select_preferred_named_model(
    candidates: impl Iterator<Item = (u32, u32, [u32; 3])>,
) -> Option<(u32, [u32; 3])> {
    let mut best: Option<(usize, u32, u32, [u32; 3])> = None;
    for (display_id, model_fdid, skin_fdids) in candidates {
        let filled_slots = skin_fdids.iter().filter(|&&fdid| fdid != 0).count();
        if filled_slots == 0 {
            continue;
        }
        let better = match best {
            None => true,
            Some((best_filled, best_display_id, _, _)) => {
                filled_slots > best_filled
                    || (filled_slots == best_filled && display_id < best_display_id)
            }
        };
        if better {
            best = Some((filled_slots, display_id, model_fdid, skin_fdids));
        }
    }
    best.map(|(_, _, model_fdid, skin_fdids)| (model_fdid, skin_fdids))
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
    fn empty_default_returns_none() {
        let map = CreatureDisplayMap::default();
        assert_eq!(map.get_fdid(123), None);
        assert_eq!(map.get_skin_fdids(123), None);
        assert_eq!(map.get_scale(123), None);
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn preferred_skin_fdids_for_model_fdid_chooses_most_populated() {
        let mut map = CreatureDisplayMap::default();
        map.entries.insert(
            20,
            CreatureDisplay {
                model_fdid: 9000,
                skin_fdids: [111, 0, 0],
                scale_milli: 1000,
            },
        );
        map.entries.insert(
            10,
            CreatureDisplay {
                model_fdid: 9000,
                skin_fdids: [222, 333, 0],
                scale_milli: 1000,
            },
        );
        map.entries.insert(
            30,
            CreatureDisplay {
                model_fdid: 9000,
                skin_fdids: [444, 0, 0],
                scale_milli: 1000,
            },
        );

        assert_eq!(map.get_skin_fdids_for_model_fdid(9000), Some([222, 333, 0]));
    }

    #[test]
    fn preferred_skin_fdids_for_model_fdid_returns_none_for_empty_slots() {
        let mut map = CreatureDisplayMap::default();
        map.entries.insert(
            1,
            CreatureDisplay {
                model_fdid: 1234,
                skin_fdids: [0, 0, 0],
                scale_milli: 1000,
            },
        );
        assert_eq!(map.get_skin_fdids_for_model_fdid(1234), None);
    }

    #[test]
    fn get_scale_returns_display_scale() {
        let mut map = CreatureDisplayMap::default();
        map.entries.insert(
            42,
            CreatureDisplay {
                model_fdid: 1234,
                skin_fdids: [0, 0, 0],
                scale_milli: 550,
            },
        );
        assert_eq!(map.get_scale(42), Some(0.55));
    }

    #[test]
    fn resolve_skin_fdids_for_local_boar_model() {
        let display_path = Path::new("data/CreatureDisplayInfo.csv");
        let model_path = Path::new("data/CreatureModelData.csv");
        let boar_model = Path::new("data/models/boar.m2");
        if !display_path.exists() || !model_path.exists() || !boar_model.exists() {
            return;
        }
        let map = CreatureDisplayMap::load(display_path, model_path);
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
