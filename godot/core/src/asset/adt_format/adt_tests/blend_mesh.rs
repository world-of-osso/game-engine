use super::fixtures::adt_file_payload;
use super::{BlendMeshBounds, BlendMeshHeader, BlendMeshVertex, FlightBounds, load_adt_raw};

#[test]
fn load_adt_raw_reads_top_level_blend_mesh_chunks() {
    let data = adt_file_payload(true);

    let parsed = load_adt_raw(&data).expect("expected ADT with blend mesh to parse");
    let blend_mesh = parsed.blend_mesh.expect("expected blend mesh data");

    assert_eq!(
        blend_mesh.headers,
        vec![BlendMeshHeader {
            map_object_id: 77,
            texture_id: 5,
            unknown: 0,
            index_count: 3,
            vertex_count: 3,
            index_start: 0,
            vertex_start: 0,
        }]
    );
    assert_eq!(
        blend_mesh.bounds,
        vec![BlendMeshBounds {
            map_object_id: 77,
            min: [1.0, 2.0, 3.0],
            max: [4.0, 5.0, 6.0],
        }]
    );
    assert_eq!(blend_mesh.vertices.len(), 3);
    assert_eq!(
        blend_mesh.vertices[0],
        BlendMeshVertex {
            position: [10.0, 20.0, 30.0],
            normal: [0.0, 1.0, 0.0],
            uv: [0.25, 0.75],
            color: [[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]],
        }
    );
    assert_eq!(blend_mesh.indices, vec![0, 1, 2]);
    assert_eq!(parsed.chunks[0].blend_batches.len(), 2);
}

#[test]
fn load_adt_raw_reads_top_level_mfbo_flight_bounds() {
    let data = adt_file_payload(false);

    let parsed = load_adt_raw(&data).expect("expected ADT with MFBO to parse");

    assert_eq!(
        parsed.flight_bounds,
        Some(FlightBounds {
            min_heights: [-10, -9, -8, -7, -6, -5, -4, -3, -2],
            max_heights: [20, 21, 22, 23, 24, 25, 26, 27, 28],
        })
    );
}

#[test]
fn load_adt_raw_rejects_partial_top_level_blend_mesh_data() {
    let mut data = adt_file_payload(false);
    super::fixtures::append_subchunk(&mut data, b"HMBM", {
        let mut payload = Vec::new();
        payload.extend_from_slice(&77u32.to_le_bytes());
        payload.extend_from_slice(&5u32.to_le_bytes());
        payload.extend_from_slice(&0u32.to_le_bytes());
        payload.extend_from_slice(&3u32.to_le_bytes());
        payload.extend_from_slice(&3u32.to_le_bytes());
        payload.extend_from_slice(&0u32.to_le_bytes());
        payload.extend_from_slice(&0u32.to_le_bytes());
        payload
    });

    let err = match load_adt_raw(&data) {
        Ok(_) => panic!("expected incomplete blend mesh data to fail"),
        Err(err) => err,
    };
    assert!(err.contains("missing VNBM"));
}

#[test]
fn blend_batch_fields_parsed_correctly() {
    let data = adt_file_payload(true);
    let parsed = load_adt_raw(&data).expect("should parse");
    let batches = &parsed.chunks[0].blend_batches;
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0].mbmh_index, 1);
    assert_eq!(batches[0].index_count, 3);
    assert_eq!(batches[0].index_first, 5);
    assert_eq!(batches[0].vertex_count, 4);
    assert_eq!(batches[0].vertex_first, 6);
}

#[test]
fn blend_mesh_vertex_has_all_components() {
    let data = adt_file_payload(true);
    let parsed = load_adt_raw(&data).expect("should parse");
    let blend_mesh = parsed.blend_mesh.expect("should have blend mesh");

    for (i, v) in blend_mesh.vertices.iter().enumerate() {
        assert!(
            v.position.iter().any(|&c| c != 0.0),
            "vertex {i} has zero position"
        );
        let len = (v.normal[0].powi(2) + v.normal[1].powi(2) + v.normal[2].powi(2)).sqrt();
        assert!(
            (len - 1.0).abs() < 0.01,
            "vertex {i} normal length {len} not unit"
        );
        assert!(
            v.uv[0] >= 0.0 && v.uv[0] <= 1.0,
            "vertex {i} u out of range"
        );
        assert!(
            v.uv[1] >= 0.0 && v.uv[1] <= 1.0,
            "vertex {i} v out of range"
        );
    }
}

#[test]
fn blend_mesh_indices_reference_valid_vertices() {
    let data = adt_file_payload(true);
    let parsed = load_adt_raw(&data).expect("should parse");
    let blend_mesh = parsed.blend_mesh.expect("should have blend mesh");

    for (i, &idx) in blend_mesh.indices.iter().enumerate() {
        assert!(
            (idx as usize) < blend_mesh.vertices.len(),
            "index {i} value {idx} >= vertex count {}",
            blend_mesh.vertices.len()
        );
    }
}

#[test]
fn blend_mesh_header_consistency() {
    let data = adt_file_payload(true);
    let parsed = load_adt_raw(&data).expect("should parse");
    let blend_mesh = parsed.blend_mesh.expect("should have blend mesh");

    for (i, header) in blend_mesh.headers.iter().enumerate() {
        let vertex_end = header.vertex_start + header.vertex_count;
        assert!(
            vertex_end as usize <= blend_mesh.vertices.len(),
            "header {i} vertex range exceeds data"
        );
        let index_end = header.index_start + header.index_count;
        assert!(
            index_end as usize <= blend_mesh.indices.len(),
            "header {i} index range exceeds data"
        );
    }
}

#[test]
fn load_adt_raw_without_blend_mesh() {
    let data = adt_file_payload(false);
    let parsed = load_adt_raw(&data).expect("should parse");
    assert!(parsed.blend_mesh.is_none());
    assert!(!parsed.chunks[0].blend_batches.is_empty());
}
