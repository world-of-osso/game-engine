use super::*;

#[test]
fn parse_momt_empty_data() {
    let result = parse_momt(&[]);
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[test]
fn parse_momt_truncated_entry() {
    // MOMT entries are 64 bytes; provide only 32
    let data = vec![0u8; 32];
    let result = parse_momt(&data);
    // Should either parse 0 entries or error — not panic
    assert!(result.is_ok() || result.is_err());
    if let Ok(entries) = result {
        assert!(entries.is_empty());
    }
}

#[test]
fn parse_mobn_empty() {
    let result = parse_mobn(&[]);
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[test]
fn parse_mobn_truncated() {
    // BSP node entry is 16 bytes; provide 8
    let data = vec![0u8; 8];
    let result = parse_mobn(&data);
    assert!(result.is_ok() || result.is_err());
    if let Ok(nodes) = result {
        assert!(nodes.is_empty());
    }
}

#[test]
fn parse_mliq_empty_data_fails() {
    let result = parse_mliq(&[]);
    assert!(result.is_err());
}

#[test]
fn parse_mliq_truncated_header() {
    // MLIQ header is 30 bytes; provide 10
    let data = vec![0u8; 10];
    let result = parse_mliq(&data);
    assert!(result.is_err());
}

#[test]
fn parse_mliq_negative_dimensions() {
    let mut data = Vec::new();
    data.extend_from_slice(&(-1_i32).to_le_bytes()); // x_verts
    data.extend_from_slice(&(-1_i32).to_le_bytes()); // y_verts
    data.extend_from_slice(&(-1_i32).to_le_bytes()); // x_tiles
    data.extend_from_slice(&(-1_i32).to_le_bytes()); // y_tiles
    data.extend_from_slice(&[0u8; 14]); // position + material_id
    let result = parse_mliq(&data);
    // Should handle negative dimensions without panic
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn parse_mliq_zero_dimensions_ok() {
    let mut data = Vec::new();
    data.extend_from_slice(&0_i32.to_le_bytes()); // x_verts
    data.extend_from_slice(&0_i32.to_le_bytes()); // y_verts
    data.extend_from_slice(&0_i32.to_le_bytes()); // x_tiles
    data.extend_from_slice(&0_i32.to_le_bytes()); // y_tiles
    for v in [0.0_f32, 0.0, 0.0] {
        data.extend_from_slice(&v.to_le_bytes());
    }
    data.extend_from_slice(&0_i16.to_le_bytes()); // material_id
    let result = parse_mliq(&data);
    assert!(result.is_ok(), "zero-dimension MLIQ should parse");
    let liquid = result.unwrap();
    assert!(liquid.vertices.is_empty());
    assert!(liquid.tiles.is_empty());
}

#[test]
fn parse_mobr_empty() {
    let result = parse_mobr(&[]);
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[test]
fn parse_group_subchunks_empty_data() {
    let result = parse_group_subchunks(&[]);
    // Empty group data fails because required chunks (MOVT/MOVI) are missing
    assert!(result.is_err());
}

#[test]
fn load_wmo_root_empty_data() {
    let result = load_wmo_root(&[]);
    // Empty data should produce a valid but empty root
    assert!(result.is_ok());
    let root = result.unwrap();
    assert!(root.materials.is_empty());
    assert_eq!(root.n_groups, 0);
}

#[test]
fn load_wmo_root_truncated_chunk() {
    // Valid chunk tag but payload extends past data
    let mut data = Vec::new();
    data.extend_from_slice(b"TMOM"); // MOMT tag reversed
    data.extend_from_slice(&1000u32.to_le_bytes());
    data.extend_from_slice(&[0u8; 4]); // only 4 bytes of payload
    let result = load_wmo_root(&data);
    assert!(result.is_err());
}
