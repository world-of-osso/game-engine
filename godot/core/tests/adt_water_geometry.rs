use game_engine_core::adt::{WaterLayer, build_water_geometry, parse_root};

fn layer() -> WaterLayer {
    WaterLayer {
        liquid_type: 0,
        liquid_object: 3,
        min_height: 2.0,
        max_height: 6.0,
        x_offset: 2,
        y_offset: 3,
        width: 2,
        height: 2,
        exists: [0b10, 0, 0, 0, 0, 0, 0, 0],
        vertex_heights: vec![2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0],
        vertex_uvs: vec![[0.9, 0.8]; 9],
        vertex_depths: vec![0, 64, 255, 128, 192, 255, 255, 255, 255],
    }
}

#[test]
fn masked_offset_quad_preserves_authored_heights_depths_uvs_and_winding() {
    let geometry = build_water_geometry([10.0, 20.0, 30.0], &layer());
    let step = (100.0 / 3.0_f32) / 8.0;
    assert_eq!(
        geometry.positions,
        vec![
            [20.0 - 3.0 * step, 3.0, -10.0 + 3.0 * step],
            [20.0 - 3.0 * step, 4.0, -10.0 + 4.0 * step],
            [20.0 - 4.0 * step, 6.0, -10.0 + 3.0 * step],
            [20.0 - 4.0 * step, 7.0, -10.0 + 4.0 * step],
        ]
    );
    assert_eq!(geometry.normals, vec![[0.0, 1.0, 0.0]; 4]);
    assert_eq!(
        geometry.uvs,
        vec![
            [3.0 / 8.0, 3.0 / 8.0],
            [4.0 / 8.0, 3.0 / 8.0],
            [3.0 / 8.0, 4.0 / 8.0],
            [4.0 / 8.0, 4.0 / 8.0]
        ]
    );
    assert_eq!(
        geometry.colors,
        vec![
            [1.0, 1.0, 1.0, 64.0 / 255.0],
            [1.0, 1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0, 192.0 / 255.0],
            [1.0, 1.0, 1.0, 1.0]
        ]
    );
    assert_eq!(geometry.indices, [0, 2, 1, 2, 3, 1]);
}

#[test]
fn absent_and_short_vertex_payloads_use_original_height_and_opaque_depth() {
    let mut layer = layer();
    layer.vertex_heights.clear();
    layer.vertex_depths.clear();
    let geometry = build_water_geometry([10.0, 20.0, 30.0], &layer);
    assert!(geometry.positions.iter().all(|position| position[1] == 2.0));
    assert!(geometry.colors.iter().all(|color| color[3] == 1.0));
    layer.vertex_heights = vec![0.0];
    layer.vertex_depths = vec![0];
    let short = build_water_geometry([10.0, 20.0, 30.0], &layer);
    assert_eq!(
        short
            .positions
            .iter()
            .map(|position| position[1])
            .collect::<Vec<_>>(),
        vec![2.0; 4]
    );
    assert!(short.colors.iter().all(|color| color[3] == 1.0));
    layer.exists = [0; 8];
    assert!(
        build_water_geometry([10.0, 20.0, 30.0], &layer)
            .indices
            .is_empty()
    );
}

fn triangle_covers_sample(vertices: [[f32; 3]; 3], sample: [f32; 2]) -> bool {
    let side = |a: [f32; 3], b: [f32; 3]| {
        (b[0] - a[0]) * (sample[1] - a[2]) - (b[2] - a[2]) * (sample[0] - a[0])
    };
    let sides = [
        side(vertices[0], vertices[1]),
        side(vertices[1], vertices[2]),
        side(vertices[2], vertices[0]),
    ];
    sides.iter().all(|value| *value >= -0.001) || sides.iter().all(|value| *value <= 0.001)
}

#[test]
fn cached_azeroth_shore_has_authored_water_covering_deep_sample() {
    let bytes = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/terrain/azeroth_32_48.adt"
    ))
    .expect("cached ADT");
    let root = parse_root(&bytes).expect("parse cached ADT");
    let water = root.water.expect("cached MH2O");
    let sample = [-8558.0, 500.0];
    let surface = 143.98892;
    let covered = water
        .chunks
        .iter()
        .zip(&root.chunk_positions)
        .any(|(chunk, position)| {
            chunk.layers.iter().any(|layer| {
                let geometry = build_water_geometry(*position, layer);
                geometry.indices.chunks_exact(3).any(|triangle| {
                    let vertices = [
                        geometry.positions[triangle[0] as usize],
                        geometry.positions[triangle[1] as usize],
                        geometry.positions[triangle[2] as usize],
                    ];
                    triangle_covers_sample(vertices, sample)
                        && vertices
                            .iter()
                            .all(|vertex| (vertex[1] - surface).abs() < 0.2)
                })
            })
        });
    assert!(
        covered,
        "cached authored water misses deep-water shore sample"
    );
}
