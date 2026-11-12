use game_engine_core::adt::{WaterLayer, build_water_geometry, parse_root};
use game_engine_core::terrain_height_data::{bevy_to_chunk_coords, sample_chunk_height};

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
        object_vertex_bytes: Vec::new(),
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

/// G7/I2 B.5: the coverage bits gate triangles only. Masking tiles away may remove vertices
/// and indices, but it must never change a surviving vertex's authored position, UV or
/// depth channel, and no coverage value may be blended into an attribute.
#[test]
fn coverage_selects_tiles_and_leaves_authored_vertex_values_untouched() {
    let masked_layer = |exists: [u8; 8]| WaterLayer {
        liquid_type: 5,
        liquid_object: 0,
        min_height: 10.0,
        max_height: 13.0,
        x_offset: 1,
        y_offset: 2,
        width: 3,
        height: 3,
        exists,
        vertex_heights: (0..16).map(|index| 10.0 + index as f32).collect(),
        vertex_uvs: vec![[0.0, 0.0]; 16],
        vertex_depths: (0..16)
            .map(|index| u8::try_from(index * 17).unwrap())
            .collect(),
        object_vertex_bytes: Vec::new(),
    };
    let full = build_water_geometry([10.0, 20.0, 30.0], &masked_layer([0b111; 8]));
    assert_eq!(full.indices.len(), 9 * 6);
    assert_eq!(full.positions.len(), 9 * 4);

    // Only the centre tile (layer row 1, column 1) stays present.
    let mut exists = [0u8; 8];
    exists[1] = 0b010;
    let masked = build_water_geometry([10.0, 20.0, 30.0], &masked_layer(exists));
    assert_eq!(masked.indices, [0, 2, 1, 2, 3, 1]);
    assert_eq!(masked.positions.len(), 4);

    // The surviving quad carries exactly the centre tile's authored grid corners.
    let step = (100.0 / 3.0_f32) / 8.0;
    for (corner, (row, column)) in [(1, 1), (1, 2), (2, 1), (2, 2)].into_iter().enumerate() {
        let grid_index = row * 4 + column;
        let expected_depth = u8::try_from(grid_index * 17).unwrap();
        assert_eq!(
            masked.positions[corner],
            [
                20.0 - (2 + row) as f32 * step,
                10.0 + grid_index as f32,
                -(10.0 - (1 + column) as f32 * step)
            ],
            "corner {corner} row {row} column {column}"
        );
        assert_eq!(
            masked.colors[corner],
            [1.0, 1.0, 1.0, f32::from(expected_depth) / 255.0]
        );
        assert_eq!(
            masked.uvs[corner],
            [(1 + column) as f32 / 8.0, (2 + row) as f32 / 8.0]
        );
    }

    // Masking other tiles away leaves the centre quad's attributes bit-identical.
    let attributes = |geometry: &game_engine_core::adt::WaterGeometry| {
        geometry
            .positions
            .iter()
            .zip(&geometry.uvs)
            .zip(&geometry.colors)
            .map(|((position, uv), color)| (*position, *uv, *color))
            .collect::<Vec<_>>()
    };
    for vertex in attributes(&masked) {
        assert!(
            attributes(&full).contains(&vertex),
            "masking altered a vertex value: {vertex:?}"
        );
    }
}

/// G7: the authored depth byte is water depth in 1/9-yard units and pairs with the authored
/// height on the mirrored terrain grid. Rebuilding the emitted mesh and comparing each vertex
/// against the ADT terrain height must land inside one byte quantum (0.111 yd); a wrong unit
/// or a swapped row/column axis would not.
#[test]
fn emitted_depth_bytes_match_terrain_at_nine_units_per_yard() {
    let bytes = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/terrain/azeroth_32_48.adt"
    ))
    .expect("cached ADT");
    let root = parse_root(&bytes).expect("parse cached ADT");
    let water = root.water.expect("cached MH2O");
    let quantum = 1.0 / 9.0;
    let mut deep = 0;
    let mut shore_above = 0;
    let mut max_deep_residual = 0.0_f32;
    let mut max_shore_gap = 0.0_f32;
    for (index, chunk) in water.chunks.iter().enumerate() {
        let position = root.chunk_positions[index];
        for layer in &chunk.layers {
            let geometry = build_water_geometry(position, layer);
            for (vertex, color) in geometry.positions.iter().zip(&geometry.colors) {
                let depth_byte = (color[3] * 255.0).round() as u8;
                let bevy_z = vertex[2];
                let (_, (index_x, index_y)) = bevy_to_chunk_coords(vertex[0], bevy_z);
                let Some(grid) = root
                    .height_grids
                    .iter()
                    .find(|grid| grid.index_x == index_x && grid.index_y == index_y)
                else {
                    continue;
                };
                let Some(terrain) = sample_chunk_height(grid, vertex[0], bevy_z) else {
                    continue;
                };
                let gap = vertex[1] - terrain;
                if depth_byte == 0 {
                    // Byte 0 is a clamp: the terrain may sit above the surface under a bank,
                    // but never more than one quantum of real water above it.
                    if gap > 0.0 {
                        shore_above += 1;
                        max_shore_gap = max_shore_gap.max(gap);
                    }
                } else {
                    deep += 1;
                    let residual = (gap - f32::from(depth_byte) * quantum).abs();
                    max_deep_residual = max_deep_residual.max(residual);
                }
            }
        }
    }
    assert!(deep > 500, "only {deep} nonzero depth vertices");
    assert!(
        max_deep_residual <= quantum + 1e-3,
        "depth byte is not 1/9 yd: max residual {max_deep_residual} yd"
    );
    assert!(
        shore_above > 10,
        "only {shore_above} shore vertices above ground"
    );
    assert!(
        max_shore_gap <= quantum + 1e-3,
        "byte 0 still claims {max_shore_gap} yd of water"
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
