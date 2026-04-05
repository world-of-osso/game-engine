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
        let path = format!("data/models/{fdid}.wmo");
        let data = std::fs::read(&path).unwrap_or_else(|_| panic!("missing test asset: {path}"));
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
                Some(Indices::U16(values)) => values.iter().map(|&i| i as u32).collect::<Vec<_>>(),
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
