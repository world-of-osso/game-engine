use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};

pub use super::wmo_format::parser::{
    MOGP_HEADER_SIZE, RawBatch, RawGroupData, WmoGroupHeader, WmoGroupInfo, WmoLiquid,
    WmoMaterialDef, WmoPortal, WmoPortalRef, WmoRootData, find_mogp, load_wmo_root,
    parse_group_subchunks, parse_mogp_header, wmo_local_to_bevy,
};

pub struct WmoGroupData {
    pub header: WmoGroupHeader,
    pub doodad_refs: Vec<u16>,
    pub liquid: Option<WmoLiquid>,
    pub batches: Vec<WmoGroupBatch>,
}

pub struct WmoGroupBatch {
    pub mesh: Mesh,
    pub material_index: u16,
    pub has_vertex_color: bool,
}

type BatchVertexAttribs = (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 2]>);

pub fn load_wmo_group(data: &[u8]) -> Result<WmoGroupData, String> {
    let mogp_payload = find_mogp(data)?;
    if mogp_payload.len() < MOGP_HEADER_SIZE {
        return Err(format!(
            "MOGP payload too small: {} bytes",
            mogp_payload.len()
        ));
    }

    let header = parse_mogp_header(mogp_payload)?;
    let sub_chunks = &mogp_payload[MOGP_HEADER_SIZE..];
    let raw = parse_group_subchunks(sub_chunks)?;
    build_group_batches(header, raw)
}

fn build_group_batches(header: WmoGroupHeader, raw: RawGroupData) -> Result<WmoGroupData, String> {
    let whole_group_has_vertex_color = raw.colors.len() == raw.vertices.len();
    if raw.batches.is_empty() {
        let mesh = build_whole_group_mesh(&raw);
        return Ok(WmoGroupData {
            header,
            doodad_refs: raw.doodad_refs,
            liquid: raw.liquid,
            batches: vec![WmoGroupBatch {
                mesh,
                material_index: 0,
                has_vertex_color: whole_group_has_vertex_color,
            }],
        });
    }

    let mut out = Vec::with_capacity(raw.batches.len());
    for batch in &raw.batches {
        let mesh = build_batch_mesh(&raw, batch);
        out.push(WmoGroupBatch {
            mesh,
            material_index: batch.material_id,
            has_vertex_color: raw.colors.len() > batch.max_index as usize,
        });
    }
    Ok(WmoGroupData {
        header,
        doodad_refs: raw.doodad_refs,
        liquid: raw.liquid,
        batches: out,
    })
}

fn build_whole_group_mesh(raw: &RawGroupData) -> Mesh {
    let positions: Vec<[f32; 3]> = raw
        .vertices
        .iter()
        .map(|v| wmo_local_to_bevy(v[0], v[1], v[2]))
        .collect();
    let normals = convert_normals(&raw.normals, positions.len());
    let uvs = convert_uvs(&raw.uvs, positions.len());
    let colors = convert_colors(&raw.colors, positions.len());
    let indices = extract_renderable_whole_group_indices(raw);

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
        .map(|v| wmo_local_to_bevy(v[0], v[1], v[2]))
        .collect();
    let normals = if raw.normals.len() > vmax {
        raw.normals[vmin..=vmax]
            .iter()
            .map(|n| wmo_local_to_bevy(n[0], n[1], n[2]))
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
    let mut out = Vec::with_capacity(idx_end.saturating_sub(idx_start));
    for tri_start in (idx_start..idx_end).step_by(3) {
        if tri_start + 3 > idx_end || !triangle_is_renderable(raw, tri_start / 3) {
            continue;
        }
        out.extend(
            raw.indices[tri_start..tri_start + 3]
                .iter()
                .map(|&i| (i - batch.min_index) as u32),
        );
    }
    out
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
        src.iter()
            .map(|n| wmo_local_to_bevy(n[0], n[1], n[2]))
            .collect()
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

fn extract_renderable_whole_group_indices(raw: &RawGroupData) -> Vec<u32> {
    let mut out = Vec::with_capacity(raw.indices.len());
    for tri_start in (0..raw.indices.len()).step_by(3) {
        if tri_start + 3 > raw.indices.len() || !triangle_is_renderable(raw, tri_start / 3) {
            continue;
        }
        out.extend(
            raw.indices[tri_start..tri_start + 3]
                .iter()
                .map(|&i| i as u32),
        );
    }
    out
}

fn triangle_is_renderable(raw: &RawGroupData, triangle_index: usize) -> bool {
    raw.triangle_materials
        .get(triangle_index)
        .is_none_or(|triangle| triangle.material_id != 0xFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_wmo_group_reads_mogp_header_fields() {
        let mut data = Vec::new();
        let mogp_size = MOGP_HEADER_SIZE as u32 + 8 + 12 + 8 + 6;
        data.extend_from_slice(b"PGOM");
        data.extend_from_slice(&mogp_size.to_le_bytes());
        data.extend_from_slice(&12_u32.to_le_bytes());
        data.extend_from_slice(&34_u32.to_le_bytes());
        data.extend_from_slice(&0x0102_0304_u32.to_le_bytes());
        for value in [-1.0_f32, -2.0, -3.0, 4.0, 5.0, 6.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&7_u16.to_le_bytes());
        data.extend_from_slice(&8_u16.to_le_bytes());
        data.extend_from_slice(&9_u16.to_le_bytes());
        data.extend_from_slice(&10_u16.to_le_bytes());
        data.extend_from_slice(&11_u16.to_le_bytes());
        data.extend_from_slice(&12_u16.to_le_bytes());
        data.extend_from_slice(&[1_u8, 2, 3, 4]);
        data.extend_from_slice(&13_u32.to_le_bytes());
        data.extend_from_slice(&14_u32.to_le_bytes());
        data.extend_from_slice(&15_u32.to_le_bytes());
        data.extend_from_slice(&(-16_i16).to_le_bytes());
        data.extend_from_slice(&17_i16.to_le_bytes());

        data.extend_from_slice(b"TVOM");
        data.extend_from_slice(&(12_u32).to_le_bytes());
        for value in [1.0_f32, 2.0, 3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"IVOM");
        data.extend_from_slice(&(6_u32).to_le_bytes());
        data.extend_from_slice(&0_u16.to_le_bytes());
        data.extend_from_slice(&0_u16.to_le_bytes());
        data.extend_from_slice(&0_u16.to_le_bytes());

        let group = load_wmo_group(&data).expect("parse WMO group");

        assert_eq!(group.header.group_name_offset, 12);
        assert_eq!(group.header.descriptive_group_name_offset, 34);
        assert_eq!(group.header.flags, 0x0102_0304);
        assert_eq!(group.header.portal_start, 7);
        assert_eq!(group.header.portal_count, 8);
        assert_eq!(group.header.trans_batch_count, 9);
        assert_eq!(group.header.int_batch_count, 10);
        assert_eq!(group.header.ext_batch_count, 11);
        assert_eq!(group.header.batch_type_d, 12);
        assert_eq!(group.header.fog_ids, [1, 2, 3, 4]);
        assert_eq!(group.header.group_liquid, 13);
        assert_eq!(group.header.unique_id, 14);
        assert_eq!(group.header.flags2, 15);
        assert_eq!(group.header.parent_split_group_index, -16);
        assert_eq!(group.header.next_split_child_group_index, 17);
    }

    #[test]
    fn load_wmo_group_skips_collision_only_mopy_triangles() {
        let mut data = Vec::new();
        let mogp_size = MOGP_HEADER_SIZE as u32 + 12 + 20 + 20;
        data.extend_from_slice(b"PGOM");
        data.extend_from_slice(&mogp_size.to_le_bytes());
        data.extend_from_slice(&[0_u8; MOGP_HEADER_SIZE]);

        data.extend_from_slice(b"YPOM");
        data.extend_from_slice(&(4_u32).to_le_bytes());
        data.extend_from_slice(&[0x20_u8, 0x01, 0x20, 0xFF]);

        data.extend_from_slice(b"TVOM");
        data.extend_from_slice(&(12_u32).to_le_bytes());
        for value in [1.0_f32, 2.0, 3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"IVOM");
        data.extend_from_slice(&(12_u32).to_le_bytes());
        for index in [0_u16, 0, 0, 0, 0, 0] {
            data.extend_from_slice(&index.to_le_bytes());
        }

        let group = load_wmo_group(&data).expect("parse WMO group");
        let indices = match group.batches[0].mesh.indices() {
            Some(Indices::U32(values)) => values.clone(),
            other => panic!("unexpected index buffer: {other:?}"),
        };

        assert_eq!(indices, vec![0, 0, 0]);
    }

    #[test]
    fn load_wmo_group_reads_mliq_liquid_data() {
        let mut data = Vec::new();
        let mliq_size = 30_u32 + 4 * 8 + 1;
        let mogp_size = MOGP_HEADER_SIZE as u32 + 8 + mliq_size + 8 + 12 + 8 + 6;
        data.extend_from_slice(b"PGOM");
        data.extend_from_slice(&mogp_size.to_le_bytes());
        data.extend_from_slice(&[0_u8; MOGP_HEADER_SIZE]);

        data.extend_from_slice(b"QILM");
        data.extend_from_slice(&mliq_size.to_le_bytes());
        data.extend_from_slice(&2_i32.to_le_bytes());
        data.extend_from_slice(&2_i32.to_le_bytes());
        data.extend_from_slice(&1_i32.to_le_bytes());
        data.extend_from_slice(&1_i32.to_le_bytes());
        for value in [10.0_f32, 20.0, 30.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }
        data.extend_from_slice(&9_i16.to_le_bytes());
        for (raw, height) in [
            ([1_u8, 2, 3, 4], 100.0_f32),
            ([5_u8, 6, 7, 8], 101.0),
            ([9_u8, 10, 11, 12], 102.0),
            ([13_u8, 14, 15, 16], 103.0),
        ] {
            data.extend_from_slice(&raw);
            data.extend_from_slice(&height.to_le_bytes());
        }
        data.push(0b0100_0011);

        data.extend_from_slice(b"TVOM");
        data.extend_from_slice(&(12_u32).to_le_bytes());
        for value in [1.0_f32, 2.0, 3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"IVOM");
        data.extend_from_slice(&(6_u32).to_le_bytes());
        data.extend_from_slice(&0_u16.to_le_bytes());
        data.extend_from_slice(&0_u16.to_le_bytes());
        data.extend_from_slice(&0_u16.to_le_bytes());

        let group = load_wmo_group(&data).expect("parse WMO group");
        let liquid = group.liquid.expect("group liquid");

        assert_eq!(liquid.header.material_id, 9);
        assert_eq!(liquid.header.position, [10.0, 20.0, 30.0]);
        assert_eq!(liquid.vertices.len(), 4);
        assert_eq!(liquid.vertices[1].raw, [5, 6, 7, 8]);
        assert_eq!(liquid.vertices[2].height, 102.0);
        assert_eq!(liquid.tiles.len(), 1);
        assert_eq!(liquid.tiles[0].liquid_type, 3);
        assert!(liquid.tiles[0].fishable);
        assert!(!liquid.tiles[0].shared);
    }

    #[test]
    fn load_wmo_group_reads_modr_doodad_refs() {
        let mut data = Vec::new();
        let modr_size = 6_u32;
        let mogp_size = MOGP_HEADER_SIZE as u32 + 8 + modr_size + 8 + 12 + 8 + 6;
        data.extend_from_slice(b"PGOM");
        data.extend_from_slice(&mogp_size.to_le_bytes());
        data.extend_from_slice(&[0_u8; MOGP_HEADER_SIZE]);

        data.extend_from_slice(b"RDOM");
        data.extend_from_slice(&modr_size.to_le_bytes());
        for value in [4_u16, 9, 15] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"TVOM");
        data.extend_from_slice(&(12_u32).to_le_bytes());
        for value in [1.0_f32, 2.0, 3.0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        data.extend_from_slice(b"IVOM");
        data.extend_from_slice(&(6_u32).to_le_bytes());
        for value in [0_u16, 0, 0] {
            data.extend_from_slice(&value.to_le_bytes());
        }

        let group = load_wmo_group(&data).expect("parse WMO group");

        assert_eq!(group.doodad_refs, vec![4, 9, 15]);
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
                        let pos = wmo_local_to_bevy(pos[0], pos[1], pos[2]);
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
