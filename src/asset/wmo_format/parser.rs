use std::io::Cursor;

use binrw::BinRead;

use crate::asset::adt::ChunkIter;

#[path = "parser_chunks.rs"]
mod parser_chunks;
#[path = "parser_types.rs"]
mod parser_types;
pub use parser_types::*;

pub fn wmo_local_to_bevy(x: f32, y: f32, z: f32) -> [f32; 3] {
    [-x, z, y]
}

fn parse_binrw_entries<T>(data: &[u8], entry_size: usize, label: &str) -> Result<Vec<T>, String>
where
    for<'a> T: BinRead<Args<'a> = ()>,
{
    let count = data.len() / entry_size;
    let byte_len = count
        .checked_mul(entry_size)
        .ok_or_else(|| format!("{label} byte length overflow"))?;
    let slice = data
        .get(..byte_len)
        .ok_or_else(|| format!("{label} data out of bounds"))?;
    let mut cursor = Cursor::new(slice);
    let mut entries = Vec::with_capacity(count);
    for i in 0..count {
        entries.push(
            T::read_le(&mut cursor).map_err(|err| {
                format!("{label} {i} parse failed at {:#x}: {err}", i * entry_size)
            })?,
        );
    }
    Ok(entries)
}

fn parse_binrw_value<T>(data: &[u8], byte_len: usize, label: &str) -> Result<T, String>
where
    for<'a> T: BinRead<Args<'a> = ()>,
{
    let slice = data
        .get(..byte_len)
        .ok_or_else(|| format!("{label} too small: {} bytes", data.len()))?;
    T::read_le(&mut Cursor::new(slice)).map_err(|err| format!("{label} parse failed: {err}"))
}

pub fn load_wmo_root(data: &[u8]) -> Result<WmoRootData, String> {
    parser_chunks::load_wmo_root(data)
}

#[cfg(test)]
use parser_chunks::{WmoRootAccum, apply_root_chunk};

fn parse_c_string(data: &[u8]) -> Option<String> {
    let nul = data.iter().position(|&b| b == 0).unwrap_or(data.len());
    let bytes = &data[..nul];
    if bytes.is_empty() {
        return None;
    }
    Some(String::from_utf8_lossy(bytes).into_owned())
}

fn parse_fixed_c_string(bytes: &[u8]) -> String {
    parse_c_string(bytes).unwrap_or_default()
}

pub fn parse_momt(data: &[u8]) -> Result<Vec<WmoMaterialDef>, String> {
    Ok(
        parse_binrw_entries::<RawWmoMaterialDef>(data, MOMT_ENTRY_SIZE, "MOMT")?
            .into_iter()
            .map(|mat| WmoMaterialDef {
                texture_fdid: mat.texture_fdid,
                texture_2_fdid: mat.texture_2_fdid,
                texture_3_fdid: mat.texture_3_fdid,
                flags: mat.flags,
                material_flags: WmoMaterialFlags::from_bits(mat.flags),
                sidn_color: parse_bgra_color(mat._sidn_emissive_color.to_le_bytes()),
                diff_color: parse_bgra_color(mat._diff_color.to_le_bytes()),
                ground_type: mat._terrain_type,
                blend_mode: mat.blend_mode,
                shader: mat.shader,
                uv_translation_speed: None,
            })
            .collect(),
    )
}

pub fn parse_mouv(data: &[u8]) -> Result<Vec<WmoMaterialUvTransform>, String> {
    Ok(
        parse_binrw_entries::<RawWmoMaterialUvTransform>(data, MOUV_ENTRY_SIZE, "MOUV")?
            .into_iter()
            .map(|transform| WmoMaterialUvTransform {
                translation_speed: transform.translation_speed,
            })
            .collect(),
    )
}

pub fn parse_molt(data: &[u8]) -> Result<Vec<WmoLight>, String> {
    Ok(
        parse_binrw_entries::<RawWmoLight>(data, MOLT_ENTRY_SIZE, "MOLT")?
            .into_iter()
            .map(|light| WmoLight {
                light_type: WmoLightType::from_raw(light.light_type),
                use_attenuation: light.use_attenuation != 0,
                color: parse_bgra_color(light.color),
                position: light.position,
                intensity: light.intensity,
                rotation: light.rotation,
                attenuation_start: light.attenuation_start,
                attenuation_end: light.attenuation_end,
            })
            .collect(),
    )
}

pub fn parse_mods(data: &[u8]) -> Result<Vec<WmoDoodadSet>, String> {
    Ok(
        parse_binrw_entries::<RawWmoDoodadSet>(data, MODS_ENTRY_SIZE, "MODS")?
            .into_iter()
            .map(|set| WmoDoodadSet {
                name: parse_fixed_c_string(&set.name),
                start_doodad: set.start_doodad,
                n_doodads: set.n_doodads,
            })
            .collect(),
    )
}

pub fn parse_modn(data: &[u8]) -> Result<Vec<WmoDoodadName>, String> {
    let mut names = Vec::new();
    let mut offset = 0usize;

    while offset < data.len() {
        let remaining = &data[offset..];
        let Some(name) = parse_c_string(remaining) else {
            break;
        };
        let byte_len = name.len() + 1;
        names.push(WmoDoodadName {
            offset: offset as u32,
            name,
        });
        offset += byte_len;
    }

    Ok(names)
}

pub fn parse_mogn(data: &[u8]) -> Result<Vec<WmoGroupName>, String> {
    let mut names = Vec::new();
    let mut offset = 0usize;

    while offset < data.len() {
        let remaining = &data[offset..];
        let Some(name) = parse_c_string(remaining) else {
            break;
        };
        let byte_len = name.len() + 1;
        let is_antiportal = name.to_ascii_lowercase().contains("antiportal");
        names.push(WmoGroupName {
            offset: offset as u32,
            name,
            is_antiportal,
        });
        offset += byte_len;
    }

    Ok(names)
}

pub fn parse_modi(data: &[u8]) -> Result<Vec<u32>, String> {
    Ok(data
        .chunks_exact(MODI_ENTRY_SIZE)
        .map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap()))
        .collect())
}

pub fn parse_gfid(data: &[u8]) -> Result<Vec<u32>, String> {
    Ok(data
        .chunks_exact(MODI_ENTRY_SIZE)
        .map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap()))
        .collect())
}

pub fn parse_mavd(data: &[u8]) -> Result<Vec<WmoAmbientVolume>, String> {
    Ok(
        parse_binrw_entries::<RawWmoAmbientVolume>(data, MAVD_ENTRY_SIZE, "MAVD")?
            .into_iter()
            .map(|volume| WmoAmbientVolume {
                position: volume.position,
                start: volume.start,
                end: volume.end,
                color_1: parse_bgra_color(volume.color_1),
                color_2: parse_bgra_color(volume.color_2),
                color_3: parse_bgra_color(volume.color_3),
                flags: volume.flags,
                doodad_set_id: volume.doodad_set_id,
            })
            .collect(),
    )
}

pub fn parse_mbvd(data: &[u8]) -> Result<Vec<WmoAmbientBoxVolume>, String> {
    Ok(
        parse_binrw_entries::<RawWmoAmbientBoxVolume>(data, MBVD_ENTRY_SIZE, "MBVD")?
            .into_iter()
            .map(|volume| WmoAmbientBoxVolume {
                planes: volume.planes,
                end: volume.end,
                color_1: parse_bgra_color(volume.color_1),
                color_2: parse_bgra_color(volume.color_2),
                color_3: parse_bgra_color(volume.color_3),
                flags: volume.flags,
                doodad_set_id: volume.doodad_set_id,
            })
            .collect(),
    )
}

pub fn parse_mnld(data: &[u8]) -> Result<Vec<WmoNewLight>, String> {
    Ok(
        parse_binrw_entries::<RawWmoNewLight>(data, MNLD_ENTRY_SIZE, "MNLD")?
            .into_iter()
            .map(|light| WmoNewLight {
                light_type: WmoNewLightType::from_raw(light.light_type),
                light_index: light.light_index,
                flags: light.flags,
                doodad_set: light.doodad_set,
                inner_color: parse_bgra_color(light.inner_color),
                position: light.position,
                rotation: light.rotation,
                attenuation_start: light.attenuation_start,
                attenuation_end: light.attenuation_end,
                intensity: light.intensity,
                outer_color: parse_bgra_color(light.outer_color),
            })
            .collect(),
    )
}

pub fn parse_modd(data: &[u8]) -> Result<Vec<WmoDoodadDef>, String> {
    Ok(
        parse_binrw_entries::<RawWmoDoodadDef>(data, MODD_ENTRY_SIZE, "MODD")?
            .into_iter()
            .map(|doodad| WmoDoodadDef {
                name_offset: doodad.name_index_and_flags & 0x00FF_FFFF,
                flags: (doodad.name_index_and_flags >> 24) as u8,
                position: doodad.position,
                rotation: doodad.rotation,
                scale: doodad.scale,
                color: parse_bgra_color(doodad.color),
            })
            .collect(),
    )
}

pub fn parse_mfog(data: &[u8]) -> Result<Vec<WmoFog>, String> {
    Ok(
        parse_binrw_entries::<RawWmoFog>(data, MFOG_ENTRY_SIZE, "MFOG")?
            .into_iter()
            .map(|fog| WmoFog {
                flags: fog.flags,
                position: fog.position,
                smaller_radius: fog.smaller_radius,
                larger_radius: fog.larger_radius,
                fog_end: fog.fog_end,
                fog_start_multiplier: fog.fog_start_multiplier,
                color_1: parse_bgra_color(fog.color_1),
                underwater_fog_end: fog.underwater_fog_end,
                underwater_fog_start_multiplier: fog.underwater_fog_start_multiplier,
                color_2: parse_bgra_color(fog.color_2),
            })
            .collect(),
    )
}

pub fn parse_movb(data: &[u8]) -> Result<Vec<WmoVisibleBlock>, String> {
    Ok(
        parse_binrw_entries::<RawWmoVisibleBlock>(data, MOVB_ENTRY_SIZE, "MOVB")?
            .into_iter()
            .map(|block| WmoVisibleBlock {
                start_vertex: block.start_vertex,
                vertex_count: block.vertex_count,
            })
            .collect(),
    )
}

pub fn parse_mcvp(data: &[u8]) -> Result<Vec<WmoConvexVolumePlane>, String> {
    Ok(
        parse_binrw_entries::<RawWmoConvexVolumePlane>(data, MCVP_ENTRY_SIZE, "MCVP")?
            .into_iter()
            .map(|plane| WmoConvexVolumePlane {
                normal: plane.normal,
                distance: plane.distance,
                flags: plane.flags,
            })
            .collect(),
    )
}

pub fn find_mogp(data: &[u8]) -> Result<&[u8], String> {
    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        if tag == b"PGOM" {
            return Ok(payload);
        }
    }
    Err("No MOGP chunk found in WMO group file".to_string())
}

pub fn parse_mogp_header(data: &[u8]) -> Result<WmoGroupHeader, String> {
    let header: RawWmoGroupHeader = parse_binrw_value(data, MOGP_HEADER_SIZE, "MOGP")?;
    Ok(WmoGroupHeader {
        group_name_offset: header.group_name_offset,
        descriptive_group_name_offset: header.descriptive_group_name_offset,
        flags: header.flags,
        group_flags: WmoGroupFlags::from_bits(header.flags),
        bbox_min: header.bbox_min,
        bbox_max: header.bbox_max,
        portal_start: header.portal_start,
        portal_count: header.portal_count,
        trans_batch_count: header.trans_batch_count,
        int_batch_count: header.int_batch_count,
        ext_batch_count: header.ext_batch_count,
        batch_type_d: header.batch_type_d,
        fog_ids: header.fog_ids,
        group_liquid: header.group_liquid,
        unique_id: header.unique_id,
        flags2: header.flags2,
        parent_split_group_index: header.parent_split_group_index,
        next_split_child_group_index: header.next_split_child_group_index,
    })
}

pub fn parse_group_subchunks(data: &[u8]) -> Result<RawGroupData, String> {
    parser_chunks::parse_group_subchunks(data)
}

pub fn parse_mopy(data: &[u8]) -> Result<Vec<WmoTriangleMaterial>, String> {
    Ok(
        parse_binrw_entries::<RawWmoTriangleMaterial>(data, MOPY_ENTRY_SIZE, "MOPY")?
            .into_iter()
            .map(|entry| WmoTriangleMaterial {
                flags: entry.flags,
                material_id: entry.material_id,
            })
            .collect(),
    )
}

pub fn parse_mobn(data: &[u8]) -> Result<Vec<WmoBspNode>, String> {
    Ok(
        parse_binrw_entries::<RawWmoBspNode>(data, MOBN_ENTRY_SIZE, "MOBN")?
            .into_iter()
            .map(|entry| WmoBspNode {
                flags: entry.flags,
                neg_child: entry.neg_child,
                pos_child: entry.pos_child,
                face_count: entry.face_count,
                face_start: entry.face_start,
                plane_dist: entry.plane_dist,
            })
            .collect(),
    )
}

pub fn parse_mobr(data: &[u8]) -> Result<Vec<u16>, String> {
    parse_binrw_entries(data, MOBR_ENTRY_SIZE, "MOBR")
}

pub fn parse_mliq(data: &[u8]) -> Result<WmoLiquid, String> {
    let header: RawWmoLiquidHeader = parse_binrw_value(data, MLIQ_HEADER_SIZE, "MLIQ")?;
    let vertex_count = checked_mliq_count(header.x_verts, header.y_verts, "vertex")?;
    let tile_count = checked_mliq_count(header.x_tiles, header.y_tiles, "tile")?;
    let (vertices_data, tiles_data) = split_mliq_payloads(data, vertex_count, tile_count)?;
    build_mliq(header, vertices_data, tiles_data)
}

fn checked_mliq_count(width: i32, height: i32, label: &str) -> Result<usize, String> {
    width
        .checked_mul(height)
        .ok_or_else(|| format!("MLIQ {label} count overflow"))
        .map(|count| count as usize)
}

fn split_mliq_payloads(
    data: &[u8],
    vertex_count: usize,
    tile_count: usize,
) -> Result<(&[u8], &[u8]), String> {
    let vertices_offset = MLIQ_HEADER_SIZE;
    let vertices_end = vertices_offset
        .checked_add(vertex_count * MLIQ_VERTEX_SIZE)
        .ok_or_else(|| "MLIQ vertex byte length overflow".to_string())?;
    let vertices_data = data
        .get(vertices_offset..vertices_end)
        .ok_or_else(|| format!("MLIQ missing vertex payload: {} bytes", data.len()))?;
    let tiles_end = vertices_end
        .checked_add(tile_count * MLIQ_TILE_SIZE)
        .ok_or_else(|| "MLIQ tile byte length overflow".to_string())?;
    let tiles_data = data
        .get(vertices_end..tiles_end)
        .ok_or_else(|| format!("MLIQ missing tile payload: {} bytes", data.len()))?;
    Ok((vertices_data, tiles_data))
}

fn build_mliq(
    header: RawWmoLiquidHeader,
    vertices_data: &[u8],
    tiles_data: &[u8],
) -> Result<WmoLiquid, String> {
    Ok(WmoLiquid {
        header: WmoLiquidHeader {
            x_verts: header.x_verts,
            y_verts: header.y_verts,
            x_tiles: header.x_tiles,
            y_tiles: header.y_tiles,
            position: header.position,
            material_id: header.material_id,
        },
        vertices: parse_mliq_vertices(vertices_data)?,
        tiles: parse_mliq_tiles(tiles_data),
    })
}

fn parse_mliq_vertices(data: &[u8]) -> Result<Vec<WmoLiquidVertex>, String> {
    Ok(
        parse_binrw_entries::<RawWmoLiquidVertex>(data, MLIQ_VERTEX_SIZE, "MLIQ vertices")?
            .into_iter()
            .map(|vertex| WmoLiquidVertex {
                raw: vertex.raw,
                height: vertex.height,
            })
            .collect(),
    )
}

fn parse_mliq_tiles(data: &[u8]) -> Vec<WmoLiquidTile> {
    data.iter()
        .copied()
        .map(|tile| WmoLiquidTile {
            liquid_type: tile & 0x3F,
            fishable: tile & 0x40 != 0,
            shared: tile & 0x80 != 0,
        })
        .collect()
}

fn parse_vec3_array(data: &[u8]) -> Result<Vec<[f32; 3]>, String> {
    parse_binrw_entries(data, VEC3_ENTRY_SIZE, "vec3 array")
}

fn parse_vec2_array(data: &[u8]) -> Result<Vec<[f32; 2]>, String> {
    parse_binrw_entries(data, VEC2_ENTRY_SIZE, "vec2 array")
}

fn parse_u16_array(data: &[u8]) -> Vec<u16> {
    data.chunks_exact(2)
        .map(|c| u16::from_le_bytes(c.try_into().unwrap()))
        .collect()
}

fn parse_bgra_color(color: [u8; 4]) -> [f32; 4] {
    [
        color[2] as f32 / 255.0,
        color[1] as f32 / 255.0,
        color[0] as f32 / 255.0,
        color[3] as f32 / 255.0,
    ]
}

fn parse_mocv(data: &[u8]) -> Vec<[f32; 4]> {
    data.chunks_exact(4)
        .map(|c| parse_bgra_color(c.try_into().unwrap()))
        .collect()
}

fn parse_mocv_alpha(data: &[u8]) -> Vec<f32> {
    data.chunks_exact(4).map(|c| c[3] as f32 / 255.0).collect()
}

pub fn parse_moba(data: &[u8]) -> Result<Vec<RawBatch>, String> {
    Ok(
        parse_binrw_entries::<RawBatchEntry>(data, MOBA_ENTRY_SIZE, "MOBA")?
            .into_iter()
            .map(|batch| {
                let material_id = if batch.material_id_small == 0xFF {
                    batch.material_id_large
                } else {
                    batch.material_id_small as u16
                };
                RawBatch {
                    start_index: batch.start_index,
                    count: batch.count,
                    min_index: batch.min_index,
                    max_index: batch.max_index,
                    material_id,
                }
            })
            .collect(),
    )
}

#[cfg(test)]
#[path = "parser_tests/mod.rs"]
mod tests;
