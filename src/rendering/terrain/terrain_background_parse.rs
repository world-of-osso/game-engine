use crate::asset::{m2, wmo};
use crate::terrain_objects;

use super::*;

pub(super) fn parse_tile_background(
    map_name: String,
    tile_y: u32,
    tile_x: u32,
    adt_path: PathBuf,
    lod: DoodadLod,
) -> TileLoadResult {
    let start_mem = crate::terrain_memory_debug::current_process_memory_kb();
    log_tile_background_parse_start(tile_y, tile_x, &adt_path, lod, &start_mem);
    let parsed = match build_parsed_tile(map_name.clone(), tile_y, tile_x, adt_path, lod) {
        Ok(parsed) => parsed,
        Err(error) => {
            return TileLoadResult::Failed {
                map_name,
                tile_y,
                tile_x,
                error,
            };
        }
    };
    log_tile_background_parse_success(&parsed, &start_mem);
    TileLoadResult::Success(Box::new(parsed))
}

fn build_parsed_tile(
    map_name: String,
    tile_y: u32,
    tile_x: u32,
    adt_path: PathBuf,
    lod: DoodadLod,
) -> Result<ParsedTile, String> {
    let adt_data = load_parsed_adt_data(tile_y, tile_x, &adt_path)?;
    let tex_data = load_parsed_tile_textures(tile_y, tile_x, &adt_path, &adt_data);
    let obj_data = load_parsed_tile_objects(tile_y, tile_x, &adt_path, lod);
    let (ground_images, height_images) = decode_tile_textures(&tex_data, &adt_path);
    let chunk_alpha_maps = pack_tile_alpha_maps(&tex_data);
    let chunk_shadow_maps = pack_tile_shadow_maps(&adt_data);
    let preloaded_doodads = preload_doodad_models(obj_data.as_ref());
    let preloaded_wmos = preload_wmo_data(obj_data.as_ref());

    Ok(ParsedTile {
        map_name,
        tile_y,
        tile_x,
        adt_path,
        adt_data,
        tex_data,
        obj_data,
        lod,
        ground_images,
        height_images,
        chunk_alpha_maps,
        chunk_shadow_maps,
        preloaded_doodads,
        preloaded_wmos,
    })
}

fn preload_doodad_models(obj_data: Option<&adt_obj::AdtObjData>) -> Vec<Option<PreloadedDoodad>> {
    let Some(obj) = obj_data else {
        return Vec::new();
    };
    obj.doodads
        .iter()
        .map(|doodad| {
            let m2_path = terrain_objects::resolve_doodad_m2(doodad)?;
            if !m2_path.exists() {
                return None;
            }
            match m2::load_m2_uncached(&m2_path, &[0, 0, 0]) {
                Ok(model) => Some(PreloadedDoodad {
                    path: m2_path,
                    model,
                }),
                Err(e) => {
                    eprintln!(
                        "preload_doodad_models: failed to load {}: {e}",
                        m2_path.display()
                    );
                    None
                }
            }
        })
        .collect()
}

fn preload_wmo_data(obj_data: Option<&adt_obj::AdtObjData>) -> Vec<Option<PreloadedWmo>> {
    let Some(obj) = obj_data else {
        return Vec::new();
    };
    obj.wmos
        .iter()
        .map(|placement| {
            let root_fdid = terrain_objects::resolve_wmo_fdid(placement)?;
            let root_path = terrain_objects::ensure_wmo_asset(root_fdid)?;
            let root_data = std::fs::read(&root_path).ok()?;
            let root = match wmo::load_wmo_root(&root_data) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("preload_wmo_data: failed to parse WMO {root_fdid}: {e}");
                    return None;
                }
            };
            let group_fdids = terrain_objects::resolve_wmo_group_fdids(
                root_fdid,
                root.n_groups,
                &root.group_file_data_ids,
            );
            let groups = preload_wmo_groups(&root, &group_fdids);
            Some(PreloadedWmo {
                root_fdid,
                root,
                groups,
                group_fdids,
            })
        })
        .collect()
}

fn preload_wmo_groups(
    root: &wmo::WmoRootData,
    group_fdids: &[Option<u32>],
) -> Vec<(u32, crate::asset::wmo::WmoGroupData)> {
    let mut groups = Vec::new();
    for fdid_opt in group_fdids {
        let Some(fdid) = fdid_opt else { continue };
        let Some(group_path) = terrain_objects::ensure_wmo_asset(*fdid) else {
            continue;
        };
        let Ok(data) = std::fs::read(&group_path) else {
            continue;
        };
        match wmo::load_wmo_group_with_root(&data, Some(root)) {
            Ok(group) => groups.push((*fdid, group)),
            Err(e) => {
                eprintln!("preload_wmo_groups: failed to parse WMO group {fdid}: {e}");
            }
        }
    }
    groups
}

fn decode_tile_textures(
    tex_data: &Option<adt::AdtTexData>,
    adt_path: &Path,
) -> (Vec<Option<Image>>, Vec<Option<Image>>) {
    match tex_data {
        Some(td) => (
            crate::terrain_material::decode_ground_images(td, adt_path),
            crate::terrain_material::decode_height_images(td, adt_path),
        ),
        None => (Vec::new(), Vec::new()),
    }
}

fn pack_tile_alpha_maps(tex_data: &Option<adt::AdtTexData>) -> Vec<Image> {
    tex_data
        .as_ref()
        .map(|td| {
            td.chunk_layers
                .iter()
                .map(|cl| crate::terrain_material::pack_alpha_map_raw(&cl.layers))
                .collect()
        })
        .unwrap_or_default()
}

fn pack_tile_shadow_maps(adt_data: &adt::AdtData) -> Vec<Image> {
    adt_data
        .chunks
        .iter()
        .map(|chunk| crate::terrain_material::pack_shadow_map_raw(chunk.shadow_map.as_ref()))
        .collect()
}

fn load_parsed_adt_data(tile_y: u32, tile_x: u32, adt_path: &Path) -> Result<adt::AdtData, String> {
    let adt_data = load_and_parse_adt(adt_path)?;
    eprintln!(
        "parse_tile_background adt ok ({}, {}) {} chunks={} height_grids={}",
        tile_y,
        tile_x,
        adt_path.display(),
        adt_data.chunks.len(),
        adt_data.height_grids.len(),
    );
    Ok(adt_data)
}

fn load_parsed_tile_textures(
    tile_y: u32,
    tile_x: u32,
    adt_path: &Path,
    adt_data: &adt::AdtData,
) -> Option<adt::AdtTexData> {
    let tex_data = load_tex0(adt_path, Some(adt_data));
    eprintln!(
        "parse_tile_background tex ok ({}, {}) {} tex={}",
        tile_y,
        tile_x,
        adt_path.display(),
        tex_data.as_ref().map_or(0, |td| td.texture_fdids.len()),
    );
    tex_data
}

fn load_parsed_tile_objects(
    tile_y: u32,
    tile_x: u32,
    adt_path: &Path,
    lod: DoodadLod,
) -> Option<adt_obj::AdtObjData> {
    let obj_data = load_obj_for_lod(adt_path, lod);
    eprintln!(
        "parse_tile_background obj ok ({}, {}) {} doodads={} wmos={}",
        tile_y,
        tile_x,
        adt_path.display(),
        obj_data.as_ref().map_or(0, |obj| obj.doodads.len()),
        obj_data.as_ref().map_or(0, |obj| obj.wmos.len()),
    );
    obj_data
}

fn log_tile_background_parse_start(
    tile_y: u32,
    tile_x: u32,
    adt_path: &Path,
    lod: DoodadLod,
    start_mem: &crate::terrain_memory_debug::ProcessMemoryKb,
) {
    eprintln!(
        "parse_tile_background start ({}, {}) {} lod={:?} rss={}MiB anon={}MiB",
        tile_y,
        tile_x,
        adt_path.display(),
        lod,
        start_mem.rss_kb / 1024,
        start_mem.anon_kb / 1024,
    );
}

fn log_tile_background_parse_success(
    parsed: &ParsedTile,
    start_mem: &crate::terrain_memory_debug::ProcessMemoryKb,
) {
    let end_mem = crate::terrain_memory_debug::current_process_memory_kb();
    eprintln!(
        "parse_tile_background success ({}, {}) {} rss={}MiB anon={}MiB delta_rss={}MiB",
        parsed.tile_y,
        parsed.tile_x,
        parsed.adt_path.display(),
        end_mem.rss_kb / 1024,
        end_mem.anon_kb / 1024,
        (end_mem.rss_kb as i64 - start_mem.rss_kb as i64) / 1024,
    );
}
