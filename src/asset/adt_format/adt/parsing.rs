use std::io::Cursor;
use std::mem::size_of;

use super::*;
use crate::asset::read_bytes::{read_f32, read_i8, read_u32};
use binrw::BinRead;

fn parse_mcvt(payload: &[u8]) -> Result<[f32; MCVT_COUNT], String> {
    if payload.len() < MCVT_COUNT * 4 {
        return Err(format!(
            "MCVT too small: {} bytes (need {})",
            payload.len(),
            MCVT_COUNT * 4
        ));
    }
    let mut heights = [0.0f32; MCVT_COUNT];
    for (i, h) in heights.iter_mut().enumerate() {
        *h = read_f32(payload, i * 4)?;
    }
    Ok(heights)
}

#[cfg(test)]
mod normal_tests {
    use super::*;

    #[test]
    fn mcnr_flat_ground_normal_faces_up() {
        let payload = [0_u8, 0, 127].repeat(MCVT_COUNT);
        let normals = parse_mcnr(&payload).unwrap();
        assert_eq!(normals, [[0.0, 1.0, 0.0]; MCVT_COUNT]);
    }

    #[test]
    fn mcnr_adventurers_rest_slope_matches_geometry() {
        // MCNR bytes and MCVT-derived face normal from terrain tile 2703_31_37.
        let payload = [(-16_i8) as u8, 3, 125].repeat(MCVT_COUNT);
        let normals = parse_mcnr(&payload).unwrap();
        let geometric_normal = [-0.11974868, 0.992213, -0.034257512];
        let alignment: f32 = normals[0]
            .iter()
            .zip(geometric_normal)
            .map(|(decoded, geometric)| decoded * geometric)
            .sum();
        assert!(alignment > 0.99, "normal/geometry alignment: {alignment}");
    }
}

fn parse_mcnr(payload: &[u8]) -> Result<[[f32; 3]; MCVT_COUNT], String> {
    if payload.len() < MCVT_COUNT * 3 {
        return Err(format!(
            "MCNR too small: {} bytes (need {})",
            payload.len(),
            MCVT_COUNT * 3
        ));
    }
    let mut normals = [[0.0f32; 3]; MCVT_COUNT];
    for (i, n) in normals.iter_mut().enumerate() {
        let nx = read_i8(payload, i * 3)? as f32 / 127.0;
        let nz = read_i8(payload, i * 3 + 1)? as f32 / 127.0;
        let ny = read_i8(payload, i * 3 + 2)? as f32 / 127.0;
        let mut normal = [nx, ny, -nz];
        let len = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        if len > 0.0001 {
            normal[0] /= len;
            normal[1] /= len;
            normal[2] /= len;
        } else {
            normal = [0.0, 1.0, 0.0];
        }
        *n = normal;
    }
    Ok(normals)
}

pub(super) fn parse_mccv(payload: &[u8]) -> Result<[[f32; 4]; MCVT_COUNT], String> {
    let expected_len = MCVT_COUNT * MCCV_BYTES_PER_VERTEX;
    if payload.len() < expected_len {
        return Err(format!(
            "MCCV too small: {} bytes (need {})",
            payload.len(),
            expected_len
        ));
    }
    let mut colors = [[1.0f32; 4]; MCVT_COUNT];
    for (i, color) in colors.iter_mut().enumerate() {
        let base = i * MCCV_BYTES_PER_VERTEX;
        let blue = payload[base];
        let green = payload[base + 1];
        let red = payload[base + 2];
        let alpha = payload[base + 3];
        *color = [
            f32::from(red) / 127.0,
            f32::from(green) / 127.0,
            f32::from(blue) / 127.0,
            f32::from(alpha) / 255.0,
        ];
    }
    Ok(colors)
}

pub(super) fn parse_mclv(payload: &[u8]) -> Result<[[f32; 4]; MCVT_COUNT], String> {
    let expected_len = MCVT_COUNT * MCLV_BYTES_PER_VERTEX;
    if payload.len() < expected_len {
        return Err(format!(
            "MCLV too small: {} bytes (need {})",
            payload.len(),
            expected_len
        ));
    }

    let mut colors = [[1.0f32; 4]; MCVT_COUNT];
    for (i, color) in colors.iter_mut().enumerate() {
        let base = i * MCLV_BYTES_PER_VERTEX;
        let blue = payload[base];
        let green = payload[base + 1];
        let red = payload[base + 2];
        *color = [
            f32::from(red) / 128.0,
            f32::from(green) / 128.0,
            f32::from(blue) / 128.0,
            1.0,
        ];
    }
    Ok(colors)
}

fn parse_mcsh(payload: &[u8]) -> Result<[u8; MCSH_BYTES], String> {
    if payload.len() < MCSH_BYTES {
        return Err(format!(
            "MCSH too small: {} bytes (need {})",
            payload.len(),
            MCSH_BYTES
        ));
    }

    let mut shadow_map = [0; MCSH_BYTES];
    shadow_map.copy_from_slice(&payload[..MCSH_BYTES]);
    Ok(shadow_map)
}

fn fix_shadow_map_edges(shadow_map: &mut [u8; MCSH_BYTES]) {
    const SHADOW_MAP_SIDE: usize = 64;

    for row in 0..SHADOW_MAP_SIDE {
        let last_source_col = shadow_map_bit(shadow_map, row, 62);
        set_shadow_map_bit(shadow_map, row, 63, last_source_col);
    }
    for col in 0..SHADOW_MAP_SIDE {
        let last_source_row = shadow_map_bit(shadow_map, 62, col);
        set_shadow_map_bit(shadow_map, 63, col, last_source_row);
    }
}

fn shadow_map_bit(shadow_map: &[u8; MCSH_BYTES], row: usize, col: usize) -> bool {
    let byte_index = row * 8 + col / 8;
    let mask = 1 << (7 - (col % 8));
    (shadow_map[byte_index] & mask) != 0
}

fn set_shadow_map_bit(shadow_map: &mut [u8; MCSH_BYTES], row: usize, col: usize, value: bool) {
    let byte_index = row * 8 + col / 8;
    let mask = 1 << (7 - (col % 8));
    if value {
        shadow_map[byte_index] |= mask;
    } else {
        shadow_map[byte_index] &= !mask;
    }
}

fn parse_mcse(payload: &[u8]) -> Result<Vec<SoundEmitter>, String> {
    if !payload.len().is_multiple_of(MCSE_BYTES_PER_EMITTER) {
        return Err(format!(
            "MCSE size must be a multiple of {MCSE_BYTES_PER_EMITTER} bytes: {} bytes",
            payload.len()
        ));
    }

    let mut emitters = Vec::with_capacity(payload.len() / MCSE_BYTES_PER_EMITTER);
    let mut cursor = Cursor::new(payload);
    while (cursor.position() as usize) < payload.len() {
        emitters.push(
            SoundEmitter::read_le(&mut cursor)
                .map_err(|err| format!("MCSE emitter parse failed: {err}"))?,
        );
    }
    Ok(emitters)
}

fn parse_mcbb(payload: &[u8]) -> Result<Vec<BlendBatch>, String> {
    if !payload.len().is_multiple_of(MCBB_BYTES_PER_BATCH) {
        return Err(format!(
            "MCBB size must be a multiple of {MCBB_BYTES_PER_BATCH} bytes: {} bytes",
            payload.len()
        ));
    }

    let mut batches = Vec::with_capacity(payload.len() / MCBB_BYTES_PER_BATCH);
    let mut cursor = Cursor::new(payload);
    while (cursor.position() as usize) < payload.len() {
        batches.push(
            BlendBatch::read_le(&mut cursor)
                .map_err(|err| format!("MCBB batch parse failed: {err}"))?,
        );
    }
    Ok(batches)
}

fn parse_mcdd(payload: &[u8]) -> Result<[u8; MCDD_BYTES], String> {
    if payload.len() < MCDD_BYTES {
        return Err(format!(
            "MCDD too small: {} bytes (need {})",
            payload.len(),
            MCDD_BYTES
        ));
    }

    let mut disable = [0; MCDD_BYTES];
    disable.copy_from_slice(&payload[..MCDD_BYTES]);
    Ok(disable)
}

fn parse_blend_mesh_headers(payload: &[u8]) -> Result<Vec<BlendMeshHeader>, String> {
    parse_binrw_array(payload, MBMH_BYTES_PER_HEADER, "MBMH header")
}

fn parse_blend_mesh_bounds(payload: &[u8]) -> Result<Vec<BlendMeshBounds>, String> {
    parse_binrw_array(payload, MBBB_BYTES_PER_BOUND, "MBBB bounds")
}

fn parse_blend_mesh_vertices(payload: &[u8]) -> Result<Vec<BlendMeshVertex>, String> {
    parse_binrw_array(payload, MBNV_BYTES_PER_VERTEX, "MBNV vertex")
}

fn parse_blend_mesh_indices(payload: &[u8]) -> Result<Vec<u16>, String> {
    if !payload.len().is_multiple_of(size_of::<u16>()) {
        return Err(format!(
            "MBMI size must be a multiple of {} bytes: {} bytes",
            size_of::<u16>(),
            payload.len()
        ));
    }

    Ok(payload
        .chunks_exact(size_of::<u16>())
        .map(|chunk| u16::from_le_bytes(chunk.try_into().unwrap()))
        .collect())
}

fn parse_binrw_array<T>(payload: &[u8], entry_size: usize, label: &str) -> Result<Vec<T>, String>
where
    for<'a> T: BinRead<Args<'a> = ()>,
{
    if !payload.len().is_multiple_of(entry_size) {
        return Err(format!(
            "{label} array size must be a multiple of {entry_size} bytes: {} bytes",
            payload.len()
        ));
    }

    let mut entries = Vec::with_capacity(payload.len() / entry_size);
    let mut cursor = Cursor::new(payload);
    while (cursor.position() as usize) < payload.len() {
        entries
            .push(T::read_le(&mut cursor).map_err(|err| format!("{label} parse failed: {err}"))?);
    }
    Ok(entries)
}

pub(super) fn parse_blend_mesh_data(
    root_chunks: &AdtRootChunks<'_>,
) -> Result<Option<BlendMeshData>, String> {
    let has_any_blend_mesh_chunk = root_chunks.mbmh.is_some()
        || root_chunks.mbbb.is_some()
        || root_chunks.mbnv.is_some()
        || root_chunks.mbmi.is_some();
    if !has_any_blend_mesh_chunk {
        return Ok(None);
    }

    let headers = root_chunks
        .mbmh
        .ok_or("ADT has blend mesh chunks but is missing HMBM (MBMH)".to_string())
        .and_then(parse_blend_mesh_headers)?;
    let vertices = root_chunks
        .mbnv
        .ok_or("ADT has blend mesh chunks but is missing VNBM (MBNV)".to_string())
        .and_then(parse_blend_mesh_vertices)?;
    let indices = root_chunks
        .mbmi
        .ok_or("ADT has blend mesh chunks but is missing IMBM (MBMI)".to_string())
        .and_then(parse_blend_mesh_indices)?;
    let bounds = match root_chunks.mbbb {
        Some(payload) => parse_blend_mesh_bounds(payload)?,
        None => Vec::new(),
    };

    Ok(Some(BlendMeshData {
        headers,
        bounds,
        vertices,
        indices,
    }))
}

pub(super) fn parse_mfbo(payload: &[u8]) -> Result<FlightBounds, String> {
    const MFBO_ENTRIES: usize = 18;
    const MFBO_BYTES: usize = MFBO_ENTRIES * size_of::<i16>();

    if payload.len() < MFBO_BYTES {
        return Err(format!(
            "MFBO too small: {} bytes (need {})",
            payload.len(),
            MFBO_BYTES
        ));
    }

    let mut values = [0i16; MFBO_ENTRIES];
    for (index, value) in values.iter_mut().enumerate() {
        let base = index * size_of::<i16>();
        *value = i16::from_le_bytes(payload[base..base + size_of::<i16>()].try_into().unwrap());
    }

    let mut min_heights = [0i16; 9];
    let mut max_heights = [0i16; 9];
    min_heights.copy_from_slice(&values[..9]);
    max_heights.copy_from_slice(&values[9..]);
    Ok(FlightBounds {
        min_heights,
        max_heights,
    })
}

pub(super) fn parse_mver(payload: &[u8]) -> Result<u32, String> {
    read_u32(payload, 0).map_err(|err| format!("MVER parse failed: {err}"))
}

pub(super) fn parse_mlhd(payload: &[u8]) -> Result<LodHeader, String> {
    if payload.len() < 28 {
        return Err(format!("MLHD too small: {} bytes (need 28)", payload.len()));
    }

    Ok(LodHeader {
        flags: read_u32(payload, 0)?,
        bounds_min: [
            read_f32(payload, 4)?,
            read_f32(payload, 12)?,
            read_f32(payload, 20)?,
        ],
        bounds_max: [
            read_f32(payload, 8)?,
            read_f32(payload, 16)?,
            read_f32(payload, 24)?,
        ],
    })
}

pub(super) fn parse_mlvh(payload: &[u8]) -> Result<Vec<f32>, String> {
    if !payload.len().is_multiple_of(size_of::<f32>()) {
        return Err(format!(
            "MLVH size {} is not a multiple of {}",
            payload.len(),
            size_of::<f32>()
        ));
    }

    (0..payload.len())
        .step_by(size_of::<f32>())
        .map(|offset| read_f32(payload, offset))
        .collect()
}

pub(super) fn parse_mlll(payload: &[u8]) -> Result<Vec<LodLevel>, String> {
    const MLLL_RECORD_BYTES: usize = 20;
    if !payload.len().is_multiple_of(MLLL_RECORD_BYTES) {
        return Err(format!(
            "MLLL size {} is not a multiple of {}",
            payload.len(),
            MLLL_RECORD_BYTES
        ));
    }

    (0..payload.len())
        .step_by(MLLL_RECORD_BYTES)
        .map(|offset| {
            Ok(LodLevel {
                vertex_step: read_f32(payload, offset)?,
                payload: [
                    read_u32(payload, offset + 4)?,
                    read_u32(payload, offset + 8)?,
                    read_u32(payload, offset + 12)?,
                    read_u32(payload, offset + 16)?,
                ],
            })
        })
        .collect()
}

pub(super) fn parse_mlnd(payload: &[u8]) -> Result<Vec<LodQuadTreeNode>, String> {
    const MLND_RECORD_BYTES: usize = 20;
    if !payload.len().is_multiple_of(MLND_RECORD_BYTES) {
        return Err(format!(
            "MLND size {} is not a multiple of {}",
            payload.len(),
            MLND_RECORD_BYTES
        ));
    }

    (0..payload.len())
        .step_by(MLND_RECORD_BYTES)
        .map(|offset| {
            Ok(LodQuadTreeNode {
                words16: [
                    u16::from_le_bytes(payload[offset..offset + 2].try_into().unwrap()),
                    u16::from_le_bytes(payload[offset + 2..offset + 4].try_into().unwrap()),
                    u16::from_le_bytes(payload[offset + 4..offset + 6].try_into().unwrap()),
                    u16::from_le_bytes(payload[offset + 6..offset + 8].try_into().unwrap()),
                ],
                words32: [
                    read_u32(payload, offset + 8)?,
                    read_u32(payload, offset + 12)?,
                    read_u32(payload, offset + 16)?,
                ],
            })
        })
        .collect()
}

pub(super) fn parse_u16_block(payload: &[u8], label: &str) -> Result<Vec<u16>, String> {
    if !payload.len().is_multiple_of(size_of::<u16>()) {
        return Err(format!(
            "{label} size {} is not a multiple of {}",
            payload.len(),
            size_of::<u16>()
        ));
    }

    (0..payload.len())
        .step_by(size_of::<u16>())
        .map(|offset| {
            Ok(u16::from_le_bytes(
                payload[offset..offset + size_of::<u16>()]
                    .try_into()
                    .unwrap(),
            ))
        })
        .collect()
}

fn parse_mlln(payload: &[u8]) -> Result<LodLiquidPatchHeader, String> {
    const MLLN_WORDS: usize = 6;
    const MLLN_BYTES: usize = MLLN_WORDS * size_of::<u32>();
    if payload.len() < MLLN_BYTES {
        return Err(format!(
            "MLLN too small: {} bytes (need {})",
            payload.len(),
            MLLN_BYTES
        ));
    }

    let mut words = [0u32; MLLN_WORDS];
    for (index, value) in words.iter_mut().enumerate() {
        *value = read_u32(payload, index * size_of::<u32>())?;
    }
    Ok(LodLiquidPatchHeader { words })
}

fn parse_mllv(payload: &[u8]) -> Result<Vec<[f32; 3]>, String> {
    const MLLV_VERTEX_BYTES: usize = 3 * size_of::<f32>();
    if !payload.len().is_multiple_of(MLLV_VERTEX_BYTES) {
        return Err(format!(
            "MLLV size {} is not a multiple of {}",
            payload.len(),
            MLLV_VERTEX_BYTES
        ));
    }

    (0..payload.len())
        .step_by(MLLV_VERTEX_BYTES)
        .map(|offset| {
            Ok([
                read_f32(payload, offset)?,
                read_f32(payload, offset + size_of::<f32>())?,
                read_f32(payload, offset + size_of::<f32>() * 2)?,
            ])
        })
        .collect()
}

pub(super) fn parse_lod_liquids(
    groups: Vec<LodLiquidChunkGroup<'_>>,
) -> Result<Vec<LodLiquidPatch>, String> {
    groups
        .into_iter()
        .map(|group| {
            Ok(LodLiquidPatch {
                header: parse_mlln(group.header)?,
                indices: parse_u16_block(group.indices, "MLLI")?,
                vertices: parse_mllv(group.vertices)?,
            })
        })
        .collect()
}

pub(super) fn parse_lod_object_placements(
    payload: Option<&[u8]>,
    label: &str,
) -> Result<Vec<LodObjectPlacement>, String> {
    const LOD_OBJECT_PLACEMENT_BYTES: usize = 40;
    let Some(payload) = payload else {
        return Ok(Vec::new());
    };
    if !payload.len().is_multiple_of(LOD_OBJECT_PLACEMENT_BYTES) {
        return Err(format!(
            "{label} size {} is not a multiple of {}",
            payload.len(),
            LOD_OBJECT_PLACEMENT_BYTES
        ));
    }

    (0..payload.len())
        .step_by(LOD_OBJECT_PLACEMENT_BYTES)
        .map(|offset| {
            Ok(LodObjectPlacement {
                id: read_u32(payload, offset)?,
                asset_id: read_u32(payload, offset + 4)?,
                position: [
                    read_f32(payload, offset + 8)?,
                    read_f32(payload, offset + 12)?,
                    read_f32(payload, offset + 16)?,
                ],
                rotation: [
                    read_f32(payload, offset + 20)?,
                    read_f32(payload, offset + 24)?,
                    read_f32(payload, offset + 28)?,
                ],
                scale: read_f32(payload, offset + 32)?,
                flags: read_u32(payload, offset + 36)?,
            })
        })
        .collect()
}

pub(super) fn parse_lod_object_visibility(
    payload: Option<&[u8]>,
    label: &str,
) -> Result<Vec<LodObjectVisibility>, String> {
    const LOD_OBJECT_VISIBILITY_BYTES: usize = 28;
    let Some(payload) = payload else {
        return Ok(Vec::new());
    };
    if !payload.len().is_multiple_of(LOD_OBJECT_VISIBILITY_BYTES) {
        return Err(format!(
            "{label} size {} is not a multiple of {}",
            payload.len(),
            LOD_OBJECT_VISIBILITY_BYTES
        ));
    }

    (0..payload.len())
        .step_by(LOD_OBJECT_VISIBILITY_BYTES)
        .map(|offset| {
            Ok(LodObjectVisibility {
                bounds_min: [
                    read_f32(payload, offset)?,
                    read_f32(payload, offset + 4)?,
                    read_f32(payload, offset + 8)?,
                ],
                bounds_max: [
                    read_f32(payload, offset + 12)?,
                    read_f32(payload, offset + 16)?,
                    read_f32(payload, offset + 20)?,
                ],
                radius: read_f32(payload, offset + 24)?,
            })
        })
        .collect()
}

pub(super) fn parse_mcnk(
    payload: &[u8],
    texture_shadow: Option<[u8; MCSH_BYTES]>,
) -> Result<McnkData, String> {
    if payload.len() < size_of::<McnkHeader>() {
        return Err(format!("MCNK payload too small: {} bytes", payload.len()));
    }
    let header: McnkHeader = parse_binrw_value(payload, 0, "MCNK header")?;
    let flags = McnkFlags::from_bits(header.flags);
    let subchunks = parse_mcnk_subchunks(&payload[128..], flags, texture_shadow)?;
    Ok(build_mcnk_data(header, flags, subchunks))
}

fn build_mcnk_data(
    header: McnkHeader,
    flags: McnkFlags,
    subchunks: McnkSubchunksResult,
) -> McnkData {
    let (
        heights,
        normals,
        vertex_colors,
        vertex_lighting,
        shadow_map,
        sound_emitters,
        blend_batches,
        detail_doodad_disable,
    ) = subchunks;
    McnkData {
        index_x: header.index_x,
        index_y: header.index_y,
        pos: [header.pos_x, header.pos_y, header.pos_z],
        flags,
        area_id: header._area_id,
        shadow_map,
        vertex_lighting,
        sound_emitters,
        blend_batches,
        detail_doodad_disable,
        holes_low_res: header._holes_low_res,
        holes_high_res: flags.high_res_holes.then_some(header._holes_high_res),
        heights,
        normals,
        vertex_colors,
    }
}

pub(super) fn parse_mcnk_subchunks(
    sub: &[u8],
    flags: McnkFlags,
    texture_shadow: Option<[u8; MCSH_BYTES]>,
) -> Result<McnkSubchunksResult, String> {
    let mut accum = empty_mcnk_subchunk_accum();
    apply_mcnk_subchunk_stream(&mut accum, sub)?;
    if let Some(shadow) = texture_shadow {
        if accum
            .shadow_map
            .is_some_and(|root_shadow| root_shadow != shadow)
        {
            return Err("conflicting root and texture-companion HSCM data".to_string());
        }
        accum.shadow_map = Some(shadow);
    }
    finalize_mcnk_subchunks(accum, flags)
}

pub(super) fn parse_texture_shadow_maps(
    data: &[u8],
) -> Result<Vec<Option<[u8; MCSH_BYTES]>>, String> {
    let mut shadows = Vec::new();
    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        if tag == b"KNCM" {
            shadows.push(parse_texture_chunk_shadow(payload)?);
        }
    }
    Ok(shadows)
}

fn parse_texture_chunk_shadow(data: &[u8]) -> Result<Option<[u8; MCSH_BYTES]>, String> {
    let mut shadow = None;
    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        if tag == b"HSCM" {
            if shadow.is_some() {
                return Err("duplicate HSCM in texture companion chunk".to_string());
            }
            shadow = Some(parse_mcsh(payload)?);
        }
    }
    Ok(shadow)
}

fn empty_mcnk_subchunk_accum() -> McnkSubchunkAccum {
    McnkSubchunkAccum {
        heights: None,
        normals: None,
        vertex_colors: None,
        vertex_lighting: None,
        shadow_map: None,
        sound_emitters: None,
        blend_batches: None,
        detail_doodad_disable: None,
    }
}

fn apply_mcnk_subchunk_stream(accum: &mut McnkSubchunkAccum, sub: &[u8]) -> Result<(), String> {
    for chunk in ChunkIter::new(sub) {
        let (tag, payload) = chunk?;
        apply_mcnk_subchunk(accum, tag, payload)?;
    }
    Ok(())
}

fn apply_mcnk_subchunk(
    accum: &mut McnkSubchunkAccum,
    tag: &[u8; 4],
    payload: &[u8],
) -> Result<(), String> {
    match tag {
        b"TVCM" => accum.heights = Some(parse_mcvt(payload)?),
        b"RNCM" => accum.normals = Some(parse_mcnr(payload)?),
        b"VCCM" => accum.vertex_colors = Some(parse_mccv(payload)?),
        b"VLCM" => accum.vertex_lighting = Some(parse_mclv(payload)?),
        b"HSCM" => accum.shadow_map = Some(parse_mcsh(payload)?),
        b"MCSE" => accum.sound_emitters = Some(parse_mcse(payload)?),
        b"BBCM" => accum.blend_batches = Some(parse_mcbb(payload)?),
        b"DDCM" => accum.detail_doodad_disable = Some(parse_mcdd(payload)?),
        _ => {}
    }
    Ok(())
}

fn finalize_mcnk_subchunks(
    accum: McnkSubchunkAccum,
    flags: McnkFlags,
) -> Result<McnkSubchunksResult, String> {
    let heights = accum.heights.ok_or("MCNK missing TVCM sub-chunk")?;
    let normals = accum.normals.unwrap_or([[0.0, 1.0, 0.0]; MCVT_COUNT]);
    let vertex_colors = resolve_mcnk_vertex_colors(accum.vertex_colors, flags)?;
    let shadow_map = resolve_mcnk_shadow_map(accum.shadow_map, flags)?;
    Ok((
        heights,
        normals,
        vertex_colors,
        accum.vertex_lighting,
        shadow_map,
        accum.sound_emitters.unwrap_or_default(),
        accum.blend_batches.unwrap_or_default(),
        accum.detail_doodad_disable,
    ))
}

fn resolve_mcnk_vertex_colors(
    vertex_colors: Option<[[f32; 4]; MCVT_COUNT]>,
    flags: McnkFlags,
) -> Result<[[f32; 4]; MCVT_COUNT], String> {
    match vertex_colors {
        Some(colors) => Ok(colors),
        None if !flags.has_mccv => Ok([[1.0, 1.0, 1.0, 1.0]; MCVT_COUNT]),
        None => Err("MCNK flagged with MCCV but missing VCCM sub-chunk".to_string()),
    }
}

fn resolve_mcnk_shadow_map(
    shadow_map: Option<[u8; MCSH_BYTES]>,
    flags: McnkFlags,
) -> Result<Option<[u8; MCSH_BYTES]>, String> {
    match shadow_map {
        Some(mut shadow_map) => {
            if !flags.do_not_fix_alpha_map {
                fix_shadow_map_edges(&mut shadow_map);
            }
            Ok(Some(shadow_map))
        }
        None if !flags.has_mcsh => Ok(None),
        None => Err("MCNK flagged with MCSH but missing HSCM sub-chunk".to_string()),
    }
}

pub(super) fn collect_adt_chunks(data: &[u8]) -> AdtChunksResult<'_> {
    let mut root_chunks = AdtRootChunks {
        mcnks: Vec::with_capacity(256),
        mh2o: None,
        mfbo: None,
        mbmh: None,
        mbbb: None,
        mbnv: None,
        mbmi: None,
    };
    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        match tag {
            b"KNCM" => root_chunks.mcnks.push(payload),
            b"O2HM" => root_chunks.mh2o = Some(payload),
            b"OFBM" => root_chunks.mfbo = Some(payload),
            b"HMBM" => root_chunks.mbmh = Some(payload),
            b"BBBM" => root_chunks.mbbb = Some(payload),
            b"VNBM" => root_chunks.mbnv = Some(payload),
            b"IMBM" => root_chunks.mbmi = Some(payload),
            _ => {}
        }
    }
    if root_chunks.mcnks.is_empty() {
        return Err("No KNCM (MCNK) chunks found in ADT file".to_string());
    }
    Ok(root_chunks)
}
