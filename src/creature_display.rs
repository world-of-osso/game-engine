use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;

/// Per-display creature data: M2 model FDID and up to 3 skin texture FDIDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureDisplay {
    pub model_fdid: u32,
    pub skin_fdids: [u32; 3],
}

/// Bevy resource mapping creature display_id → M2 model FileDataID.
///
/// Built from two wago.tools DB2 CSV exports:
/// - `CreatureDisplayInfo.csv`: display_id → ModelID + TextureVariationFileDataID[0-2]
/// - `CreatureModelData.csv`: ModelID → FileDataID
#[derive(Resource, Default)]
pub struct CreatureDisplayMap {
    entries: HashMap<u32, CreatureDisplay>,
}

impl CreatureDisplayMap {
    /// Load from the two CSV files, joining display_id → model_id → fdid.
    pub fn load(display_info_path: &Path, model_data_path: &Path) -> Self {
        let model_fdids = parse_model_data(model_data_path);
        let entries = parse_display_info(display_info_path, &model_fdids);
        Self { entries }
    }

    /// Look up the M2 FDID for a creature display_id.
    pub fn get_fdid(&self, display_id: u32) -> Option<u32> {
        self.entries.get(&display_id).map(|e| e.model_fdid)
    }

    /// Look up the skin texture FDIDs for a creature display_id.
    pub fn get_skin_fdids(&self, display_id: u32) -> Option<[u32; 3]> {
        self.entries.get(&display_id).map(|e| e.skin_fdids)
    }

    /// Pick a preferred non-empty skin set for a model FDID.
    ///
    /// When multiple display IDs share the same model, we pick the entry with the
    /// most populated texture slots (then lowest display ID for deterministic ties).
    pub fn get_skin_fdids_for_model_fdid(&self, model_fdid: u32) -> Option<[u32; 3]> {
        select_preferred_skin_fdids(
            self.entries
                .iter()
                .filter(|(_, entry)| entry.model_fdid == model_fdid)
                .map(|(display_id, entry)| (*display_id, entry.skin_fdids)),
        )
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
        let model_name = model_path.file_name()?.to_str()?.to_ascii_lowercase();
        select_preferred_skin_fdids(self.entries.iter().filter_map(|(display_id, entry)| {
            let wow_path = game_engine::listfile::lookup_fdid(entry.model_fdid)?;
            let wow_name = Path::new(wow_path)
                .file_name()?
                .to_str()?
                .to_ascii_lowercase();
            (wow_name == model_name).then_some((*display_id, entry.skin_fdids))
        }))
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

fn select_preferred_skin_fdids(
    candidates: impl Iterator<Item = (u32, [u32; 3])>,
) -> Option<[u32; 3]> {
    let mut best: Option<(usize, u32, [u32; 3])> = None;
    for (display_id, skin_fdids) in candidates {
        let filled_slots = skin_fdids.iter().filter(|&&fdid| fdid != 0).count();
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
            best = Some((filled_slots, display_id, skin_fdids));
        }
    }
    best.map(|(_, _, skin_fdids)| skin_fdids)
}

/// Parse CreatureModelData.csv: model_id → file_data_id.
fn parse_model_data(path: &Path) -> HashMap<u32, u32> {
    let mut map = HashMap::new();
    let Ok(content) = std::fs::read_to_string(path) else {
        return map;
    };
    let mut lines = content.lines();
    let Some((id_col, fdid_col)) = find_two_columns(lines.next().unwrap_or(""), "ID", "FileDataID")
    else {
        return map;
    };
    for line in lines {
        insert_model_entry(&mut map, line, id_col, fdid_col);
    }
    map
}

fn insert_model_entry(map: &mut HashMap<u32, u32>, line: &str, id_col: usize, fdid_col: usize) {
    let cols: Vec<&str> = line.split(',').collect();
    let Some(id) = cols.get(id_col).and_then(|s| s.parse::<u32>().ok()) else {
        return;
    };
    let Some(fdid) = cols.get(fdid_col).and_then(|s| s.parse::<u32>().ok()) else {
        return;
    };
    if fdid > 0 {
        map.insert(id, fdid);
    }
}

/// Column indices for CreatureDisplayInfo.csv parsing.
struct DisplayInfoColumns {
    id: usize,
    model: usize,
    tex_var: [usize; 3],
}

/// Parse CreatureDisplayInfo.csv and join with model_fdids to get display_id → CreatureDisplay.
fn parse_display_info(
    path: &Path,
    model_fdids: &HashMap<u32, u32>,
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
        insert_display_entry(&mut map, line, &cols, model_fdids);
    }
    map
}

fn find_display_info_columns(header: &str) -> Option<DisplayInfoColumns> {
    let headers: Vec<&str> = header.split(',').collect();
    let find = |name: &str| headers.iter().position(|h| h.trim() == name);
    Some(DisplayInfoColumns {
        id: find("ID")?,
        model: find("ModelID")?,
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
    model_fdids: &HashMap<u32, u32>,
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
    if let Some(&model_fdid) = model_fdids.get(&model_id) {
        map.insert(
            display_id,
            CreatureDisplay {
                model_fdid,
                skin_fdids: [
                    parse_fdid(col.tex_var[0]),
                    parse_fdid(col.tex_var[1]),
                    parse_fdid(col.tex_var[2]),
                ],
            },
        );
    }
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
            },
        );
        map.entries.insert(
            10,
            CreatureDisplay {
                model_fdid: 9000,
                skin_fdids: [222, 333, 0],
            },
        );
        map.entries.insert(
            30,
            CreatureDisplay {
                model_fdid: 9000,
                skin_fdids: [444, 0, 0],
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
            },
        );
        assert_eq!(map.get_skin_fdids_for_model_fdid(1234), None);
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
}
