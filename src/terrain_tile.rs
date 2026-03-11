//! ADT tile path resolution and coordinate utilities.

use std::path::{Path, PathBuf};

use crate::asset::adt::{self, CHUNK_SIZE};
use crate::terrain::DoodadLod;

/// Distance threshold in tiles: tiles farther than this use LOD1 doodads.
const LOD1_TILE_DISTANCE: u32 = 2;
/// Distance threshold in tiles: tiles farther than this use LOD2 doodads.
const LOD2_TILE_DISTANCE: u32 = 4;

/// Compute the LOD level for a tile based on Chebyshev distance from player.
pub(crate) fn tile_lod_for_distance(ty: u32, tx: u32, center_y: u32, center_x: u32) -> DoodadLod {
    let dy = ty.abs_diff(center_y);
    let dx = tx.abs_diff(center_x);
    let dist = dy.max(dx);
    if dist > LOD2_TILE_DISTANCE {
        DoodadLod::Lod2
    } else if dist > LOD1_TILE_DISTANCE {
        DoodadLod::Lod1
    } else {
        DoodadLod::Full
    }
}

/// WoW tile size in yards: 16 chunks × 33.33 yards/chunk = 533.33.
pub(crate) const TILE_SIZE: f32 = CHUNK_SIZE * 16.0;

/// Convert a Bevy world position to WoW ADT tile coordinates.
///
/// Returns (row, col) matching the ADT filename convention: `map_{row}_{col}.adt`.
/// WoW MCNK stores position as [Y, X, Z]; Bevy maps: bx=wow_x, bz=-wow_y.
/// ADT filename row = f(wow_x) = f(bx), col = f(wow_y) = f(-bz).
pub fn bevy_to_tile_coords(bx: f32, bz: f32) -> (u32, u32) {
    let center = 32.0 * TILE_SIZE;
    let row = ((center - bx) / TILE_SIZE).floor() as i32;
    let col = ((center + bz) / TILE_SIZE).floor() as i32;
    (row.clamp(0, 63) as u32, col.clamp(0, 63) as u32)
}

/// Resolve the local file path for an ADT tile via listfile FDID lookup.
pub(crate) fn resolve_tile_path(
    map_name: &str,
    tile_y: u32,
    tile_x: u32,
) -> Result<PathBuf, String> {
    let wow_path = format!("world/maps/{map_name}/{map_name}_{tile_y}_{tile_x}.adt");
    let fdid = game_engine::listfile::lookup_path(&wow_path)
        .ok_or_else(|| format!("Tile ({tile_y},{tile_x}) not in listfile: {wow_path}"))?;
    let local = PathBuf::from(format!("data/terrain/{map_name}_{tile_y}_{tile_x}.adt"));
    if local.exists() {
        return Ok(local);
    }
    // Fall back to FDID-based naming.
    let fdid_path = PathBuf::from(format!("data/terrain/{fdid}.adt"));
    if fdid_path.exists() {
        return Ok(fdid_path);
    }
    Err(format!(
        "ADT tile files not found: {} or {}",
        local.display(),
        fdid_path.display()
    ))
}

/// Parse map name and tile coordinates from an ADT filename.
///
/// Supports both `mapname_Y_X.adt` and FDID-based `778027.adt` (via listfile reverse lookup).
pub(crate) fn parse_tile_coords_from_path(
    adt_path: &Path,
) -> Result<(String, u32, u32), String> {
    let stem = adt_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| format!("Invalid ADT path: {}", adt_path.display()))?;

    // Try name-based: "azeroth_32_48"
    if let Some(result) = try_parse_named_stem(stem) {
        return Ok(result);
    }
    // Try FDID-based: "778027" → reverse lookup via listfile
    if let Ok(fdid) = stem.parse::<u32>() {
        return parse_coords_from_fdid(fdid);
    }
    Err(format!("Cannot parse tile coords from: {stem}"))
}

/// Try to parse "mapname_Y_X" from an ADT stem.
pub(crate) fn try_parse_named_stem(stem: &str) -> Option<(String, u32, u32)> {
    let parts: Vec<&str> = stem.rsplitn(3, '_').collect();
    if parts.len() < 3 {
        return None;
    }
    let tile_x = parts[0].parse::<u32>().ok()?;
    let tile_y = parts[1].parse::<u32>().ok()?;
    let map_name = parts[2].to_string();
    Some((map_name, tile_y, tile_x))
}

/// Reverse-lookup an FDID to extract map name and tile coordinates.
fn parse_coords_from_fdid(fdid: u32) -> Result<(String, u32, u32), String> {
    let wow_path = game_engine::listfile::lookup_fdid(fdid)
        .ok_or_else(|| format!("FDID {fdid} not in listfile"))?;
    // Path like "world/maps/azeroth/azeroth_32_48.adt"
    let filename = wow_path.rsplit('/').next().unwrap_or(wow_path);
    let stem = filename.strip_suffix(".adt").unwrap_or(filename);
    try_parse_named_stem(stem)
        .ok_or_else(|| format!("Cannot parse tile coords from listfile path: {wow_path}"))
}

/// Resolve companion file path (e.g. "_tex0", "_obj0") for an ADT.
///
/// For name-based files (e.g. `azeroth_32_48.adt`), appends suffix directly.
/// For FDID-based files (e.g. `778027.adt`), looks up the companion FDID via listfile.
pub(crate) fn resolve_companion_path(adt_path: &Path, suffix: &str) -> Option<PathBuf> {
    let stem = adt_path.file_stem()?.to_str()?;
    // Name-based: "azeroth_32_48" → "azeroth_32_48_tex0.adt"
    let direct = adt_path.with_file_name(format!("{stem}{suffix}.adt"));
    if direct.exists() {
        return Some(direct);
    }
    // FDID-based: reverse lookup to get WoW path, then find companion FDID
    let fdid: u32 = stem.parse().ok()?;
    let wow_path = game_engine::listfile::lookup_fdid(fdid)?;
    let wow_stem = wow_path.strip_suffix(".adt")?;
    let companion_wow = format!("{wow_stem}{suffix}.adt");
    let companion_fdid = game_engine::listfile::lookup_path(&companion_wow)?;
    let companion_path = adt_path.with_file_name(format!("{companion_fdid}.adt"));
    if companion_path.exists() { Some(companion_path) } else { None }
}

/// Try to load the companion _tex0.adt file.
pub(crate) fn load_tex0(adt_path: &Path) -> Option<adt::AdtTexData> {
    let tex0_path = resolve_companion_path(adt_path, "_tex0")?;
    let data = std::fs::read(&tex0_path).ok()?;
    match adt::load_adt_tex0(&data) {
        Ok(td) => {
            eprintln!(
                "Loaded _tex0: {} textures, {} chunks",
                td.texture_fdids.len(),
                td.chunk_layers.len()
            );
            Some(td)
        }
        Err(e) => {
            eprintln!("Failed to parse _tex0: {e}");
            None
        }
    }
}
