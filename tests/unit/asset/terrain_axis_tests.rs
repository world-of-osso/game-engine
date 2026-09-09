use crate::asset::adt::{self, CHUNK_SIZE, UNIT_SIZE, vertex_index};
use bevy::mesh::{Mesh, VertexAttributeValues};
use bevy::prelude::Vec3;

fn append_chunk(out: &mut Vec<u8>, tag: &[u8; 4], payload: &[u8]) {
    out.extend_from_slice(tag);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
}

// Four authored chunks: row has ten times column's slope, with raised centers.
fn asymmetric_tile() -> Vec<u8> {
    let mut data = Vec::new();
    for iy in 0..2u32 {
        for ix in 0..2u32 {
            let mut chunk = vec![0; 128];
            chunk[4..8].copy_from_slice(&ix.to_le_bytes());
            chunk[8..12].copy_from_slice(&iy.to_le_bytes());
            chunk[104..108].copy_from_slice(&(100.0 - iy as f32 * CHUNK_SIZE).to_le_bytes());
            chunk[108..112].copy_from_slice(&(-200.0 - ix as f32 * CHUNK_SIZE).to_le_bytes());
            chunk[112..116].copy_from_slice(&50.0f32.to_le_bytes());
            let mut heights = Vec::new();
            for row in 0..=16 {
                let inner = row % 2 != 0;
                for col in 0..if inner { 8 } else { 9 } {
                    let r = iy as f32 * 8.0 + row as f32 * 0.5;
                    let c = ix as f32 * 8.0 + col as f32 + if inner { 0.5 } else { 0.0 };
                    let h = 10.0 * r + c + if inner { 3.0 } else { 0.0 };
                    heights.extend_from_slice(&h.to_le_bytes());
                }
            }
            append_chunk(&mut chunk, b"TVCM", &heights);
            append_chunk(&mut chunk, b"RNCM", &[0; 435]);
            append_chunk(&mut data, b"KNCM", &chunk);
        }
    }
    data
}

fn positions(mesh: &Mesh) -> &[[f32; 3]] {
    let Some(VertexAttributeValues::Float32x3(p)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else {
        panic!("missing positions")
    };
    p
}

fn assert_position(actual: [f32; 3], expected: [f32; 3]) {
    assert!(
        (Vec3::from(actual) - Vec3::from(expected)).length() < 0.001,
        "{actual:?} != {expected:?}"
    );
}

#[test]
fn terrain_axis_authored_samples_and_borders() {
    let terrain = adt::load_adt(&asymmetric_tile()).unwrap();
    for chunk in &terrain.chunks {
        let p = positions(&chunk.mesh);
        for row in 0..=16 {
            let inner = row % 2 != 0;
            for col in 0..if inner { 8 } else { 9 } {
                let r = chunk.index_y as f32 * 8.0 + row as f32 * 0.5;
                let c = chunk.index_x as f32 * 8.0 + col as f32 + if inner { 0.5 } else { 0.0 };
                assert_position(
                    p[vertex_index(row, col)],
                    [
                        100.0 - r * UNIT_SIZE,
                        50.0 + 10.0 * r + c + if inner { 3.0 } else { 0.0 },
                        200.0 + c * UNIT_SIZE,
                    ],
                );
            }
        }
        for neighbor in &terrain.chunks {
            let q = positions(&neighbor.mesh);
            for n in 0..=8 {
                if neighbor.index_x == chunk.index_x && neighbor.index_y == chunk.index_y + 1 {
                    assert_position(p[vertex_index(16, n)], q[vertex_index(0, n)]);
                }
                if neighbor.index_y == chunk.index_y && neighbor.index_x == chunk.index_x + 1 {
                    assert_position(p[vertex_index(n * 2, 8)], q[vertex_index(n * 2, 0)]);
                }
            }
        }
    }
}

#[test]
fn terrain_axis_water_offsets_match_authored_grid() {
    let layer = adt::WaterLayer {
        liquid_type: 0,
        liquid_object: 3,
        min_height: 7.0,
        max_height: 7.0,
        x_offset: 5,
        y_offset: 2,
        width: 1,
        height: 1,
        exists: [1, 0, 0, 0, 0, 0, 0, 0],
        vertex_heights: vec![7.0; 4],
        vertex_uvs: vec![],
        vertex_depths: vec![],
    };
    let mesh = adt::build_water_mesh([-200.0, 100.0, 0.0], &layer);
    let p = positions(&mesh);
    assert_position(
        p[0],
        [100.0 - 2.0 * UNIT_SIZE, 7.0, 200.0 + 5.0 * UNIT_SIZE],
    );
    let indices: Vec<_> = mesh.indices().unwrap().iter().collect();
    for triangle in indices.chunks_exact(3) {
        let [a, b, c] = [p[triangle[0]], p[triangle[1]], p[triangle[2]]].map(Vec3::from);
        assert!((b - a).cross(c - a).y > 0.0);
    }
}

#[test]
fn terrain_axis_mesh_winding_points_up() {
    let terrain = adt::load_adt(&asymmetric_tile()).unwrap();
    for chunk in &terrain.chunks {
        let p = positions(&chunk.mesh);
        let indices: Vec<_> = chunk.mesh.indices().unwrap().iter().collect();
        for tri in indices.chunks_exact(3) {
            let [a, b, c] = [p[tri[0]], p[tri[1]], p[tri[2]]].map(Vec3::from);
            assert!((b - a).cross(c - a).y > 0.0);
        }
    }
}

#[test]
fn terrain_axis_loaded_heights_preserve_authored_samples() {
    let terrain = adt::load_adt(&asymmetric_tile()).unwrap();
    for grid in &terrain.height_grids {
        for row in 0..=16 {
            let inner = row % 2 != 0;
            for col in 0..if inner { 8 } else { 9 } {
                let r = grid.index_y as f32 * 8.0 + row as f32 * 0.5;
                let c = grid.index_x as f32 * 8.0 + col as f32 + if inner { 0.5 } else { 0.0 };
                let expected = 10.0 * r + c + if inner { 3.0 } else { 0.0 };
                assert_eq!(grid.heights[vertex_index(row, col)], expected);
            }
        }
    }
}
