//! Merged exterior batches draw exactly the triangles and vertex streams of their inputs.
use super::{MergeKey, MergedBatch};
use crate::wmo::scene::RetailWmoStreams;
use game_engine_core::asset::wmo_format::parser::WmoGroupFlags;
use game_engine_core::wmo::{WmoBatchType, WmoGroupHeader, WmoMeshBatch};

fn header(flags: u32, center: [f32; 2]) -> WmoGroupHeader {
    WmoGroupHeader {
        group_name_offset: 0,
        descriptive_group_name_offset: 0,
        flags,
        group_flags: WmoGroupFlags::from_bits(flags),
        bbox_min: [center[0] - 10.0, center[1] - 10.0, 0.0],
        bbox_max: [center[0] + 10.0, center[1] + 10.0, 20.0],
        portal_start: 0,
        portal_count: 0,
        trans_batch_count: 0,
        int_batch_count: 0,
        ext_batch_count: 1,
        batch_type_d: 0,
        fog_ids: [0; 4],
        group_liquid: 0,
        unique_id: 0,
        flags2: 0,
        parent_split_group_index: -1,
        next_split_child_group_index: -1,
    }
}

const EXTERIOR: u32 = 0x8;
const INTERIOR: u32 = 0x2000;

/// A quad of two triangles at height `y`, its vertex colours and second UVs numbered
/// from `first` so each vertex's values are recognisable after merging.
fn quad(y: f32, first: f32) -> (WmoMeshBatch, RetailWmoStreams) {
    let positions = vec![[0.0, y, 0.0], [1.0, y, 0.0], [1.0, y, 1.0], [0.0, y, 1.0]];
    let mesh = WmoMeshBatch {
        normals: vec![[0.0, 1.0, 0.0]; 4],
        uvs: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        second_uvs: None,
        third_uvs: None,
        second_color_blend_alphas: None,
        colors: Some((0..4).map(|i| [first + i as f32, 0.0, 0.0, 1.0]).collect()),
        indices: vec![0, 1, 2, 0, 2, 3],
        material_index: 7,
        batch_type: WmoBatchType::Exterior,
        uses_second_color_blend_alpha: false,
        uses_second_uv_set: true,
        uses_third_uv_set: false,
        uses_generated_tangents: false,
        has_vertex_color: true,
        positions,
    };
    let streams = RetailWmoStreams {
        uv2: Some((0..4).map(|i| [first + i as f32, 0.5]).collect()),
        ..Default::default()
    };
    (mesh, streams)
}

/// Each triangle as its corners' (position, colour red, second U).
fn corners(mesh: &WmoMeshBatch, streams: &RetailWmoStreams) -> Vec<[([f32; 3], f32, f32); 3]> {
    let colors = mesh.colors.as_ref().unwrap();
    let uv2 = streams.uv2.as_ref().unwrap();
    let corner = |i: u32| {
        let i = i as usize;
        (mesh.positions[i], colors[i][0], uv2[i][0])
    };
    mesh.indices
        .chunks_exact(3)
        .map(|t| [corner(t[0]), corner(t[1]), corner(t[2])])
        .collect()
}

#[test]
fn merged_batch_draws_each_input_triangle_with_its_own_vertex_streams() {
    let (first, first_streams) = quad(0.0, 10.0);
    let (second, second_streams) = quad(5.0, 20.0);
    let mut merged = MergedBatch::new(first.clone(), first_streams, 3);
    merged.append(&second, &second_streams, 9);
    merged.append(&second, &second_streams, 9);
    let mut expected = corners(&first, &quad(0.0, 10.0).1);
    for _ in 0..2 {
        expected.extend(corners(&second, &second_streams));
    }
    assert_eq!(corners(&merged.mesh, &merged.streams), expected);
    assert_eq!(merged.mesh.positions.len(), 12);
    assert_eq!(merged.groups, [3, 9]);
}

#[test]
fn exterior_opaque_batches_of_one_material_share_a_key_within_a_cell_only() {
    let (mesh, streams) = quad(0.0, 0.0);
    let key =
        |flags, center, blend| MergeKey::of(&header(flags, center), true, blend, &mesh, &streams);
    let here = key(EXTERIOR, [10.0, 10.0], 0);
    assert!(here.is_some());
    assert_eq!(
        key(EXTERIOR, [200.0, 30.0], 1),
        here,
        "same 256-yard cell, alpha-tested"
    );
    assert_ne!(key(EXTERIOR, [300.0, 10.0], 0), here, "next cell");
    assert_eq!(
        key(EXTERIOR, [10.0, 10.0], 2),
        None,
        "blended batches keep their draw order"
    );
    assert_eq!(
        key(INTERIOR, [10.0, 10.0], 0),
        None,
        "interiors keep exact portal culling"
    );
    assert_eq!(
        MergeKey::of(&header(EXTERIOR, [10.0, 10.0]), false, 0, &mesh, &streams),
        None,
        "groups the portal graph never draws"
    );
}

#[test]
fn a_batch_whose_streams_do_not_match_its_vertices_keeps_its_own_node() {
    let (mesh, mut streams) = quad(0.0, 0.0);
    streams.uv2.as_mut().unwrap().pop();
    assert_eq!(
        MergeKey::of(&header(EXTERIOR, [0.0, 0.0]), true, 0, &mesh, &streams),
        None
    );
}
