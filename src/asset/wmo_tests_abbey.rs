use super::*;

#[test]
fn abbey_group_batch_mesh_indices_stay_in_bounds() {
    for fdid in 107075..=107087 {
        let path = format!("data/models/{fdid}.wmo");
        let data = std::fs::read(&path).unwrap_or_else(|_| panic!("missing test asset: {path}"));
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
        assert_group_triangles_match_source(fdid);
    }
}

type PackedTriangle = [u32; 9];

fn assert_group_triangles_match_source(fdid: u32) {
    let path = format!("data/models/{fdid}.wmo");
    let data = std::fs::read(&path).unwrap_or_else(|_| panic!("missing test asset: {path}"));
    let raw = parse_raw_group(&data, &path);
    let group = load_wmo_group(&data).unwrap_or_else(|e| panic!("{path}: {e}"));

    let mut expected = collect_expected_triangles(&raw);
    expected.sort_unstable();
    let mut actual = collect_actual_triangles(&group, &path);
    actual.sort_unstable();

    assert_eq!(
        actual, expected,
        "{path}: reconstructed triangles differ from source"
    );
}

fn parse_raw_group(data: &[u8], path: &str) -> RawGroupData {
    let mogp = find_mogp(data).unwrap_or_else(|e| panic!("{path}: {e}"));
    parse_group_subchunks(&mogp[MOGP_HEADER_SIZE..]).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn collect_expected_triangles(raw: &RawGroupData) -> Vec<PackedTriangle> {
    let mut triangles = Vec::new();
    for batch in &raw.batches {
        collect_batch_source_triangles(raw, batch, &mut triangles);
    }
    triangles
}

fn collect_batch_source_triangles(
    raw: &RawGroupData,
    batch: &RawBatch,
    triangles: &mut Vec<PackedTriangle>,
) {
    let start = batch.start_index as usize;
    let end = start + batch.count as usize;
    for tri in raw.indices[start..end].chunks_exact(3) {
        triangles.push(pack_source_triangle(raw, tri));
    }
}

fn pack_source_triangle(raw: &RawGroupData, tri: &[u16]) -> PackedTriangle {
    let mut packed = [0_u32; 9];
    for (vertex_slot, &index) in tri.iter().enumerate() {
        let pos = raw.vertices[index as usize];
        let bevy_pos = wmo_local_to_bevy(pos[0], pos[1], pos[2]);
        write_packed_vertex(&mut packed, vertex_slot, bevy_pos);
    }
    packed
}

fn collect_actual_triangles(group: &WmoGroupData, path: &str) -> Vec<PackedTriangle> {
    let mut triangles = Vec::new();
    for (batch_idx, batch) in group.batches.iter().enumerate() {
        collect_batch_mesh_triangles(batch, path, batch_idx, &mut triangles);
    }
    triangles
}

fn collect_batch_mesh_triangles(
    batch: &WmoGroupBatch,
    path: &str,
    batch_idx: usize,
    triangles: &mut Vec<PackedTriangle>,
) {
    let positions = mesh_positions(batch, path, batch_idx);
    let indices = mesh_indices(batch, path, batch_idx);
    for tri in indices.chunks_exact(3) {
        triangles.push(pack_mesh_triangle(positions, tri));
    }
}

fn mesh_positions<'a>(batch: &'a WmoGroupBatch, path: &str, batch_idx: usize) -> &'a [[f32; 3]] {
    match batch.mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(bevy::mesh::VertexAttributeValues::Float32x3(values)) => values.as_slice(),
        _ => panic!("{path} batch {batch_idx}: missing positions"),
    }
}

fn mesh_indices(batch: &WmoGroupBatch, path: &str, batch_idx: usize) -> Vec<u32> {
    match batch.mesh.indices() {
        Some(Indices::U16(values)) => values.iter().map(|&index| index as u32).collect(),
        Some(Indices::U32(values)) => values.clone(),
        None => panic!("{path} batch {batch_idx}: missing index buffer"),
    }
}

fn pack_mesh_triangle(positions: &[[f32; 3]], tri: &[u32]) -> PackedTriangle {
    let mut packed = [0_u32; 9];
    for (vertex_slot, &index) in tri.iter().enumerate() {
        write_packed_vertex(&mut packed, vertex_slot, positions[index as usize]);
    }
    packed
}

fn write_packed_vertex(packed: &mut PackedTriangle, vertex_slot: usize, pos: [f32; 3]) {
    let offset = vertex_slot * 3;
    packed[offset] = pos[0].to_bits();
    packed[offset + 1] = pos[1].to_bits();
    packed[offset + 2] = pos[2].to_bits();
}

#[test]
fn abbey_groups_with_mocv_expose_mesh_vertex_colors() {
    for fdid in [107076u32, 107077, 107081, 107084, 107085, 107087] {
        let path = format!("data/models/{fdid}.wmo");
        let data = std::fs::read(&path).unwrap_or_else(|_| panic!("missing test asset: {path}"));
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
