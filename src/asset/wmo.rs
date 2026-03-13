use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};

use super::adt::ChunkIter;
use super::m2::wow_to_bevy;

// ── primitive readers ────────────────────────────────────────────────────────

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

// ── types ───────────────────────────────────────────────────────────────────

#[allow(dead_code)]
pub struct WmoRootData {
    pub n_groups: u32,
    pub materials: Vec<WmoMaterialDef>,
    pub portals: Vec<WmoPortal>,
    pub portal_refs: Vec<WmoPortalRef>,
    pub group_infos: Vec<WmoGroupInfo>,
}

/// A portal polygon (doorway/opening between groups).
#[allow(dead_code)]
pub struct WmoPortal {
    pub vertices: Vec<[f32; 3]>,
    pub normal: [f32; 3],
}

/// A portal reference linking a group to a portal and destination group.
#[allow(dead_code)]
pub struct WmoPortalRef {
    pub portal_index: u16,
    pub group_index: u16,
    pub side: i16,
}

/// Per-group info from MOGI chunk: flags and bounding box.
#[allow(dead_code)]
pub struct WmoGroupInfo {
    pub flags: u32,
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
}

#[allow(dead_code)]
pub struct WmoMaterialDef {
    pub texture_fdid: u32,
    pub flags: u32,
    pub blend_mode: u32,
}

pub struct WmoGroupData {
    pub batches: Vec<WmoGroupBatch>,
}

pub struct WmoGroupBatch {
    pub mesh: Mesh,
    pub material_index: u16,
    pub has_vertex_color: bool,
}

/// Type alias for the return value of `parse_mopt`: portals and their vertex ranges.
type PortalsAndRanges = (Vec<WmoPortal>, Vec<(u16, u16)>);

/// Type alias for extracted batch vertex attributes: positions, normals, uvs.
type BatchVertexAttribs = (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 2]>);

// ── root file parsing ───────────────────────────────────────────────────────

/// Parse a WMO root file and return header info + materials.
pub fn load_wmo_root(data: &[u8]) -> Result<WmoRootData, String> {
    let mut n_groups = 0u32;
    let mut materials = Vec::new();
    let mut portals = Vec::new();
    let mut portal_refs = Vec::new();
    let mut group_infos = Vec::new();
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
    })
}

#[allow(clippy::too_many_arguments)]
fn apply_root_chunk(
    tag: &[u8],
    payload: &[u8],
    n_groups: &mut u32,
    materials: &mut Vec<WmoMaterialDef>,
    portals: &mut Vec<WmoPortal>,
    mopt_raw: &mut Vec<(u16, u16)>,
    portal_refs: &mut Vec<WmoPortalRef>,
    group_infos: &mut Vec<WmoGroupInfo>,
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
        _ => {}
    }
    Ok(())
}

/// Parse MOMT chunk: 64 bytes per material entry.
fn parse_momt(data: &[u8]) -> Result<Vec<WmoMaterialDef>, String> {
    let count = data.len() / 64;
    let mut mats = Vec::with_capacity(count);
    for i in 0..count {
        let base = i * 64;
        let flags = read_u32(data, base)?;
        let blend_mode = read_u32(data, base + 8)?;
        let texture_fdid = read_u32(data, base + 0x0C)?;
        mats.push(WmoMaterialDef {
            texture_fdid,
            flags,
            blend_mode,
        });
    }
    Ok(mats)
}

/// Parse MOPT chunk: 20 bytes per portal. Returns portals (with empty vertices) and vertex ranges.
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

/// Resolve portal vertices from MOPV data into parsed portals.
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

/// Parse MOPR chunk: 8 bytes per portal reference.
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

/// Parse MOGI chunk: 32 bytes per group (flags u32, bbox_min [f32;3], bbox_max [f32;3], name_ofs i32).
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

// ── group file parsing ──────────────────────────────────────────────────────

/// Size of the MOGP header before sub-chunks begin.
const MOGP_HEADER_SIZE: usize = 68;

/// Parse a WMO group file and return meshes per render batch.
pub fn load_wmo_group(data: &[u8]) -> Result<WmoGroupData, String> {
    let mogp_payload = find_mogp(data)?;
    if mogp_payload.len() < MOGP_HEADER_SIZE {
        return Err(format!(
            "MOGP payload too small: {} bytes",
            mogp_payload.len()
        ));
    }

    let sub_chunks = &mogp_payload[MOGP_HEADER_SIZE..];
    let raw = parse_group_subchunks(sub_chunks)?;
    build_group_batches(&raw)
}

/// Find the MOGP chunk payload in a group file.
fn find_mogp(data: &[u8]) -> Result<&[u8], String> {
    for chunk in ChunkIter::new(data) {
        let (tag, payload) = chunk?;
        if tag == b"PGOM" {
            return Ok(payload);
        }
    }
    Err("No MOGP chunk found in WMO group file".to_string())
}

struct RawGroupData {
    vertices: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u16>,
    batches: Vec<RawBatch>,
}

struct RawBatch {
    start_index: u32,
    count: u16,
    min_index: u16,
    max_index: u16,
    material_id: u16,
}

/// Parse MOVT, MOVI, MONR, MOTV, MOCV, MOBA sub-chunks from MOGP payload.
fn parse_group_subchunks(data: &[u8]) -> Result<RawGroupData, String> {
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

/// Parse array of [f32; 3] from chunk payload.
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

/// Parse array of [f32; 2] from chunk payload.
fn parse_vec2_array(data: &[u8]) -> Result<Vec<[f32; 2]>, String> {
    let count = data.len() / 8;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let base = i * 8;
        out.push([read_f32(data, base)?, read_f32(data, base + 4)?]);
    }
    Ok(out)
}

/// Parse array of u16 from chunk payload.
fn parse_u16_array(data: &[u8]) -> Vec<u16> {
    data.chunks_exact(2)
        .map(|c| u16::from_le_bytes(c.try_into().unwrap()))
        .collect()
}

/// Parse MOCV vertex colors stored as BGRA bytes.
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

/// Parse MOBA chunk: 24 bytes per render batch.
fn parse_moba(data: &[u8]) -> Result<Vec<RawBatch>, String> {
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

// ── mesh building ───────────────────────────────────────────────────────────

/// Build Bevy meshes from raw group data, one mesh per render batch.
fn build_group_batches(raw: &RawGroupData) -> Result<WmoGroupData, String> {
    let whole_group_has_vertex_color = raw.colors.len() == raw.vertices.len();
    if raw.batches.is_empty() {
        let mesh = build_whole_group_mesh(raw);
        return Ok(WmoGroupData {
            batches: vec![WmoGroupBatch {
                mesh,
                material_index: 0,
                has_vertex_color: whole_group_has_vertex_color,
            }],
        });
    }

    let mut out = Vec::with_capacity(raw.batches.len());
    for batch in &raw.batches {
        let mesh = build_batch_mesh(raw, batch);
        out.push(WmoGroupBatch {
            mesh,
            material_index: batch.material_id,
            has_vertex_color: raw.colors.len() > batch.max_index as usize,
        });
    }
    Ok(WmoGroupData { batches: out })
}

/// Build a Bevy mesh for the entire group (fallback when no MOBA batches).
fn build_whole_group_mesh(raw: &RawGroupData) -> Mesh {
    let positions: Vec<[f32; 3]> = raw
        .vertices
        .iter()
        .map(|v| wow_to_bevy(v[0], v[1], v[2]))
        .collect();
    let normals = convert_normals(&raw.normals, positions.len());
    let uvs = convert_uvs(&raw.uvs, positions.len());
    let colors = convert_colors(&raw.colors, positions.len());
    let indices: Vec<u32> = raw.indices.iter().map(|&i| i as u32).collect();

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    if let Some(colors) = colors {
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Build a Bevy mesh for one render batch, slicing and re-indexing vertices.
fn build_batch_mesh(raw: &RawGroupData, batch: &RawBatch) -> Mesh {
    let vmin = batch.min_index as usize;
    let vmax = (batch.max_index as usize).min(raw.vertices.len().saturating_sub(1));
    let vert_count = vmax - vmin + 1;

    let (positions, normals, uvs) = extract_batch_vertices(raw, vmin, vmax, vert_count);
    let indices = extract_batch_indices(raw, batch);
    let colors = extract_batch_colors(raw, vmin, vmax);

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    if let Some(colors) = colors {
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn extract_batch_vertices(
    raw: &RawGroupData,
    vmin: usize,
    vmax: usize,
    vert_count: usize,
) -> BatchVertexAttribs {
    let positions: Vec<[f32; 3]> = raw.vertices[vmin..=vmax]
        .iter()
        .map(|v| wow_to_bevy(v[0], v[1], v[2]))
        .collect();
    let normals = if raw.normals.len() > vmax {
        raw.normals[vmin..=vmax]
            .iter()
            .map(|n| wow_to_bevy(n[0], n[1], n[2]))
            .collect()
    } else {
        vec![[0.0, 1.0, 0.0]; vert_count]
    };
    let uvs = if raw.uvs.len() > vmax {
        raw.uvs[vmin..=vmax].to_vec()
    } else {
        vec![[0.0, 0.0]; vert_count]
    };
    (positions, normals, uvs)
}

fn extract_batch_indices(raw: &RawGroupData, batch: &RawBatch) -> Vec<u32> {
    let idx_start = batch.start_index as usize;
    let idx_end = (idx_start + batch.count as usize).min(raw.indices.len());
    raw.indices[idx_start..idx_end]
        .iter()
        .map(|&i| (i - batch.min_index) as u32)
        .collect()
}

fn extract_batch_colors(raw: &RawGroupData, vmin: usize, vmax: usize) -> Option<Vec<[f32; 4]>> {
    if raw.colors.len() > vmax {
        Some(raw.colors[vmin..=vmax].to_vec())
    } else {
        None
    }
}

fn convert_normals(src: &[[f32; 3]], expected: usize) -> Vec<[f32; 3]> {
    if src.len() == expected {
        src.iter().map(|n| wow_to_bevy(n[0], n[1], n[2])).collect()
    } else {
        vec![[0.0, 1.0, 0.0]; expected]
    }
}

fn convert_uvs(src: &[[f32; 2]], expected: usize) -> Vec<[f32; 2]> {
    if src.len() == expected {
        src.to_vec()
    } else {
        vec![[0.0, 0.0]; expected]
    }
}

fn convert_colors(src: &[[f32; 4]], expected: usize) -> Option<Vec<[f32; 4]>> {
    (src.len() == expected).then(|| src.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_momt_entry_size() {
        // 64 bytes of zeros should parse as one material with zero fields
        let data = vec![0u8; 64];
        let mats = parse_momt(&data).unwrap();
        assert_eq!(mats.len(), 1);
        assert_eq!(mats[0].texture_fdid, 0);
    }

    #[test]
    fn parse_moba_entry_size() {
        // 24 bytes of zeros should parse as one batch
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

    #[test]
    fn abbey_group_batch_mesh_indices_stay_in_bounds() {
        for fdid in 107075..=107087 {
            let path = format!("data/models/{fdid}.wmo");
            let data =
                std::fs::read(&path).unwrap_or_else(|_| panic!("missing test asset: {path}"));
            let group = load_wmo_group(&data).unwrap_or_else(|e| panic!("{path}: {e}"));

            for (batch_idx, batch) in group.batches.iter().enumerate() {
                let vertex_count = batch.mesh.count_vertices();
                let Some(indices) = batch.mesh.indices() else {
                    panic!("{path} batch {batch_idx}: missing index buffer");
                };

                match indices {
                    Indices::U16(values) => {
                        let offending = values
                            .iter()
                            .copied()
                            .find(|&index| index as usize >= vertex_count);
                        assert!(
                            offending.is_none(),
                            "{path} batch {batch_idx}: index {:?} out of bounds for {vertex_count} vertices",
                            offending
                        );
                    }
                    Indices::U32(values) => {
                        let offending = values
                            .iter()
                            .copied()
                            .find(|&index| index as usize >= vertex_count);
                        assert!(
                            offending.is_none(),
                            "{path} batch {batch_idx}: index {:?} out of bounds for {vertex_count} vertices",
                            offending
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn abbey_group_batches_preserve_source_triangles() {
        for fdid in 107075..=107087 {
            let path = format!("data/models/{fdid}.wmo");
            let data =
                std::fs::read(&path).unwrap_or_else(|_| panic!("missing test asset: {path}"));
            let mogp = find_mogp(&data).unwrap_or_else(|e| panic!("{path}: {e}"));
            let raw = parse_group_subchunks(&mogp[MOGP_HEADER_SIZE..])
                .unwrap_or_else(|e| panic!("{path}: {e}"));
            let group = load_wmo_group(&data).unwrap_or_else(|e| panic!("{path}: {e}"));

            let mut expected = Vec::new();
            for batch in &raw.batches {
                let start = batch.start_index as usize;
                let end = start + batch.count as usize;
                for tri in raw.indices[start..end].chunks_exact(3) {
                    let mut packed = Vec::with_capacity(9);
                    for &index in tri {
                        let pos = raw.vertices[index as usize];
                        let pos = wow_to_bevy(pos[0], pos[1], pos[2]);
                        packed.extend(pos.into_iter().map(f32::to_bits));
                    }
                    expected.push(packed);
                }
            }
            expected.sort_unstable();

            let mut actual = Vec::new();
            for (batch_idx, batch) in group.batches.iter().enumerate() {
                let positions = match batch.mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
                    Some(bevy::mesh::VertexAttributeValues::Float32x3(values)) => values,
                    _ => panic!("{path} batch {batch_idx}: missing positions"),
                };
                let indices = match batch.mesh.indices() {
                    Some(Indices::U16(values)) => {
                        values.iter().map(|&i| i as u32).collect::<Vec<_>>()
                    }
                    Some(Indices::U32(values)) => values.clone(),
                    None => panic!("{path} batch {batch_idx}: missing index buffer"),
                };

                for tri in indices.chunks_exact(3) {
                    let mut packed = Vec::with_capacity(9);
                    for &index in tri {
                        let pos = positions[index as usize];
                        packed.extend(pos.into_iter().map(f32::to_bits));
                    }
                    actual.push(packed);
                }
            }
            actual.sort_unstable();

            assert_eq!(
                actual, expected,
                "{path}: reconstructed triangles differ from source"
            );
        }
    }

    #[test]
    fn abbey_groups_with_mocv_expose_mesh_vertex_colors() {
        for fdid in [107076u32, 107077, 107081, 107084, 107085, 107087] {
            let path = format!("data/models/{fdid}.wmo");
            let data =
                std::fs::read(&path).unwrap_or_else(|_| panic!("missing test asset: {path}"));
            let group = load_wmo_group(&data).unwrap_or_else(|e| panic!("{path}: {e}"));

            assert!(
                group.batches.iter().any(|batch| matches!(
                    batch.mesh.attribute(Mesh::ATTRIBUTE_COLOR),
                    Some(bevy::mesh::VertexAttributeValues::Float32x4(_))
                )),
                "{path}: expected at least one batch with vertex colors"
            );
            assert!(
                group.batches.iter().any(|batch| batch.has_vertex_color),
                "{path}: expected at least one batch flagged as vertex-colored"
            );
        }
    }
}
