use super::*;

pub(super) fn parse_tile_background(
    tile_y: u32,
    tile_x: u32,
    adt_path: PathBuf,
    lod: DoodadLod,
) -> TileLoadResult {
    let start_mem = crate::terrain_memory_debug::current_process_memory_kb();
    log_tile_background_parse_start(tile_y, tile_x, &adt_path, lod, &start_mem);
    let parsed = match build_parsed_tile(tile_y, tile_x, adt_path, lod) {
        Ok(parsed) => parsed,
        Err(error) => {
            return TileLoadResult::Failed {
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

    Ok(ParsedTile {
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
    })
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
