//! Merged water blocks draw exactly the triangles of their MH2O layers.
use super::{append_geometry, empty_geometry, water_block};
use game_engine_core::adt::{WaterGeometry, WaterLayer, build_water_geometry};

fn layer(min_height: f32) -> WaterLayer {
    WaterLayer {
        liquid_type: 5,
        liquid_object: 0,
        min_height,
        max_height: min_height + 1.0,
        x_offset: 1,
        y_offset: 2,
        width: 2,
        height: 1,
        exists: [0b11, 0, 0, 0, 0, 0, 0, 0],
        vertex_heights: vec![
            min_height,
            min_height + 0.5,
            min_height + 1.0,
            0.0,
            0.25,
            0.5,
        ],
        vertex_uvs: vec![[0.1, 0.2]; 6],
        vertex_depths: vec![0, 40, 80, 120, 160, 200],
        object_vertex_bytes: Vec::new(),
    }
}

/// Each triangle as its three (position, uv, depth) corners in winding order.
fn triangles(geometry: &WaterGeometry) -> Vec<[([f32; 3], [f32; 2], f32); 3]> {
    geometry
        .indices
        .chunks_exact(3)
        .map(|triangle| {
            <[u32; 3]>::try_from(triangle).unwrap().map(|index| {
                let index = index as usize;
                (
                    geometry.positions[index],
                    geometry.uvs[index],
                    geometry.colors[index][3],
                )
            })
        })
        .collect()
}

#[test]
fn blocks_group_four_by_four_chunks_of_the_row_major_grid() {
    assert_eq!(water_block(0), 0);
    assert_eq!(water_block(3), 0);
    assert_eq!(water_block(3 * 16 + 3), 0);
    assert_eq!(water_block(4), 1);
    assert_eq!(water_block(15), 3);
    assert_eq!(water_block(4 * 16), 4);
    assert_eq!(water_block(7 * 16 + 9), 6);
    assert_eq!(water_block(255), 15);
}

#[test]
fn merged_block_keeps_every_layer_triangle_with_its_vertex_data() {
    let first = build_water_geometry([100.0, 20.0, 50.0], &layer(20.0));
    let second = build_water_geometry([66.67, 21.0, 50.0], &layer(21.0));
    assert_eq!(first.indices.len(), 12);
    let expected: Vec<_> = triangles(&first)
        .into_iter()
        .chain(triangles(&second))
        .collect();
    let vertices = first.positions.len() + second.positions.len();
    let mut merged = empty_geometry();
    append_geometry(&mut merged, first);
    append_geometry(&mut merged, second);
    assert_eq!(merged.positions.len(), vertices);
    assert_eq!(merged.normals.len(), vertices);
    assert_eq!(triangles(&merged), expected);
}
