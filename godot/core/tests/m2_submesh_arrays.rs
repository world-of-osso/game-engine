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
