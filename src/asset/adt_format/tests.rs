use super::{
    BlendBatch, BlendMeshBounds, BlendMeshHeader, BlendMeshVertex, FlightBounds, LodHeader,
    LodLevel, LodQuadTreeNode, MCNK_FLAG_DO_NOT_FIX_ALPHA_MAP, MCNK_FLAG_HAS_MCCV,
    MCNK_FLAG_HAS_MCSH, MCNK_FLAG_HIGH_RES_HOLES, MCNK_FLAG_IMPASS, MCVT_COUNT, McnkFlags,
    load_adt_raw, load_lod_adt, parse_mccv, parse_mclv, parse_mcnk, parse_mcnk_subchunks,
};

const TEST_AREA_ID: u32 = 0x1234_5678;

#[test]
fn parse_mccv_reads_bgra_and_maps_neutral_to_one() {
    let mut payload = vec![0x7F; MCVT_COUNT * 4];
    for i in 0..MCVT_COUNT {
        payload[i * 4 + 3] = 0xFF;
    }
    payload[0..4].copy_from_slice(&[0x20, 0x40, 0x60, 0x80]);

    let colors = parse_mccv(&payload).expect("expected MCCV colors");
    assert_eq!(colors.len(), MCVT_COUNT);
    assert_eq!(colors[1], [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(colors[0][0], 0x60 as f32 / 127.0);
    assert_eq!(colors[0][1], 0x40 as f32 / 127.0);
    assert_eq!(colors[0][2], 0x20 as f32 / 127.0);
    assert_eq!(colors[0][3], 0x80 as f32 / 255.0);
}

#[test]
fn mcnk_flags_decode_named_bits() {
    let flags = McnkFlags::from_bits(
        MCNK_FLAG_HAS_MCSH
            | MCNK_FLAG_IMPASS
            | MCNK_FLAG_HAS_MCCV
            | MCNK_FLAG_DO_NOT_FIX_ALPHA_MAP
            | MCNK_FLAG_HIGH_RES_HOLES,
    );

    assert!(flags.has_mcsh);
    assert!(flags.impass);
    assert!(flags.has_mccv);
    assert!(flags.do_not_fix_alpha_map);
    assert!(flags.high_res_holes);
}

#[test]
fn parse_mcnk_subchunks_requires_mccv_when_flagged() {
    let payload = mcnk_subchunks_payload(false, false, false, false, false);

    let err = parse_mcnk_subchunks(
        &payload,
        McnkFlags {
            has_mcsh: false,
            impass: false,
            has_mccv: true,
            do_not_fix_alpha_map: false,
            high_res_holes: false,
        },
    )
    .expect_err("expected missing MCCV to be rejected");

    assert!(err.contains("flagged with MCCV"));
}

#[test]
fn parse_mcnk_subchunks_defaults_vertex_colors_when_mccv_not_flagged() {
    let payload = mcnk_subchunks_payload(false, false, false, false, false);

    let (
        _,
        _,
        colors,
        vertex_lighting,
        shadow_map,
        sound_emitters,
        blend_batches,
        detail_doodad_disable,
    ) = parse_mcnk_subchunks(&payload, McnkFlags::default())
        .expect("expected missing optional MCCV to default");

    assert_eq!(colors[0], [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(colors[MCVT_COUNT - 1], [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(vertex_lighting, None);
    assert_eq!(shadow_map, None);
    assert!(sound_emitters.is_empty());
    assert!(blend_batches.is_empty());
    assert_eq!(detail_doodad_disable, None);
}

#[test]
fn parse_mcnk_reads_area_id_from_header() {
    let mut payload = vec![0; 128];
    payload[0..4].copy_from_slice(&0u32.to_le_bytes());
    payload[4..8].copy_from_slice(&3u32.to_le_bytes());
    payload[8..12].copy_from_slice(&7u32.to_le_bytes());
    payload[60..64].copy_from_slice(&TEST_AREA_ID.to_le_bytes());
    append_subchunk(&mut payload, b"TVCM", vec![0; MCVT_COUNT * 4]);
    append_subchunk(&mut payload, b"RNCM", vec![0; MCVT_COUNT * 3]);

    let chunk = parse_mcnk(&payload).expect("expected MCNK header to parse");
    assert_eq!(chunk.area_id, TEST_AREA_ID);
}

#[test]
fn parse_mclv_reads_bgra_and_uses_neutral_center() {
    let mut payload = vec![0; MCVT_COUNT * 4];
    payload[0..4].copy_from_slice(&[0x40, 0x60, 0x80, 0xFF]);

    let colors = parse_mclv(&payload).expect("expected MCLV colors");
    assert_eq!(colors.len(), MCVT_COUNT);
    assert_eq!(colors[0], [128.0 / 128.0, 96.0 / 128.0, 64.0 / 128.0, 1.0]);
    assert_eq!(colors[1], [0.0, 0.0, 0.0, 1.0]);
}

#[test]
fn parse_mcnk_subchunks_reads_mcsh_when_flagged() {
    let payload = mcnk_subchunks_payload(true, false, false, false, false);

    let (
        _,
        _,
        _,
        vertex_lighting,
        shadow_map,
        sound_emitters,
        blend_batches,
        detail_doodad_disable,
    ) = parse_mcnk_subchunks(
        &payload,
        McnkFlags {
            has_mcsh: true,
            impass: false,
            has_mccv: false,
            do_not_fix_alpha_map: false,
            high_res_holes: false,
        },
    )
    .expect("expected MCSH shadow map");

    let shadow_map = shadow_map.expect("expected parsed shadow map");
    assert_eq!(shadow_map[0], 0b1000_0000);
    assert_eq!(shadow_map[1], 0b0100_0000);
    assert_eq!(vertex_lighting, None);
    assert!(sound_emitters.is_empty());
    assert!(blend_batches.is_empty());
    assert_eq!(detail_doodad_disable, None);
}

#[test]
fn parse_mcnk_subchunks_fixes_shadow_map_edges_by_default() {
    let payload = mcnk_subchunks_payload(true, false, false, false, false);

    let (_, _, _, _, shadow_map, _, _, _) = parse_mcnk_subchunks(
        &payload,
        McnkFlags {
            has_mcsh: true,
            impass: false,
            has_mccv: false,
            do_not_fix_alpha_map: false,
            high_res_holes: false,
        },
    )
    .expect("expected MCSH shadow map");

    let shadow_map = shadow_map.expect("expected parsed shadow map");
    assert!(!shadow_map_bit(&shadow_map, 0, 62));
    assert!(!shadow_map_bit(&shadow_map, 0, 63));
    assert!(!shadow_map_bit(&shadow_map, 63, 0));
    assert!(shadow_map_bit(&shadow_map, 63, 1));
    assert!(!shadow_map_bit(&shadow_map, 63, 63));
}

#[test]
fn parse_mcnk_subchunks_preserves_shadow_map_edges_when_flagged() {
    let payload = mcnk_subchunks_payload(true, false, false, false, false);

    let (_, _, _, _, shadow_map, _, _, _) = parse_mcnk_subchunks(
        &payload,
        McnkFlags {
            has_mcsh: true,
            impass: false,
            has_mccv: false,
            do_not_fix_alpha_map: true,
            high_res_holes: false,
        },
    )
    .expect("expected MCSH shadow map");

    let shadow_map = shadow_map.expect("expected parsed shadow map");
    assert!(!shadow_map_bit(&shadow_map, 0, 62));
    assert!(shadow_map_bit(&shadow_map, 0, 63));
    assert!(shadow_map_bit(&shadow_map, 63, 0));
    assert!(!shadow_map_bit(&shadow_map, 63, 1));
    assert!(!shadow_map_bit(&shadow_map, 63, 63));
}

#[test]
fn parse_mcnk_subchunks_reads_mcse_emitters() {
    let payload = mcnk_subchunks_payload(false, false, true, false, false);

    let (
        _,
        _,
        _,
        vertex_lighting,
        shadow_map,
        sound_emitters,
        blend_batches,
        detail_doodad_disable,
    ) = parse_mcnk_subchunks(&payload, McnkFlags::default()).expect("expected MCSE emitters");

    assert_eq!(vertex_lighting, None);
    assert_eq!(shadow_map, None);
    assert_eq!(sound_emitters.len(), 2);
    assert_eq!(sound_emitters[0].sound_entry_id, 42);
    assert_eq!(sound_emitters[0].position, [100.0, 200.0, 300.0]);
    assert_eq!(sound_emitters[0].size_min, [10.0, 20.0, 30.0]);
    assert_eq!(sound_emitters[1].sound_entry_id, 7);
    assert_eq!(sound_emitters[1].position, [1.0, 2.0, 3.0]);
    assert_eq!(sound_emitters[1].size_min, [4.0, 5.0, 6.0]);
    assert!(blend_batches.is_empty());
    assert_eq!(detail_doodad_disable, None);
}

#[test]
fn parse_mcnk_subchunks_reads_mcbb_batches() {
    let payload = mcnk_subchunks_payload(false, false, false, true, false);

    let (
        _,
        _,
        _,
        vertex_lighting,
        shadow_map,
        sound_emitters,
        blend_batches,
        detail_doodad_disable,
    ) = parse_mcnk_subchunks(&payload, McnkFlags::default()).expect("expected MCBB batches");

    assert_eq!(vertex_lighting, None);
    assert_eq!(shadow_map, None);
    assert!(sound_emitters.is_empty());
    assert_eq!(blend_batches.len(), 2);
    assert_eq!(
        blend_batches[0],
        BlendBatch {
            mbmh_index: 1,
            index_count: 3,
            index_first: 5,
            vertex_count: 4,
            vertex_first: 6,
        }
    );
    assert_eq!(
        blend_batches[1],
        BlendBatch {
            mbmh_index: 7,
            index_count: 8,
            index_first: 9,
            vertex_count: 10,
            vertex_first: 11,
        }
    );
    assert_eq!(detail_doodad_disable, None);
}

#[test]
fn parse_mcnk_subchunks_reads_mcdd_disable_bitmap() {
    let payload = mcnk_subchunks_payload(false, false, false, false, true);

    let (
        _,
        _,
        _,
        vertex_lighting,
        shadow_map,
        sound_emitters,
        blend_batches,
        detail_doodad_disable,
    ) = parse_mcnk_subchunks(&payload, McnkFlags::default()).expect("expected MCDD bitmap");

    let detail_doodad_disable = detail_doodad_disable.expect("expected parsed MCDD bitmap");
    assert_eq!(vertex_lighting, None);
    assert_eq!(shadow_map, None);
    assert!(sound_emitters.is_empty());
    assert!(blend_batches.is_empty());
    assert_eq!(detail_doodad_disable[0], 0b0000_0001);
    assert_eq!(detail_doodad_disable[1], 0b1000_0000);
    assert_eq!(detail_doodad_disable[7], 0b0101_0101);
}

#[test]
fn parse_mcnk_subchunks_requires_mcsh_when_flagged() {
    let payload = mcnk_subchunks_payload(false, false, false, false, false);

    let err = parse_mcnk_subchunks(
        &payload,
        McnkFlags {
            has_mcsh: true,
            impass: false,
            has_mccv: false,
            do_not_fix_alpha_map: false,
            high_res_holes: false,
        },
    )
    .expect_err("expected missing MCSH to be rejected");

    assert!(err.contains("flagged with MCSH"));
}

#[test]
fn parse_mcnk_subchunks_reads_mclv_even_when_it_is_not_first() {
    let payload = mcnk_subchunks_payload(false, true, false, false, false);

    let (
        _,
        _,
        _,
        vertex_lighting,
        shadow_map,
        sound_emitters,
        blend_batches,
        detail_doodad_disable,
    ) = parse_mcnk_subchunks(&payload, McnkFlags::default())
        .expect("expected vertex lighting to be parsed");

    let vertex_lighting = vertex_lighting.expect("expected parsed MCLV");
    assert_eq!(vertex_lighting[0], [1.0, 0.5, 0.0, 1.0]);
    assert_eq!(shadow_map, None);
    assert!(sound_emitters.is_empty());
    assert!(blend_batches.is_empty());
    assert_eq!(detail_doodad_disable, None);
}

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
    append_subchunk(
        &mut data,
        b"HMBM",
        blend_mesh_header_payload(77, 5, 0, 3, 3, 0, 0),
    );

    let err = match load_adt_raw(&data) {
        Ok(_) => panic!("expected incomplete blend mesh data to fail"),
        Err(err) => err,
    };
    assert!(err.contains("missing VNBM"));
}

fn mcnk_subchunks_payload(
    include_mcsh: bool,
    include_mclv: bool,
    include_mcse: bool,
    include_mcbb: bool,
    include_mcdd: bool,
) -> Vec<u8> {
    let mut payload = Vec::new();
    append_base_subchunks(&mut payload);
    append_optional_mcsh_subchunks(&mut payload, include_mcsh);
    append_optional_mclv_subchunk(&mut payload, include_mclv);
    append_optional_mcse_subchunk(&mut payload, include_mcse);
    append_optional_mcbb_subchunk(&mut payload, include_mcbb);
    append_optional_mcdd_subchunk(&mut payload, include_mcdd);
    payload
}

fn adt_file_payload(include_blend_mesh: bool) -> Vec<u8> {
    let mut payload = Vec::new();
    append_subchunk(
        &mut payload,
        b"KNCM",
        mcnk_payload(false, false, false, true, false),
    );
    append_subchunk(&mut payload, b"OFBM", mfbo_payload());
    if include_blend_mesh {
        append_subchunk(
            &mut payload,
            b"HMBM",
            blend_mesh_header_payload(77, 5, 0, 3, 3, 0, 0),
        );
        append_subchunk(&mut payload, b"BBBM", blend_mesh_bounds_payload(77));
        append_subchunk(&mut payload, b"VNBM", blend_mesh_vertex_payload());
        append_subchunk(&mut payload, b"IMBM", blend_mesh_index_payload());
    }
    payload
}

fn mfbo_payload() -> Vec<u8> {
    let mut payload = Vec::new();
    for value in [-10i16, -9, -8, -7, -6, -5, -4, -3, -2] {
        payload.extend_from_slice(&value.to_le_bytes());
    }
    for value in [20i16, 21, 22, 23, 24, 25, 26, 27, 28] {
        payload.extend_from_slice(&value.to_le_bytes());
    }
    payload
}

fn mcnk_payload(
    include_mcsh: bool,
    include_mclv: bool,
    include_mcse: bool,
    include_mcbb: bool,
    include_mcdd: bool,
) -> Vec<u8> {
    let mut payload = vec![0; 128];
    payload[4..8].copy_from_slice(&3u32.to_le_bytes());
    payload[8..12].copy_from_slice(&7u32.to_le_bytes());
    payload[60..64].copy_from_slice(&TEST_AREA_ID.to_le_bytes());
    payload.extend_from_slice(&mcnk_subchunks_payload(
        include_mcsh,
        include_mclv,
        include_mcse,
        include_mcbb,
        include_mcdd,
    ));
    payload
}

fn append_base_subchunks(payload: &mut Vec<u8>) {
    append_subchunk(payload, b"TVCM", vec![0; MCVT_COUNT * 4]);
    append_subchunk(payload, b"RNCM", vec![0; MCVT_COUNT * 3]);
}

fn append_optional_mcsh_subchunks(payload: &mut Vec<u8>, include_mcsh: bool) {
    if !include_mcsh {
        return;
    }

    let mut shadow_map = vec![0; 512];
    set_shadow_map_payload_bit(&mut shadow_map, 0, 62, false);
    set_shadow_map_payload_bit(&mut shadow_map, 0, 63, true);
    set_shadow_map_payload_bit(&mut shadow_map, 62, 0, false);
    set_shadow_map_payload_bit(&mut shadow_map, 62, 1, true);
    set_shadow_map_payload_bit(&mut shadow_map, 63, 0, true);
    set_shadow_map_payload_bit(&mut shadow_map, 63, 1, false);
    set_shadow_map_payload_bit(&mut shadow_map, 63, 63, false);
    append_subchunk(payload, b"HSCM", shadow_map);
    append_subchunk(payload, b"VCCM", vec![0x7F; MCVT_COUNT * 4]);
}

fn shadow_map_bit(shadow_map: &[u8; 512], row: usize, col: usize) -> bool {
    let byte = shadow_map[row * 8 + col / 8];
    ((byte >> (7 - (col % 8))) & 1) != 0
}

fn set_shadow_map_payload_bit(shadow_map: &mut [u8], row: usize, col: usize, value: bool) {
    let byte_index = row * 8 + col / 8;
    let mask = 1 << (7 - (col % 8));
    if value {
        shadow_map[byte_index] |= mask;
    } else {
        shadow_map[byte_index] &= !mask;
    }
}

fn append_optional_mclv_subchunk(payload: &mut Vec<u8>, include_mclv: bool) {
    if !include_mclv {
        return;
    }

    let mut vertex_lighting = vec![0; MCVT_COUNT * 4];
    vertex_lighting[0..4].copy_from_slice(&[0x00, 0x40, 0x80, 0xFF]);
    append_subchunk(payload, b"VLCM", vertex_lighting);
}

fn append_optional_mcse_subchunk(payload: &mut Vec<u8>, include_mcse: bool) {
    if !include_mcse {
        return;
    }

    let mut sound_emitters = Vec::new();
    sound_emitters.extend_from_slice(&42u32.to_le_bytes());
    sound_emitters.extend_from_slice(&100.0f32.to_le_bytes());
    sound_emitters.extend_from_slice(&200.0f32.to_le_bytes());
    sound_emitters.extend_from_slice(&300.0f32.to_le_bytes());
    sound_emitters.extend_from_slice(&10.0f32.to_le_bytes());
    sound_emitters.extend_from_slice(&20.0f32.to_le_bytes());
    sound_emitters.extend_from_slice(&30.0f32.to_le_bytes());
    sound_emitters.extend_from_slice(&7u32.to_le_bytes());
    sound_emitters.extend_from_slice(&1.0f32.to_le_bytes());
    sound_emitters.extend_from_slice(&2.0f32.to_le_bytes());
    sound_emitters.extend_from_slice(&3.0f32.to_le_bytes());
    sound_emitters.extend_from_slice(&4.0f32.to_le_bytes());
    sound_emitters.extend_from_slice(&5.0f32.to_le_bytes());
    sound_emitters.extend_from_slice(&6.0f32.to_le_bytes());
    append_subchunk(payload, b"MCSE", sound_emitters);
}

fn append_optional_mcbb_subchunk(payload: &mut Vec<u8>, include_mcbb: bool) {
    if !include_mcbb {
        return;
    }

    let mut blend_batches = Vec::new();
    append_blend_batch(&mut blend_batches, 1, 3, 5, 4, 6);
    append_blend_batch(&mut blend_batches, 7, 8, 9, 10, 11);
    append_subchunk(payload, b"BBCM", blend_batches);
}

fn append_optional_mcdd_subchunk(payload: &mut Vec<u8>, include_mcdd: bool) {
    if !include_mcdd {
        return;
    }

    let mut disable = [0u8; 64];
    disable[0] = 0b0000_0001;
    disable[1] = 0b1000_0000;
    disable[7] = 0b0101_0101;
    append_subchunk(payload, b"DDCM", disable.to_vec());
}

fn append_blend_batch(
    payload: &mut Vec<u8>,
    mbmh_index: u32,
    index_count: u32,
    index_first: u32,
    vertex_count: u32,
    vertex_first: u32,
) {
    payload.extend_from_slice(&mbmh_index.to_le_bytes());
    payload.extend_from_slice(&index_count.to_le_bytes());
    payload.extend_from_slice(&index_first.to_le_bytes());
    payload.extend_from_slice(&vertex_count.to_le_bytes());
    payload.extend_from_slice(&vertex_first.to_le_bytes());
}

fn blend_mesh_header_payload(
    map_object_id: u32,
    texture_id: u32,
    unknown: u32,
    index_count: u32,
    vertex_count: u32,
    index_start: u32,
    vertex_start: u32,
) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&map_object_id.to_le_bytes());
    payload.extend_from_slice(&texture_id.to_le_bytes());
    payload.extend_from_slice(&unknown.to_le_bytes());
    payload.extend_from_slice(&index_count.to_le_bytes());
    payload.extend_from_slice(&vertex_count.to_le_bytes());
    payload.extend_from_slice(&index_start.to_le_bytes());
    payload.extend_from_slice(&vertex_start.to_le_bytes());
    payload
}

fn blend_mesh_bounds_payload(map_object_id: u32) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&map_object_id.to_le_bytes());
    for value in [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0] {
        payload.extend_from_slice(&value.to_le_bytes());
    }
    payload
}

fn blend_mesh_vertex_payload() -> Vec<u8> {
    let mut payload = Vec::new();
    append_blend_mesh_vertex(
        &mut payload,
        [10.0, 20.0, 30.0],
        [0.0, 1.0, 0.0],
        [0.25, 0.75],
        [[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]],
    );
    append_blend_mesh_vertex(
        &mut payload,
        [11.0, 21.0, 31.0],
        [1.0, 0.0, 0.0],
        [0.5, 0.25],
        [[13, 14, 15, 16], [17, 18, 19, 20], [21, 22, 23, 24]],
    );
    append_blend_mesh_vertex(
        &mut payload,
        [12.0, 22.0, 32.0],
        [0.0, 0.0, 1.0],
        [1.0, 0.0],
        [[25, 26, 27, 28], [29, 30, 31, 32], [33, 34, 35, 36]],
    );
    payload
}

fn append_blend_mesh_vertex(
    payload: &mut Vec<u8>,
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
    color: [[u8; 4]; 3],
) {
    for value in position {
        payload.extend_from_slice(&value.to_le_bytes());
    }
    for value in normal {
        payload.extend_from_slice(&value.to_le_bytes());
    }
    for value in uv {
        payload.extend_from_slice(&value.to_le_bytes());
    }
    for rgba in color {
        payload.extend_from_slice(&rgba);
    }
}

fn blend_mesh_index_payload() -> Vec<u8> {
    let mut payload = Vec::new();
    for index in [0u16, 1, 2] {
        payload.extend_from_slice(&index.to_le_bytes());
    }
    payload
}

fn append_subchunk(payload: &mut Vec<u8>, tag: &[u8; 4], chunk_payload: Vec<u8>) {
    payload.extend_from_slice(tag);
    payload.extend_from_slice(&(chunk_payload.len() as u32).to_le_bytes());
    payload.extend_from_slice(&chunk_payload);
}

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
        Some(super::LodLiquidDirectory {
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
    assert_eq!(lod.m2_placements[0].flags, 0x55AA);
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
    append_subchunk(&mut payload, b"REVM", 18u32.to_le_bytes().to_vec());
    append_subchunk(
        &mut payload,
        b"DHLM",
        lod_header_payload(7, [10.0, 30.0, 50.0], [20.0, 40.0, 60.0]),
    );
    append_subchunk(
        &mut payload,
        b"HVLM",
        [1.5f32, 2.5, 3.5, 4.5]
            .into_iter()
            .flat_map(|value| value.to_le_bytes())
            .collect(),
    );
    append_subchunk(&mut payload, b"LLLM", lod_levels_payload());
    append_subchunk(&mut payload, b"DNLM", lod_nodes_payload());
    append_subchunk(
        &mut payload,
        b"IVLM",
        lod_index_payload(&[0, 1, 2, 3, 4, 5]),
    );
    append_subchunk(&mut payload, b"ISLM", lod_index_payload(&[6, 7, 8, 9]));
    append_subchunk(&mut payload, b"DLLM", vec![0xAA, 0xBB, 0xCC, 0xDD]);
    append_subchunk(
        &mut payload,
        b"NLLM",
        lod_liquid_header_payload([1, 2, 3, 4, 5, 6]),
    );
    append_subchunk(&mut payload, b"ILLM", lod_index_payload(&[10, 11, 12]));
    append_subchunk(
        &mut payload,
        b"VLLM",
        lod_liquid_vertices_payload(&[[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]),
    );
    append_subchunk(
        &mut payload,
        b"DDLM",
        lod_object_placement_payload(100, 200, [1.0, 2.0, 3.0], [4.0, 5.0, 6.0], 1.5, 0x55AA),
    );
    append_subchunk(
        &mut payload,
        b"XDLM",
        lod_object_visibility_payload([10.0, 20.0, 30.0], [40.0, 50.0, 60.0], 70.0),
    );
    append_subchunk(
        &mut payload,
        b"DMLM",
        lod_object_placement_payload(300, 400, [7.0, 8.0, 9.0], [10.0, 11.0, 12.0], 2.5, 0xAA55),
    );
    append_subchunk(
        &mut payload,
        b"XMLM",
        lod_object_visibility_payload([15.0, 25.0, 35.0], [45.0, 55.0, 65.0], 75.0),
    );
    payload
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
