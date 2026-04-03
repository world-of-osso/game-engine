use crate::asset::adt::ChunkIter;

fn read_u8(data: &[u8], off: usize) -> Result<u8, String> {
    data.get(off)
        .copied()
        .ok_or_else(|| format!("read_u8 out of bounds at {off:#x}"))
}

fn read_u16(data: &[u8], off: usize) -> Result<u16, String> {
    let bytes: [u8; 2] = data
        .get(off..off + 2)
        .ok_or_else(|| format!("read_u16 out of bounds at {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(u16::from_le_bytes(bytes))
}

fn read_u32(data: &[u8], off: usize) -> Result<u32, String> {
    let bytes: [u8; 4] = data
        .get(off..off + 4)
        .ok_or_else(|| format!("read_u32 out of bounds at {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(u32::from_le_bytes(bytes))
}

fn read_f32(data: &[u8], off: usize) -> Result<f32, String> {
    let bytes: [u8; 4] = data
        .get(off..off + 4)
        .ok_or_else(|| format!("read_f32 out of bounds at {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(f32::from_le_bytes(bytes))
}

#[allow(dead_code)]
pub struct WmoRootData {
    pub n_groups: u32,
    pub materials: Vec<WmoMaterialDef>,
    pub portals: Vec<WmoPortal>,
    pub portal_refs: Vec<WmoPortalRef>,
    pub group_infos: Vec<WmoGroupInfo>,
    pub skybox_wow_path: Option<String>,
}

#[allow(dead_code)]
pub struct WmoPortal {
    pub vertices: Vec<[f32; 3]>,
    pub normal: [f32; 3],
}

#[allow(dead_code)]
pub struct WmoPortalRef {
    pub portal_index: u16,
    pub group_index: u16,
    pub side: i16,
}

#[allow(dead_code)]
pub struct WmoGroupInfo {
    pub flags: u32,
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
}

#[allow(dead_code)]
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

pub const MOGP_HEADER_SIZE: usize = 68;

pub fn wmo_local_to_bevy(x: f32, y: f32, z: f32) -> [f32; 3] {
    [-x, z, y]
}

type PortalsAndRanges = (Vec<WmoPortal>, Vec<(u16, u16)>);

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
            &mut n_groups,
            &mut materials,
            &mut portals,
            &mut mopt_raw,
            &mut portal_refs,
            &mut group_infos,
            &mut skybox_wow_path,
            &mut portal_vertices,
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

#[allow(clippy::too_many_arguments)]
pub fn apply_root_chunk(
    tag: &[u8],
    payload: &[u8],
    n_groups: &mut u32,
    materials: &mut Vec<WmoMaterialDef>,
    portals: &mut Vec<WmoPortal>,
    mopt_raw: &mut Vec<(u16, u16)>,
    portal_refs: &mut Vec<WmoPortalRef>,
    group_infos: &mut Vec<WmoGroupInfo>,
    skybox_wow_path: &mut Option<String>,
    portal_vertices: &mut Vec<[f32; 3]>,
) -> Result<(), String> {
    match tag {
        b"DHOM" => {
            if payload.len() < 64 {
                return Err(format!("MOHD too small: {} bytes", payload.len()));
            }
            *n_groups = read_u32(payload, 4)?;
        }
        b"TMOM" => *materials = parse_momt(payload)?,
        b"VPOM" => *portal_vertices = parse_vec3_array(payload)?,
        b"TPOM" => {
            let (p, raw) = parse_mopt(payload)?;
            *portals = p;
            *mopt_raw = raw;
        }
        b"RPOM" => *portal_refs = parse_mopr(payload)?,
        b"IGOM" => *group_infos = parse_mogi(payload)?,
        b"BSOM" => *skybox_wow_path = parse_c_string(payload),
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
    let count = data.len() / 64;
    let mut mats = Vec::with_capacity(count);
    for i in 0..count {
        let base = i * 64;
        let flags = read_u32(data, base)?;
        let shader = read_u32(data, base + 4)?;
        let blend_mode = read_u32(data, base + 8)?;
        let texture_fdid = read_u32(data, base + 0x0C)?;
        let texture_2_fdid = read_u32(data, base + 0x18)?;
        let texture_3_fdid = read_u32(data, base + 0x24)?;
        mats.push(WmoMaterialDef {
            texture_fdid,
            texture_2_fdid,
            texture_3_fdid,
            flags,
            blend_mode,
            shader,
        });
    }
    Ok(mats)
}

fn parse_mopt(data: &[u8]) -> Result<PortalsAndRanges, String> {
    let count = data.len() / 20;
    let mut portals = Vec::with_capacity(count);
    let mut ranges = Vec::with_capacity(count);
    for i in 0..count {
        let base = i * 20;
        let start_vertex = read_u16(data, base)?;
        let vert_count = read_u16(data, base + 2)?;
        let normal = [
            read_f32(data, base + 4)?,
            read_f32(data, base + 8)?,
            read_f32(data, base + 12)?,
        ];
        portals.push(WmoPortal {
            vertices: Vec::new(),
            normal,
        });
        ranges.push((start_vertex, vert_count));
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
    let count = data.len() / 8;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let base = i * 8;
        let portal_index = read_u16(data, base)?;
        let group_index = read_u16(data, base + 2)?;
        let side = read_u16(data, base + 4)? as i16;
        out.push(WmoPortalRef {
            portal_index,
            group_index,
            side,
        });
    }
    Ok(out)
}

fn parse_mogi(data: &[u8]) -> Result<Vec<WmoGroupInfo>, String> {
    let count = data.len() / 32;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let base = i * 32;
        let flags = read_u32(data, base)?;
        let bbox_min = [
            read_f32(data, base + 4)?,
            read_f32(data, base + 8)?,
            read_f32(data, base + 12)?,
        ];
        let bbox_max = [
            read_f32(data, base + 16)?,
            read_f32(data, base + 20)?,
            read_f32(data, base + 24)?,
        ];
        out.push(WmoGroupInfo {
            flags,
            bbox_min,
            bbox_max,
        });
    }
    Ok(out)
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
    let count = data.len() / 12;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let base = i * 12;
        out.push([
            read_f32(data, base)?,
            read_f32(data, base + 4)?,
            read_f32(data, base + 8)?,
        ]);
    }
    Ok(out)
}

fn parse_vec2_array(data: &[u8]) -> Result<Vec<[f32; 2]>, String> {
    let count = data.len() / 8;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let base = i * 8;
        out.push([read_f32(data, base)?, read_f32(data, base + 4)?]);
    }
    Ok(out)
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
    let count = data.len() / 24;
    let mut batches = Vec::with_capacity(count);
    for i in 0..count {
        let base = i * 24;
        let material_id_large = read_u16(data, base + 0x0A)?;
        let start_index = read_u32(data, base + 0x0C)?;
        let count = read_u16(data, base + 0x10)?;
        let min_index = read_u16(data, base + 0x12)?;
        let max_index = read_u16(data, base + 0x14)?;
        let material_id_small = read_u8(data, base + 0x17)?;
        let material_id = if material_id_small == 0xFF {
            material_id_large
        } else {
            material_id_small as u16
        };
        batches.push(RawBatch {
            start_index,
            count,
            min_index,
            max_index,
            material_id,
        });
    }
    Ok(batches)
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
            &mut n_groups,
            &mut materials,
            &mut portals,
            &mut mopt_raw,
            &mut portal_refs,
            &mut group_infos,
            &mut skybox_wow_path,
            &mut portal_vertices,
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
