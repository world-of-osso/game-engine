use game_engine_core::m2;

fn boar() -> m2::Model {
    let read = |name: &str| {
        let path = format!("{}/../../data/models/{name}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
    };
    m2::parse_model(&read("boar.m2"), &read("boar00.skin")).unwrap()
}

/// Every submesh triangle keeps its vertices, in engine axes, wound the other way.
#[test]
fn submesh_streams_hold_each_triangle_in_engine_axes_with_reversed_winding() {
    let model = boar();
    for sub in &model.submeshes {
        let arrays = m2::submesh_arrays(&model, sub).unwrap();
        let start = sub.triangle_start as usize;
        let triangles = &model.indices[start..start + sub.triangle_count as usize];
        assert_eq!(arrays.indices.len(), triangles.len());
        assert_eq!(arrays.bones.len(), arrays.positions.len() * 4);
        assert_eq!(arrays.weights.len(), arrays.positions.len() * 4);
        for (authored, local) in triangles.chunks(3).zip(arrays.indices.chunks(3)) {
            for (corner, &global) in [0, 2, 1].into_iter().zip(authored) {
                let vertex = &model.vertices[global as usize];
                let [x, y, z] = vertex.position;
                let local = local[corner] as usize;
                assert_eq!(arrays.positions[local], [x, z, -y]);
                assert_eq!(arrays.uv[local], vertex.tex_coords);
                assert_eq!(
                    arrays.weights[local * 4],
                    f32::from(vertex.bone_weights[0]) / 255.0
                );
            }
        }
    }
}

#[test]
fn a_submesh_past_the_index_buffer_is_an_error() {
    let model = boar();
    let past = m2::Submesh {
        mesh_part_id: 0,
        vertex_start: 0,
        vertex_count: 0,
        triangle_start: model.indices.len() as u32,
        triangle_count: 3,
    };
    assert_eq!(
        m2::submesh_arrays(&model, &past).unwrap_err(),
        "Submesh indices out of bounds"
    );
}

/// A merged surface draws each submesh's triangles on that submesh's own vertices.
#[test]
fn merged_streams_draw_every_submesh_triangle_on_its_own_vertices() {
    let model = boar();
    assert_eq!(model.submeshes.len(), 2);
    let subs: Vec<&m2::Submesh> = model.submeshes.iter().collect();
    let merged = m2::merged_submesh_arrays(&model, &subs).unwrap();
    let parts: Vec<m2::SubmeshArrays> = subs
        .iter()
        .map(|sub| m2::submesh_arrays(&model, sub).unwrap())
        .collect();
    let corner = |arrays: &m2::SubmeshArrays, index: i32| {
        let index = index as usize;
        (
            arrays.positions[index],
            arrays.uv[index],
            arrays.bones[index * 4..index * 4 + 4].to_vec(),
        )
    };
    let mut drawn = Vec::new();
    for part in &parts {
        drawn.extend(part.indices.iter().map(|&index| corner(part, index)));
    }
    let merged_drawn: Vec<_> = merged
        .indices
        .iter()
        .map(|&index| corner(&merged, index))
        .collect();
    assert_eq!(merged_drawn, drawn);
    assert_eq!(
        merged.positions.len(),
        parts[0].positions.len() + parts[1].positions.len()
    );
    assert_eq!(merged.weights.len(), merged.positions.len() * 4);
}
