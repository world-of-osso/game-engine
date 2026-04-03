use std::io::Cursor;

use binrw::BinRead;

use crate::asset::adt::ChunkIter;

pub struct WmoRootData {
    pub n_groups: u32,
    pub materials: Vec<WmoMaterialDef>,
    pub portals: Vec<WmoPortal>,
    pub portal_refs: Vec<WmoPortalRef>,
    pub group_infos: Vec<WmoGroupInfo>,
    pub skybox_wow_path: Option<String>,
}

pub struct WmoPortal {
    pub vertices: Vec<[f32; 3]>,
    pub normal: [f32; 3],
}

pub struct WmoPortalRef {
    pub portal_index: u16,
    pub group_index: u16,
    pub side: i16,
}

pub struct WmoGroupInfo {
    pub flags: u32,
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
}

pub struct WmoMaterialDef {
    pub texture_fdid: u32,
    pub texture_2_fdid: u32,
    pub texture_3_fdid: u32,
    pub flags: u32,
    pub blend_mode: u32,
    pub shader: u32,
}

pub struct RawGroupData {
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub colors: Vec<[f32; 4]>,
    pub indices: Vec<u16>,
    pub batches: Vec<RawBatch>,
}

pub struct RawBatch {
    pub start_index: u32,
    pub count: u16,
    pub min_index: u16,
    pub max_index: u16,
    pub material_id: u16,
}

const MOHD_HEADER_SIZE: usize = 64;
const MOMT_ENTRY_SIZE: usize = 64;
const MOPT_ENTRY_SIZE: usize = 20;
const MOPR_ENTRY_SIZE: usize = 8;
const MOGI_ENTRY_SIZE: usize = 32;
const VEC3_ENTRY_SIZE: usize = 12;
const VEC2_ENTRY_SIZE: usize = 8;
const MOBA_ENTRY_SIZE: usize = 24;

#[derive(BinRead)]
#[br(little)]
struct MohdHeader {
    _n_materials: u32,
    n_groups: u32,
    _n_portals: u32,
    _n_lights: u32,
    _n_models: u32,
    _n_doodads: u32,
    _n_sets: u32,
    _ambient_color: u32,
    _wmo_id: u32,
    _bbox_min: [f32; 3],
    _bbox_max: [f32; 3],
    _flags: u16,
    _n_lod: u16,
}

#[derive(BinRead)]
#[br(little)]
struct RawWmoMaterialDef {
    flags: u32,
    shader: u32,
    blend_mode: u32,
    texture_fdid: u32,
    _sidn_emissive_color: u32,
    _frame_sidn_runtime_data: [u32; 2],
    texture_2_fdid: u32,
    _diff_color: u32,
    texture_3_fdid: u32,
    _color_2: u32,
    _terrain_type: u32,
    _texture_3_flags: u32,
    _run_time_data: [u32; 3],
}

#[derive(BinRead)]
#[br(little)]
struct RawWmoPortal {
    start_vertex: u16,
    vert_count: u16,
    normal: [f32; 3],
    _unknown: f32,
}

#[derive(BinRead)]
#[br(little)]
struct RawWmoPortalRef {
    portal_index: u16,
    group_index: u16,
    side: i16,
    _padding: u16,
}

#[derive(BinRead)]
#[br(little)]
struct RawWmoGroupInfo {
    flags: u32,
    bbox_min: [f32; 3],
    bbox_max: [f32; 3],
    _name_offset: u32,
}

#[derive(BinRead)]
#[br(little)]
struct RawBatchEntry {
    _possible_box_1: [u8; 10],
    material_id_large: u16,
    start_index: u32,
    count: u16,
    min_index: u16,
    max_index: u16,
    _possible_box_2: u8,
    material_id_small: u8,
}

pub const MOGP_HEADER_SIZE: usize = 68;

pub fn wmo_local_to_bevy(x: f32, y: f32, z: f32) -> [f32; 3] {
    [-x, z, y]
}

type PortalsAndRanges = (Vec<WmoPortal>, Vec<(u16, u16)>);

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
    let mut n_groups = 0u32;
    let mut materials = Vec::new();
    let mut portals = Vec::new();
    let mut portal_refs = Vec::new();
    let mut group_infos = Vec::new();
    let mut skybox_wow_path = None;
    let mut portal_vertices: Vec<[f32; 3]> = Vec::new();
    let mut mopt_raw: Vec<(u16, u16)> = Vec::new();

    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        apply_root_chunk(
            tag,
            payload,
            &mut WmoRootChunkState {
                n_groups: &mut n_groups,
                materials: &mut materials,
                portals: &mut portals,
                mopt_raw: &mut mopt_raw,
                portal_refs: &mut portal_refs,
                group_infos: &mut group_infos,
                skybox_wow_path: &mut skybox_wow_path,
                portal_vertices: &mut portal_vertices,
            },
        )?;
    }

    resolve_portal_vertices(&mut portals, &mopt_raw, &portal_vertices);
    Ok(WmoRootData {
        n_groups,
        materials,
        portals,
        portal_refs,
        group_infos,
        skybox_wow_path,
    })
}

struct WmoRootChunkState<'a> {
    n_groups: &'a mut u32,
    materials: &'a mut Vec<WmoMaterialDef>,
    portals: &'a mut Vec<WmoPortal>,
    mopt_raw: &'a mut Vec<(u16, u16)>,
    portal_refs: &'a mut Vec<WmoPortalRef>,
    group_infos: &'a mut Vec<WmoGroupInfo>,
    skybox_wow_path: &'a mut Option<String>,
    portal_vertices: &'a mut Vec<[f32; 3]>,
}

fn apply_root_chunk(
    tag: &[u8],
    payload: &[u8],
    state: &mut WmoRootChunkState<'_>,
) -> Result<(), String> {
    match tag {
        b"DHOM" => {
            let header: MohdHeader = parse_binrw_value(payload, MOHD_HEADER_SIZE, "MOHD")?;
            *state.n_groups = header.n_groups;
        }
        b"TMOM" => *state.materials = parse_momt(payload)?,
        b"VPOM" => *state.portal_vertices = parse_vec3_array(payload)?,
        b"TPOM" => {
            let (p, raw) = parse_mopt(payload)?;
            *state.portals = p;
            *state.mopt_raw = raw;
        }
        b"RPOM" => *state.portal_refs = parse_mopr(payload)?,
        b"IGOM" => *state.group_infos = parse_mogi(payload)?,
        b"BSOM" => *state.skybox_wow_path = parse_c_string(payload),
        _ => {}
    }
    Ok(())
}

fn parse_c_string(data: &[u8]) -> Option<String> {
    let nul = data.iter().position(|&b| b == 0).unwrap_or(data.len());
    let bytes = &data[..nul];
    if bytes.is_empty() {
        return None;
    }
    Some(String::from_utf8_lossy(bytes).into_owned())
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
                blend_mode: mat.blend_mode,
                shader: mat.shader,
            })
            .collect(),
    )
}

fn parse_mopt(data: &[u8]) -> Result<PortalsAndRanges, String> {
    let raw = parse_binrw_entries::<RawWmoPortal>(data, MOPT_ENTRY_SIZE, "MOPT")?;
    let mut portals = Vec::with_capacity(raw.len());
    let mut ranges = Vec::with_capacity(raw.len());
    for portal in raw {
        portals.push(WmoPortal {
            vertices: Vec::new(),
            normal: portal.normal,
        });
        ranges.push((portal.start_vertex, portal.vert_count));
    }
    Ok((portals, ranges))
}

fn resolve_portal_vertices(
    portals: &mut [WmoPortal],
    ranges: &[(u16, u16)],
    vertices: &[[f32; 3]],
) {
    for (portal, &(start, count)) in portals.iter_mut().zip(ranges.iter()) {
        let s = start as usize;
        let e = (s + count as usize).min(vertices.len());
        if s < vertices.len() {
            portal.vertices = vertices[s..e].to_vec();
        }
    }
}

fn parse_mopr(data: &[u8]) -> Result<Vec<WmoPortalRef>, String> {
    Ok(
        parse_binrw_entries::<RawWmoPortalRef>(data, MOPR_ENTRY_SIZE, "MOPR")?
            .into_iter()
            .map(|portal_ref| WmoPortalRef {
                portal_index: portal_ref.portal_index,
                group_index: portal_ref.group_index,
                side: portal_ref.side,
            })
            .collect(),
    )
}

fn parse_mogi(data: &[u8]) -> Result<Vec<WmoGroupInfo>, String> {
    Ok(
        parse_binrw_entries::<RawWmoGroupInfo>(data, MOGI_ENTRY_SIZE, "MOGI")?
            .into_iter()
            .map(|group| WmoGroupInfo {
                flags: group.flags,
                bbox_min: group.bbox_min,
                bbox_max: group.bbox_max,
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

pub fn parse_group_subchunks(data: &[u8]) -> Result<RawGroupData, String> {
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    let mut batches = Vec::new();

    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        match tag {
            b"TVOM" => vertices = parse_vec3_array(payload)?,
            b"RNOM" => normals = parse_vec3_array(payload)?,
            b"VTOM" => uvs = parse_vec2_array(payload)?,
            b"VCOM" => colors = parse_mocv(payload),
            b"IVOM" => indices = parse_u16_array(payload),
            b"ABOM" => batches = parse_moba(payload)?,
            _ => {}
        }
    }

    if vertices.is_empty() {
        return Err("WMO group missing MOVT (vertices)".to_string());
    }
    if indices.is_empty() {
        return Err("WMO group missing MOVI (indices)".to_string());
    }

    Ok(RawGroupData {
        vertices,
        normals,
        uvs,
        colors,
        indices,
        batches,
    })
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

fn parse_mocv(data: &[u8]) -> Vec<[f32; 4]> {
    data.chunks_exact(4)
        .map(|c| {
            [
                c[2] as f32 / 255.0,
                c[1] as f32 / 255.0,
                c[0] as f32 / 255.0,
                c[3] as f32 / 255.0,
            ]
        })
        .collect()
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
mod tests {
    use super::*;

    #[test]
    fn parse_momt_entry_size() {
        let data = vec![0u8; 64];
        let mats = parse_momt(&data).unwrap();
        assert_eq!(mats.len(), 1);
        assert_eq!(mats[0].texture_fdid, 0);
    }

    #[test]
    fn parse_root_chunk_reads_mosb_skybox_name() {
        let mut n_groups = 0;
        let mut materials = Vec::new();
        let mut portals = Vec::new();
        let mut portal_refs = Vec::new();
        let mut group_infos = Vec::new();
        let mut skybox_wow_path = None;
        let mut portal_vertices = Vec::new();
        let mut mopt_raw = Vec::new();

        apply_root_chunk(
            b"BSOM",
            b"environments/stars/deathskybox.m2\0",
            &mut WmoRootChunkState {
                n_groups: &mut n_groups,
                materials: &mut materials,
                portals: &mut portals,
                mopt_raw: &mut mopt_raw,
                portal_refs: &mut portal_refs,
                group_infos: &mut group_infos,
                skybox_wow_path: &mut skybox_wow_path,
                portal_vertices: &mut portal_vertices,
            },
        )
        .expect("parse MOSB chunk");

        assert_eq!(
            skybox_wow_path.as_deref(),
            Some("environments/stars/deathskybox.m2")
        );
    }

    #[test]
    fn parse_moba_entry_size() {
        let data = vec![0u8; 24];
        let batches = parse_moba(&data).unwrap();
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].material_id, 0);
    }

    #[test]
    fn parse_mocv_bgra_to_rgba() {
        let colors = parse_mocv(&[0x11, 0x22, 0x33, 0x44]);
        assert_eq!(colors.len(), 1);
        assert_eq!(
            colors[0],
            [
                0x33 as f32 / 255.0,
                0x22 as f32 / 255.0,
                0x11 as f32 / 255.0,
                0x44 as f32 / 255.0,
            ]
        );
    }
}
