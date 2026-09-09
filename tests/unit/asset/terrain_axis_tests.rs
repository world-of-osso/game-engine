use crate::asset::adt::{self, CHUNK_SIZE, UNIT_SIZE, vertex_index};
use bevy::mesh::{Mesh, VertexAttributeValues};
use bevy::prelude::Vec3;

fn append_chunk(out: &mut Vec<u8>, tag: &[u8; 4], payload: &[u8]) {
    out.extend_from_slice(tag);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
}

fn authored_samples() -> impl Iterator<Item = (usize, usize)> {
    (0..=16).flat_map(|row| {
        let columns = if row % 2 == 0 { 9 } else { 8 };
        (0..columns).map(move |col| (row, col))
    })
}

fn authored_coordinates(ix: u32, iy: u32, row: usize, col: usize) -> (f32, f32) {
    let inner_offset = if row % 2 == 0 { 0.0 } else { 0.5 };
    let r = iy as f32 * 8.0 + row as f32 * 0.5;
    let c = ix as f32 * 8.0 + col as f32 + inner_offset;
    (r, c)
}

fn authored_height(ix: u32, iy: u32, row: usize, col: usize) -> f32 {
    let (r, c) = authored_coordinates(ix, iy, row, col);
    let center_raise = if row % 2 == 0 { 0.0 } else { 3.0 };
    10.0 * r + c + center_raise
}

fn encode_authored_chunk(ix: u32, iy: u32) -> Vec<u8> {
    let mut chunk = vec![0; 128];
    chunk[4..8].copy_from_slice(&ix.to_le_bytes());
    chunk[8..12].copy_from_slice(&iy.to_le_bytes());
    chunk[104..108].copy_from_slice(&(100.0 - iy as f32 * CHUNK_SIZE).to_le_bytes());
    chunk[108..112].copy_from_slice(&(-200.0 - ix as f32 * CHUNK_SIZE).to_le_bytes());
    chunk[112..116].copy_from_slice(&50.0f32.to_le_bytes());
    let heights: Vec<_> = authored_samples()
        .flat_map(|(row, col)| authored_height(ix, iy, row, col).to_le_bytes())
        .collect();
    append_chunk(&mut chunk, b"TVCM", &heights);
    append_chunk(&mut chunk, b"RNCM", &[0; 435]);
    chunk
}

// Four authored chunks: row has ten times column's slope, with raised centers.
fn asymmetric_tile() -> Vec<u8> {
    let mut data = Vec::new();
    for (ix, iy) in (0..2).flat_map(|iy| (0..2).map(move |ix| (ix, iy))) {
        append_chunk(&mut data, b"KNCM", &encode_authored_chunk(ix, iy));
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

fn assert_authored_positions(chunk: &adt::McnkMesh) {
    let p = positions(&chunk.mesh);
    for (row, col) in authored_samples() {
        let (r, c) = authored_coordinates(chunk.index_x, chunk.index_y, row, col);
        let height = authored_height(chunk.index_x, chunk.index_y, row, col);
        assert_position(
            p[vertex_index(row, col)],
            [100.0 - r * UNIT_SIZE, 50.0 + height, 200.0 + c * UNIT_SIZE],
        );
    }
}

fn assert_shared_border(chunk: &adt::McnkMesh, neighbor: &adt::McnkMesh) {
    let p = positions(&chunk.mesh);
    let q = positions(&neighbor.mesh);
    if neighbor.index_x == chunk.index_x && neighbor.index_y == chunk.index_y + 1 {
        for n in 0..=8 {
            assert_position(p[vertex_index(16, n)], q[vertex_index(0, n)]);
        }
    }
    if neighbor.index_y == chunk.index_y && neighbor.index_x == chunk.index_x + 1 {
        for n in 0..=8 {
            assert_position(p[vertex_index(n * 2, 8)], q[vertex_index(n * 2, 0)]);
        }
    }
}

#[test]
fn terrain_axis_authored_samples_and_borders() {
    let terrain = adt::load_adt(&asymmetric_tile()).unwrap();
    for chunk in &terrain.chunks {
        assert_authored_positions(chunk);
        for neighbor in &terrain.chunks {
            assert_shared_border(chunk, neighbor);
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
        for (row, col) in authored_samples() {
            let expected = authored_height(grid.index_x, grid.index_y, row, col);
            assert_eq!(grid.heights[vertex_index(row, col)], expected);
        }
    }
}
