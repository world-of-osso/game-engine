use game_engine_core::adt::{self, BlendBatch, SoundEmitter};

fn append_chunk(bytes: &mut Vec<u8>, tag: &[u8; 4], payload: &[u8]) {
    bytes.extend_from_slice(tag);
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(payload);
}

fn authored_root() -> Vec<u8> {
    let mut mcnk = vec![0; 128];
    mcnk[4..8].copy_from_slice(&3u32.to_le_bytes());
    mcnk[8..12].copy_from_slice(&7u32.to_le_bytes());
    mcnk[0x68..0x6c].copy_from_slice(&12.0f32.to_le_bytes());
    mcnk[0x6c..0x70].copy_from_slice(&34.0f32.to_le_bytes());
    mcnk[0x70..0x74].copy_from_slice(&56.0f32.to_le_bytes());
    append_chunk(&mut mcnk, b"TVCM", &[0; 145 * 4]);
    append_chunk(&mut mcnk, b"RNCM", &[0; 145 * 3]);
    let mut lighting = vec![128; 145 * 4];
    lighting[0..4].copy_from_slice(&[32, 64, 128, 255]);
    append_chunk(&mut mcnk, b"VLCM", &lighting);
    let mut sound = Vec::new();
    sound.extend_from_slice(&42u32.to_le_bytes());
    for value in [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0] {
        sound.extend_from_slice(&value.to_le_bytes());
    }
    append_chunk(&mut mcnk, b"MCSE", &sound);
    let mut batch = Vec::new();
    for value in [1u32, 2, 3, 4, 5] {
        batch.extend_from_slice(&value.to_le_bytes());
    }
    append_chunk(&mut mcnk, b"BBCM", &batch);
    let mut disable = [0; 64];
    disable[0] = 1;
    disable[7] = 0x55;
    append_chunk(&mut mcnk, b"DDCM", &disable);
    let mut root = Vec::new();
    append_chunk(&mut root, b"KNCM", &mcnk);
    root
}

#[test]
fn root_retains_authored_chunk_metadata() {
    let root = adt::parse_root(&authored_root()).expect("synthetic ADT");
    assert_eq!(root.chunk_positions, vec![[34.0, 12.0, 56.0]]);
    let chunk = &root.chunks[0];
    assert_eq!(
        chunk.vertex_lighting.as_ref().unwrap()[0],
        [1.0, 0.5, 0.25, 1.0]
    );
    assert_eq!(
        chunk.sound_emitters,
        vec![SoundEmitter {
            sound_entry_id: 42,
            position: [1.0, 2.0, 3.0],
            size_min: [4.0, 5.0, 6.0],
        }]
    );
    assert_eq!(
        chunk.blend_batches,
        vec![BlendBatch {
            mbmh_index: 1,
            index_count: 2,
            index_first: 3,
            vertex_count: 4,
            vertex_first: 5,
        }]
    );
    let disable = chunk.detail_doodad_disable.unwrap();
    assert_eq!((disable[0], disable[7]), (1, 0x55));
}

#[test]
fn tile_entry_uses_authored_tile_coordinates_for_height_grid_and_center() {
    let data = authored_root();
    let plain = adt::parse_root(&data).expect("root ADT");
    let tiled = adt::parse_root_for_tile(&data, 32, 48, None).expect("tile ADT");
    assert_eq!(tiled.chunk_positions, plain.chunk_positions);
    let mesh = adt::chunk_geometry(&tiled.chunks[0], Some((32, 48)));
    assert_eq!(tiled.height_grids[0].origin_x, mesh.positions[0][0]);
    assert_eq!(tiled.height_grids[0].origin_z, mesh.positions[0][2]);
    assert!((tiled.height_grids[0].origin_x + 8766.666).abs() < 0.01);
    assert!((tiled.height_grids[0].origin_z - 100.0).abs() < 0.01);
    assert_ne!(plain.center_surface, tiled.center_surface);
}

#[test]
fn lod_retains_authored_heights_indices_and_liquids() {
    let data = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/terrain/2703_31_36_lod.adt"
    ))
    .expect("cached LOD ADT");
    let lod = adt::parse_lod(&data).expect("LOD ADT");
    assert_eq!(lod.version, 18);
    assert_eq!(lod.heights.len(), 33_152);
    assert!(lod.heights[0].is_finite());
    assert_eq!(lod.levels[0].payload, [0, 5925, 306, 5925]);
    assert_eq!(lod.nodes[0].words16, [0, 0, 5925, 0]);
    assert_eq!(lod.indices.len(), 131_535);
    assert_eq!(lod.indices.iter().copied().max(), Some(33_151));
    assert_eq!(lod.skirt_indices.len(), 127);
    assert_eq!(lod.liquids.len(), 6);
    assert_eq!(lod.liquids[0].indices.len(), 108);
    assert_eq!(lod.liquid_directory.unwrap().raw.len(), 5_652);
}

#[test]
fn tile_entry_rejects_mismatched_texture_companion() {
    let error = adt::parse_root_for_tile(&authored_root(), 32, 48, Some(b"bad"))
        .err()
        .expect("missing texture MCNK");
    assert!(
        error.contains("texture companion has 0 MCNK chunks, root has 1"),
        "{error}"
    );
}

#[test]
fn malformed_lod_reports_missing_and_truncated_chunks() {
    let missing = adt::parse_lod(b"").unwrap_err();
    assert!(missing.contains("MVER"), "{missing}");
    let truncated = adt::parse_lod(b"REVM\x08\x00\x00\x00\x12").unwrap_err();
    assert!(truncated.contains("truncated"), "{truncated}");
}
