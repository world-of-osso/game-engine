use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use bevy::prelude::*;

const NAMED_MODEL_FDID_CACHE_PATH: &str = "data/named-model-fdid-cache.csv";
const NAMED_MODEL_SKIN_CACHE_PATH: &str = "data/named-model-skin-cache.csv";

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
        let model_data = parse_model_data(model_data_path);
        let entries = parse_display_info(display_info_path, &model_data);
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
        let stem = model_path.file_stem()?.to_str()?;
        if let Ok(model_fdid) = stem.parse::<u32>() {
            return self.get_skin_fdids_for_model_fdid(model_fdid);
        }
        if let Some(skin_fdids) = cached_named_model_skin_resolution(model_path) {
            return (skin_fdids != [0, 0, 0]).then_some(skin_fdids);
        }
        if let Some(model_fdid) = cached_named_model_fdid(model_path) {
            let skin_fdids = self
                .get_skin_fdids_for_model_fdid(model_fdid)
                .unwrap_or([0, 0, 0]);
            remember_named_model_skin_fdids(model_path, skin_fdids);
            return (skin_fdids != [0, 0, 0]).then_some(skin_fdids);
        }
        if let Some(wow_path) = crate::character_models::known_wow_path_for_local_model(model_path)
            && let Some(model_fdid) = game_engine::listfile::lookup_path(wow_path)
        {
            remember_named_model_fdid(model_path, model_fdid);
            let skin_fdids = self
                .get_skin_fdids_for_model_fdid(model_fdid)
                .unwrap_or([0, 0, 0]);
            remember_named_model_skin_fdids(model_path, skin_fdids);
            return (skin_fdids != [0, 0, 0]).then_some(skin_fdids);
        }
        let model_name = model_path.file_name()?.to_str()?.to_ascii_lowercase();
        let resolved =
            select_preferred_named_model(self.entries.iter().filter_map(|(display_id, entry)| {
                let wow_path = game_engine::listfile::lookup_fdid(entry.model_fdid)?;
                let wow_name = Path::new(wow_path)
                    .file_name()?
                    .to_str()?
                    .to_ascii_lowercase();
                (wow_name == model_name).then_some((
                    *display_id,
                    entry.model_fdid,
                    entry.skin_fdids,
                ))
            }));
        if let Some((model_fdid, skin_fdids)) = resolved {
            remember_named_model_fdid(model_path, model_fdid);
            remember_named_model_skin_fdids(model_path, skin_fdids);
            return Some(skin_fdids);
        }
        remember_named_model_skin_fdids(model_path, [0, 0, 0]);
        None
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
    load_named_model_cache_lines(NAMED_MODEL_FDID_CACHE_PATH, |line, cache| {
        let Some((name, fdid)) = line.split_once(';') else {
            return;
        };
        let Ok(fdid) = fdid.parse::<u32>() else {
            return;
        };
        cache.insert(name.to_ascii_lowercase(), fdid);
    })
}

fn load_named_model_skin_cache() -> HashMap<String, [u32; 3]> {
    load_named_model_cache_lines(NAMED_MODEL_SKIN_CACHE_PATH, |line, cache| {
        let mut parts = line.split(';');
        let Some(name) = parts.next() else {
            return;
        };
        let Some(a) = parts.next().and_then(|v| v.parse::<u32>().ok()) else {
            return;
        };
        let Some(b) = parts.next().and_then(|v| v.parse::<u32>().ok()) else {
            return;
        };
        let Some(c) = parts.next().and_then(|v| v.parse::<u32>().ok()) else {
            return;
        };
        cache.insert(name.to_ascii_lowercase(), [a, b, c]);
    })
}

fn load_named_model_cache_lines<T>(
    path: &str,
    mut parse: impl FnMut(&str, &mut HashMap<String, T>),
) -> HashMap<String, T> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return HashMap::new();
    };
    let mut cache = HashMap::new();
    for line in content.lines() {
        parse(line, &mut cache);
    }
    cache
}

fn append_named_model_fdid_cache_entry(name: &str, fdid: u32) -> Result<(), String> {
    append_named_model_cache_line(NAMED_MODEL_FDID_CACHE_PATH, &format!("{name};{fdid}"))
}

fn append_named_model_skin_cache_entry(name: &str, skin_fdids: [u32; 3]) -> Result<(), String> {
    append_named_model_cache_line(
        NAMED_MODEL_SKIN_CACHE_PATH,
        &format!(
            "{name};{};{};{}",
            skin_fdids[0], skin_fdids[1], skin_fdids[2]
        ),
    )
}

fn append_named_model_cache_line(path: &str, line: &str) -> Result<(), String> {
    let cache_path = Path::new(path);
    let Some(parent) = cache_path.parent() else {
        return Err(format!("missing parent for {}", cache_path.display()));
    };
    std::fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(cache_path)
        .map_err(|e| format!("open {}: {e}", cache_path.display()))?;
    writeln!(file, "{line}").map_err(|e| format!("write {}: {e}", cache_path.display()))
}

#[derive(Clone, Copy)]
struct CreatureModelData {
    fdid: u32,
    scale_milli: u32,
}

/// Parse CreatureModelData.csv: model_id → file_data_id + model scale.
fn parse_model_data(path: &Path) -> HashMap<u32, CreatureModelData> {
    let mut map = HashMap::new();
    let Ok(content) = std::fs::read_to_string(path) else {
        return map;
    };
    let mut lines = content.lines();
    let header = lines.next().unwrap_or("");
    let Some((id_col, fdid_col)) = find_two_columns(header, "ID", "FileDataID") else {
        return map;
    };
    let scale_col = header.split(',').position(|col| col.trim() == "ModelScale");
    for line in lines {
        insert_model_entry(&mut map, line, id_col, fdid_col, scale_col);
    }
    map
}

fn insert_model_entry(
    map: &mut HashMap<u32, CreatureModelData>,
    line: &str,
    id_col: usize,
    fdid_col: usize,
    scale_col: Option<usize>,
) {
    let cols: Vec<&str> = line.split(',').collect();
    let Some(id) = cols.get(id_col).and_then(|s| s.parse::<u32>().ok()) else {
        return;
    };
    let Some(fdid) = cols.get(fdid_col).and_then(|s| s.parse::<u32>().ok()) else {
        return;
    };
    if fdid > 0 {
        map.insert(
            id,
            CreatureModelData {
                fdid,
                scale_milli: parse_scale_milli(scale_col.and_then(|idx| cols.get(idx).copied())),
            },
        );
    }
}

/// Column indices for CreatureDisplayInfo.csv parsing.
struct DisplayInfoColumns {
    id: usize,
    model: usize,
    scale: usize,
    tex_var: [usize; 3],
}

/// Parse CreatureDisplayInfo.csv and join with model_fdids to get display_id → CreatureDisplay.
fn parse_display_info(
    path: &Path,
    model_data: &HashMap<u32, CreatureModelData>,
) -> HashMap<u32, CreatureDisplay> {
    let mut map = HashMap::new();
    let Ok(content) = std::fs::read_to_string(path) else {
        return map;
    };
    let mut lines = content.lines();
    let header = lines.next().unwrap_or("");
    let Some(cols) = find_display_info_columns(header) else {
        return map;
    };
    for line in lines {
        insert_display_entry(&mut map, line, &cols, model_data);
    }
    map
}

fn find_display_info_columns(header: &str) -> Option<DisplayInfoColumns> {
    let headers: Vec<&str> = header.split(',').collect();
    let find = |name: &str| headers.iter().position(|h| h.trim() == name);
    Some(DisplayInfoColumns {
        id: find("ID")?,
        model: find("ModelID")?,
        scale: find("CreatureModelScale")?,
        tex_var: [
            find("TextureVariationFileDataID_0")?,
            find("TextureVariationFileDataID_1")?,
            find("TextureVariationFileDataID_2")?,
        ],
    })
}

fn insert_display_entry(
    map: &mut HashMap<u32, CreatureDisplay>,
    line: &str,
    col: &DisplayInfoColumns,
    model_data: &HashMap<u32, CreatureModelData>,
) {
    let cols: Vec<&str> = line.split(',').collect();
    let Some(display_id) = cols.get(col.id).and_then(|s| s.parse::<u32>().ok()) else {
        return;
    };
    let Some(model_id) = cols.get(col.model).and_then(|s| s.parse::<u32>().ok()) else {
        return;
    };
    let parse_fdid = |idx: usize| {
        cols.get(idx)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0)
    };
    let display_scale_milli = parse_scale_milli(cols.get(col.scale).copied());
    if let Some(model) = model_data.get(&model_id) {
        map.insert(
            display_id,
            CreatureDisplay {
                model_fdid: model.fdid,
                skin_fdids: [
                    parse_fdid(col.tex_var[0]),
                    parse_fdid(col.tex_var[1]),
                    parse_fdid(col.tex_var[2]),
                ],
                scale_milli: combine_scale_milli(display_scale_milli, model.scale_milli),
            },
        );
    }
}

fn parse_scale_milli(value: Option<&str>) -> u32 {
    let scale = value
        .and_then(|s| s.parse::<f32>().ok())
        .filter(|scale| *scale > 0.0)
        .unwrap_or(1.0);
    (scale * 1000.0).round() as u32
}

fn combine_scale_milli(display_scale_milli: u32, model_scale_milli: u32) -> u32 {
    let display = display_scale_milli.max(1);
    let model = model_scale_milli.max(1);
    ((display as u64 * model as u64 + 500) / 1000) as u32
}

/// Find column indices for two named headers. Returns None if either is missing.
fn find_two_columns(header: &str, name_a: &str, name_b: &str) -> Option<(usize, usize)> {
    let a = header.split(',').position(|col| col.trim() == name_a)?;
    let b = header.split(',').position(|col| col.trim() == name_b)?;
    Some((a, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_two_columns_works() {
        assert_eq!(
            find_two_columns("ID,ModelID,Flags", "ID", "ModelID"),
            Some((0, 1))
        );
        assert_eq!(find_two_columns("ID,ModelID,Flags", "ID", "Nope"), None);
    }

    #[test]
    fn empty_default_returns_none() {
        let map = CreatureDisplayMap::default();
        assert_eq!(map.get_fdid(123), None);
        assert_eq!(map.get_skin_fdids(123), None);
        assert_eq!(map.get_scale(123), None);
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn load_real_csvs() {
        let display_path = Path::new("data/CreatureDisplayInfo.csv");
        let model_path = Path::new("data/CreatureModelData.csv");
        if !display_path.exists() || !model_path.exists() {
            return; // skip if CSVs not downloaded yet
        }
        let map = CreatureDisplayMap::load(display_path, model_path);
        assert!(map.len() > 1000, "Expected many entries, got {}", map.len());

        // Display ID 4 → ModelID 8231 → FDID 1113034 (no skin textures)
        assert_eq!(map.get_fdid(4), Some(1113034));
        assert_eq!(map.get_skin_fdids(4), Some([0, 0, 0]));
        assert_eq!(map.get_scale(4), Some(1.0));
    }

    #[test]
    fn load_real_csvs_skin_fdids() {
        let display_path = Path::new("data/CreatureDisplayInfo.csv");
        let model_path = Path::new("data/CreatureModelData.csv");
        if !display_path.exists() || !model_path.exists() {
            return;
        }
        let map = CreatureDisplayMap::load(display_path, model_path);

        // Display ID 150 has all 3 TextureVariation slots populated
        assert_eq!(map.get_skin_fdids(150), Some([1245235, 1245243, 1245229]));
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
