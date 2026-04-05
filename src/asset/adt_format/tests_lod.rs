use super::{LodHeader, LodLevel, LodQuadTreeNode, append_subchunk, load_lod_adt};
use crate::asset::adt_format::adt::LodLiquidDirectory;

const TEST_M2_LOD_FLAGS: u32 = 0x55AA;
const TEST_WMO_LOD_FLAGS: u32 = 0xAA55;

#[test]
fn load_lod_adt_reads_synthetic_lod_chunks() {
    let payload = synthetic_lod_payload();

    let lod = load_lod_adt(&payload).expect("expected synthetic _lod.adt to parse");

    assert_eq!(lod.version, 18);
    assert_eq!(
        lod.header,
        LodHeader {
            flags: 7,
            bounds_min: [10.0, 30.0, 50.0],
            bounds_max: [20.0, 40.0, 60.0],
        }
    );
    assert_eq!(lod.heights, vec![1.5, 2.5, 3.5, 4.5]);
    assert_eq!(
        lod.levels,
        vec![
            LodLevel {
                vertex_step: 32.0,
                payload: [0, 12, 4, 24],
            },
            LodLevel {
                vertex_step: 16.0,
                payload: [12, 24, 8, 48],
            },
        ]
    );
    assert_eq!(
        lod.nodes,
        vec![
            LodQuadTreeNode {
                words16: [1, 2, 3, 4],
                words32: [5, 6, 7],
            },
            LodQuadTreeNode {
                words16: [8, 9, 10, 11],
                words32: [12, 13, 14],
            },
        ]
    );
    assert_eq!(lod.indices, vec![0, 1, 2, 3, 4, 5]);
    assert_eq!(lod.skirt_indices, vec![6, 7, 8, 9]);
    assert_eq!(
        lod.liquid_directory,
        Some(LodLiquidDirectory {
            raw: vec![0xAA, 0xBB, 0xCC, 0xDD],
        })
    );
    assert_eq!(lod.liquids.len(), 1);
    assert_eq!(lod.liquids[0].header.words, [1, 2, 3, 4, 5, 6]);
    assert_eq!(lod.liquids[0].indices, vec![10, 11, 12]);
    assert_eq!(
        lod.liquids[0].vertices,
        vec![[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]
    );
    assert_eq!(lod.m2_placements.len(), 1);
    assert_eq!(lod.m2_placements[0].id, 100);
    assert_eq!(lod.m2_placements[0].asset_id, 200);
    assert_eq!(lod.m2_placements[0].position, [1.0, 2.0, 3.0]);
    assert_eq!(lod.m2_placements[0].rotation, [4.0, 5.0, 6.0]);
    assert_eq!(lod.m2_placements[0].scale, 1.5);
    assert_eq!(lod.m2_placements[0].flags, TEST_M2_LOD_FLAGS);
    assert_eq!(lod.m2_visibility.len(), 1);
    assert_eq!(lod.m2_visibility[0].bounds_min, [10.0, 20.0, 30.0]);
    assert_eq!(lod.m2_visibility[0].bounds_max, [40.0, 50.0, 60.0]);
    assert_eq!(lod.m2_visibility[0].radius, 70.0);
    assert_eq!(lod.wmo_placements.len(), 1);
    assert_eq!(lod.wmo_visibility.len(), 1);
}

#[test]
fn load_lod_adt_reads_real_tile_counts_and_ranges() {
    let data = std::fs::read("data/terrain/2703_31_36_lod.adt").expect("missing test asset");

    let lod = load_lod_adt(&data).expect("expected real _lod.adt to parse");

    assert_eq!(lod.version, 18);
    assert_eq!(lod.header.flags, 0);
    assert_eq!(lod.heights.len(), 33_152);
    assert_eq!(lod.levels.len(), 4);
    assert_eq!(lod.nodes.len(), 102);
    assert_eq!(lod.indices.len(), 131_535);
    assert_eq!(lod.skirt_indices.len(), 127);
    assert_eq!(lod.liquids.len(), 6);
    assert!(lod.m2_placements.is_empty());
    assert!(lod.m2_visibility.is_empty());
    assert!(lod.wmo_placements.is_empty());
    assert!(lod.wmo_visibility.is_empty());
    assert_eq!(
        lod.liquid_directory
            .as_ref()
            .map(|directory| directory.raw.len()),
        Some(5_652)
    );
    assert_eq!(lod.indices.iter().copied().max(), Some(33_151));
    assert_eq!(lod.skirt_indices.iter().copied().max(), Some(16_640));
    assert_eq!(
        lod.levels
            .iter()
            .map(|level| level.vertex_step)
            .collect::<Vec<_>>(),
        vec![32.0, 16.0, 8.0, 4.0]
    );
    assert_eq!(lod.levels[0].payload, [0, 5925, 306, 5925]);
    assert_eq!(lod.nodes[0].words16, [0, 0, 5925, 0]);
    assert_eq!(lod.nodes[0].words32, [0, 0, 131_073]);
    assert_eq!(lod.nodes[1].words16, [3, 4, 6231, 0]);
    assert_eq!(lod.nodes[1].words32, [2520, 0, 0]);
    assert_eq!(
        lod.liquids[0].header.words,
        [0, 108, 1, 860_749_829, u32::MAX, u32::MAX]
    );
    assert_eq!(lod.liquids[0].indices.len(), 108);
    assert_eq!(lod.liquids[0].vertices.len(), 43);
    assert_eq!(lod.liquids[1].indices.len(), 336);
    assert_eq!(lod.liquids[1].vertices.len(), 127);
}

fn synthetic_lod_payload() -> Vec<u8> {
    let mut payload = Vec::new();
    append_lod_core_chunks(&mut payload);
    append_lod_liquid_chunks(&mut payload);
    append_lod_object_chunks(&mut payload);
    payload
}

fn append_lod_core_chunks(payload: &mut Vec<u8>) {
    append_subchunk(payload, b"REVM", 18u32.to_le_bytes().to_vec());
    append_subchunk(
        payload,
        b"DHLM",
        lod_header_payload(7, [10.0, 30.0, 50.0], [20.0, 40.0, 60.0]),
    );
    append_subchunk(payload, b"HVLM", lod_height_payload());
    append_subchunk(payload, b"LLLM", lod_levels_payload());
    append_subchunk(payload, b"DNLM", lod_nodes_payload());
    append_subchunk(payload, b"IVLM", lod_index_payload(&[0, 1, 2, 3, 4, 5]));
    append_subchunk(payload, b"ISLM", lod_index_payload(&[6, 7, 8, 9]));
}

fn append_lod_liquid_chunks(payload: &mut Vec<u8>) {
    append_subchunk(payload, b"DLLM", vec![0xAA, 0xBB, 0xCC, 0xDD]);
    append_subchunk(
        payload,
        b"NLLM",
        lod_liquid_header_payload([1, 2, 3, 4, 5, 6]),
    );
    append_subchunk(payload, b"ILLM", lod_index_payload(&[10, 11, 12]));
    append_subchunk(
        payload,
        b"VLLM",
        lod_liquid_vertices_payload(&[[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]),
    );
}

fn append_lod_object_chunks(payload: &mut Vec<u8>) {
    append_subchunk(
        payload,
        b"DDLM",
        lod_object_placement_payload(
            100,
            200,
            [1.0, 2.0, 3.0],
            [4.0, 5.0, 6.0],
            1.5,
            TEST_M2_LOD_FLAGS,
        ),
    );
    append_subchunk(
        payload,
        b"XDLM",
        lod_object_visibility_payload([10.0, 20.0, 30.0], [40.0, 50.0, 60.0], 70.0),
    );
    append_subchunk(
        payload,
        b"DMLM",
        lod_object_placement_payload(
            300,
            400,
            [7.0, 8.0, 9.0],
            [10.0, 11.0, 12.0],
            2.5,
            TEST_WMO_LOD_FLAGS,
        ),
    );
    append_subchunk(
        payload,
        b"XMLM",
        lod_object_visibility_payload([15.0, 25.0, 35.0], [45.0, 55.0, 65.0], 75.0),
    );
}

fn lod_height_payload() -> Vec<u8> {
    [1.5f32, 2.5, 3.5, 4.5]
        .into_iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

fn lod_header_payload(flags: u32, bounds_min: [f32; 3], bounds_max: [f32; 3]) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&flags.to_le_bytes());
    payload.extend_from_slice(&bounds_min[0].to_le_bytes());
    payload.extend_from_slice(&bounds_max[0].to_le_bytes());
    payload.extend_from_slice(&bounds_min[1].to_le_bytes());
    payload.extend_from_slice(&bounds_max[1].to_le_bytes());
    payload.extend_from_slice(&bounds_min[2].to_le_bytes());
    payload.extend_from_slice(&bounds_max[2].to_le_bytes());
    payload
}

fn lod_levels_payload() -> Vec<u8> {
    let mut payload = Vec::new();
    append_lod_level(&mut payload, 32.0, [0, 12, 4, 24]);
    append_lod_level(&mut payload, 16.0, [12, 24, 8, 48]);
    payload
}

fn append_lod_level(payload: &mut Vec<u8>, vertex_step: f32, values: [u32; 4]) {
    payload.extend_from_slice(&vertex_step.to_le_bytes());
    for value in values {
        payload.extend_from_slice(&value.to_le_bytes());
    }
}

fn lod_nodes_payload() -> Vec<u8> {
    let mut payload = Vec::new();
    append_lod_node(&mut payload, [1, 2, 3, 4], [5, 6, 7]);
    append_lod_node(&mut payload, [8, 9, 10, 11], [12, 13, 14]);
    payload
}

fn append_lod_node(payload: &mut Vec<u8>, words16: [u16; 4], words32: [u32; 3]) {
    for value in words16 {
        payload.extend_from_slice(&value.to_le_bytes());
    }
    for value in words32 {
        payload.extend_from_slice(&value.to_le_bytes());
    }
}

fn lod_index_payload(indices: &[u16]) -> Vec<u8> {
    indices
        .iter()
        .flat_map(|index| index.to_le_bytes())
        .collect()
}

fn lod_liquid_header_payload(words: [u32; 6]) -> Vec<u8> {
    words
        .into_iter()
        .flat_map(|word| word.to_le_bytes())
        .collect()
}

fn lod_liquid_vertices_payload(vertices: &[[f32; 3]]) -> Vec<u8> {
    let mut payload = Vec::new();
    for vertex in vertices {
        for component in vertex {
            payload.extend_from_slice(&component.to_le_bytes());
        }
    }
    payload
}

fn lod_object_placement_payload(
    id: u32,
    asset_id: u32,
    position: [f32; 3],
    rotation: [f32; 3],
    scale: f32,
    flags: u32,
) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&id.to_le_bytes());
    payload.extend_from_slice(&asset_id.to_le_bytes());
    for component in position {
        payload.extend_from_slice(&component.to_le_bytes());
    }
    for component in rotation {
        payload.extend_from_slice(&component.to_le_bytes());
    }
    payload.extend_from_slice(&scale.to_le_bytes());
    payload.extend_from_slice(&flags.to_le_bytes());
    payload
}

fn lod_object_visibility_payload(
    bounds_min: [f32; 3],
    bounds_max: [f32; 3],
    radius: f32,
) -> Vec<u8> {
    let mut payload = Vec::new();
    for component in bounds_min {
        payload.extend_from_slice(&component.to_le_bytes());
    }
    for component in bounds_max {
        payload.extend_from_slice(&component.to_le_bytes());
    }
    payload.extend_from_slice(&radius.to_le_bytes());
    payload
}
