use std::io::Cursor;

use binrw::BinRead;

use crate::asset::adt::ChunkIter;

pub struct WmoRootData {
    pub n_groups: u32,
    pub materials: Vec<WmoMaterialDef>,
    pub lights: Vec<WmoLight>,
    pub doodad_sets: Vec<WmoDoodadSet>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WmoLightType {
    Omni = 0,
    Spot = 1,
    Directional = 2,
    Ambient = 3,
}

impl WmoLightType {
    fn from_raw(raw: u8) -> Self {
        match raw {
            1 => Self::Spot,
            2 => Self::Directional,
            3 => Self::Ambient,
            _ => Self::Omni,
        }
    }
}

pub struct WmoLight {
    pub light_type: WmoLightType,
    pub use_attenuation: bool,
    pub color: [f32; 4],
    pub position: [f32; 3],
    pub intensity: f32,
    pub rotation: [f32; 4],
    pub attenuation_start: f32,
    pub attenuation_end: f32,
}

pub struct WmoDoodadSet {
    pub name: String,
    pub start_doodad: u32,
    pub n_doodads: u32,
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
const MOLT_ENTRY_SIZE: usize = 48;
const MODS_ENTRY_SIZE: usize = 32;
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
struct RawWmoLight {
    light_type: u8,
    use_attenuation: u8,
    _padding: [u8; 2],
    color: [u8; 4],
    position: [f32; 3],
    intensity: f32,
    rotation: [f32; 4],
    attenuation_start: f32,
    attenuation_end: f32,
}

#[derive(BinRead)]
#[br(little)]
struct RawWmoDoodadSet {
    name: [u8; 20],
    start_doodad: u32,
    n_doodads: u32,
    _unused: u32,
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
    let mut accum = WmoRootAccum::default();
    load_wmo_root_chunks(data, &mut accum)?;
    Ok(finalize_wmo_root_data(accum))
}

fn load_wmo_root_chunks(data: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        apply_root_chunk(tag, payload, accum)?;
    }
    Ok(())
}

fn finalize_wmo_root_data(mut accum: WmoRootAccum) -> WmoRootData {
    resolve_portal_vertices(&mut accum.portals, &accum.mopt_raw, &accum.portal_vertices);
    WmoRootData {
        n_groups: accum.n_groups,
        materials: accum.materials,
        lights: accum.lights,
        doodad_sets: accum.doodad_sets,
        portals: accum.portals,
        portal_refs: accum.portal_refs,
        group_infos: accum.group_infos,
        skybox_wow_path: accum.skybox_wow_path,
    }
}

#[derive(Default)]
struct WmoRootAccum {
    n_groups: u32,
    materials: Vec<WmoMaterialDef>,
    lights: Vec<WmoLight>,
    doodad_sets: Vec<WmoDoodadSet>,
    portals: Vec<WmoPortal>,
    mopt_raw: Vec<(u16, u16)>,
    portal_refs: Vec<WmoPortalRef>,
    group_infos: Vec<WmoGroupInfo>,
    skybox_wow_path: Option<String>,
    portal_vertices: Vec<[f32; 3]>,
}

fn apply_root_chunk(tag: &[u8], payload: &[u8], accum: &mut WmoRootAccum) -> Result<(), String> {
    match tag {
        b"DHOM" => {
            let header: MohdHeader = parse_binrw_value(payload, MOHD_HEADER_SIZE, "MOHD")?;
            accum.n_groups = header.n_groups;
        }
        b"TMOM" => accum.materials = parse_momt(payload)?,
        b"TLOM" => accum.lights = parse_molt(payload)?,
        b"SDOM" => accum.doodad_sets = parse_mods(payload)?,
        b"VPOM" => accum.portal_vertices = parse_vec3_array(payload)?,
        b"TPOM" => {
            let (p, raw) = parse_mopt(payload)?;
            accum.portals = p;
            accum.mopt_raw = raw;
        }
        b"RPOM" => accum.portal_refs = parse_mopr(payload)?,
        b"IGOM" => accum.group_infos = parse_mogi(payload)?,
        b"BSOM" => accum.skybox_wow_path = parse_c_string(payload),
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
                blend_mode: mat.blend_mode,
                shader: mat.shader,
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
        let mut accum = WmoRootAccum::default();

        apply_root_chunk(b"BSOM", b"environments/stars/deathskybox.m2\0", &mut accum)
            .expect("parse MOSB chunk");

        assert_eq!(
            accum.skybox_wow_path.as_deref(),
            Some("environments/stars/deathskybox.m2")
        );
    }

    #[test]
    fn parse_molt_reads_light_fields() {
        let mut data = Vec::new();
        data.push(1);
        data.push(1);
        data.extend_from_slice(&[0, 0]);
        data.extend_from_slice(&[0x10, 0x20, 0x30, 0x40]);
        for value in [1.0_f32, 2.0, 3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&4.5_f32.to_le_bytes());
        for value in [0.1_f32, 0.2, 0.3, 0.4] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&5.5_f32.to_le_bytes());
        data.extend_from_slice(&9.5_f32.to_le_bytes());

        let lights = parse_molt(&data).expect("parse MOLT");

        assert_eq!(lights.len(), 1);
        let light = &lights[0];
        assert_eq!(light.light_type, WmoLightType::Spot);
        assert!(light.use_attenuation);
        assert_eq!(
            light.color,
            [
                0x30 as f32 / 255.0,
                0x20 as f32 / 255.0,
                0x10 as f32 / 255.0,
                0x40 as f32 / 255.0,
            ]
        );
        assert_eq!(light.position, [1.0, 2.0, 3.0]);
        assert_eq!(light.intensity, 4.5);
        assert_eq!(light.rotation, [0.1, 0.2, 0.3, 0.4]);
        assert_eq!(light.attenuation_start, 5.5);
        assert_eq!(light.attenuation_end, 9.5);
    }

    #[test]
    fn load_wmo_root_reads_molt_lights() {
        let mut data = Vec::new();

        data.extend_from_slice(b"DHOM");
        data.extend_from_slice(&(MOHD_HEADER_SIZE as u32).to_le_bytes());
        let mut mohd = vec![0_u8; MOHD_HEADER_SIZE];
        mohd[4..8].copy_from_slice(&1_u32.to_le_bytes());
        mohd[12..16].copy_from_slice(&1_u32.to_le_bytes());
        data.extend_from_slice(&mohd);

        data.extend_from_slice(b"TLOM");
        data.extend_from_slice(&(MOLT_ENTRY_SIZE as u32).to_le_bytes());
        data.push(2);
        data.push(0);
        data.extend_from_slice(&[0, 0]);
        data.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
        for value in [10.0_f32, 20.0, 30.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&2.25_f32.to_le_bytes());
        for value in [0.0_f32, 0.0, 1.0, 0.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&3.0_f32.to_le_bytes());
        data.extend_from_slice(&7.0_f32.to_le_bytes());

        let root = load_wmo_root(&data).expect("parse WMO root");

        assert_eq!(root.n_groups, 1);
        assert_eq!(root.lights.len(), 1);
        let light = &root.lights[0];
        assert_eq!(light.light_type, WmoLightType::Directional);
        assert!(!light.use_attenuation);
        assert_eq!(light.position, [10.0, 20.0, 30.0]);
        assert_eq!(light.intensity, 2.25);
        assert_eq!(light.rotation, [0.0, 0.0, 1.0, 0.0]);
        assert_eq!(light.attenuation_start, 3.0);
        assert_eq!(light.attenuation_end, 7.0);
    }

    #[test]
    fn parse_mods_reads_doodad_sets() {
        let mut data = Vec::new();
        let mut name = [0_u8; 20];
        name[..14].copy_from_slice(b"$DefaultGlobal");
        data.extend_from_slice(&name);
        data.extend_from_slice(&4_u32.to_le_bytes());
        data.extend_from_slice(&9_u32.to_le_bytes());
        data.extend_from_slice(&0_u32.to_le_bytes());

        let sets = parse_mods(&data).expect("parse MODS");

        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].name, "$DefaultGlobal");
        assert_eq!(sets[0].start_doodad, 4);
        assert_eq!(sets[0].n_doodads, 9);
    }

    #[test]
    fn load_wmo_root_reads_mods_doodad_sets() {
        let mut data = Vec::new();

        data.extend_from_slice(b"DHOM");
        data.extend_from_slice(&(MOHD_HEADER_SIZE as u32).to_le_bytes());
        let mut mohd = vec![0_u8; MOHD_HEADER_SIZE];
        mohd[24..28].copy_from_slice(&2_u32.to_le_bytes());
        data.extend_from_slice(&mohd);

        data.extend_from_slice(b"SDOM");
        data.extend_from_slice(&(64_u32).to_le_bytes());

        let mut first_name = [0_u8; 20];
        first_name[..14].copy_from_slice(b"$DefaultGlobal");
        data.extend_from_slice(&first_name);
        data.extend_from_slice(&0_u32.to_le_bytes());
        data.extend_from_slice(&3_u32.to_le_bytes());
        data.extend_from_slice(&0_u32.to_le_bytes());

        let mut second_name = [0_u8; 20];
        second_name[..7].copy_from_slice(b"FirePit");
        data.extend_from_slice(&second_name);
        data.extend_from_slice(&3_u32.to_le_bytes());
        data.extend_from_slice(&5_u32.to_le_bytes());
        data.extend_from_slice(&0_u32.to_le_bytes());

        let root = load_wmo_root(&data).expect("parse WMO root");

        assert_eq!(root.doodad_sets.len(), 2);
        assert_eq!(root.doodad_sets[0].name, "$DefaultGlobal");
        assert_eq!(root.doodad_sets[0].start_doodad, 0);
        assert_eq!(root.doodad_sets[0].n_doodads, 3);
        assert_eq!(root.doodad_sets[1].name, "FirePit");
        assert_eq!(root.doodad_sets[1].start_doodad, 3);
        assert_eq!(root.doodad_sets[1].n_doodads, 5);
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
