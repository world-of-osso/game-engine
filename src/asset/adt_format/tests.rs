use super::{
    MCNK_FLAG_DO_NOT_FIX_ALPHA_MAP, MCNK_FLAG_HAS_MCCV, MCNK_FLAG_HAS_MCSH,
    MCNK_FLAG_HIGH_RES_HOLES, MCNK_FLAG_IMPASS, MCVT_COUNT, McnkFlags, parse_mccv, parse_mclv,
    parse_mcnk, parse_mcnk_subchunks,
};

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
    let payload = mcnk_subchunks_payload(false, false, false);

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
    let payload = mcnk_subchunks_payload(false, false, false);

    let (_, _, colors, vertex_lighting, shadow_map, sound_emitters) =
        parse_mcnk_subchunks(&payload, McnkFlags::default())
            .expect("expected missing optional MCCV to default");

    assert_eq!(colors[0], [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(colors[MCVT_COUNT - 1], [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(vertex_lighting, None);
    assert_eq!(shadow_map, None);
    assert!(sound_emitters.is_empty());
}

#[test]
fn parse_mcnk_reads_area_id_from_header() {
    let mut payload = vec![0; 128];
    payload[0..4].copy_from_slice(&0u32.to_le_bytes());
    payload[4..8].copy_from_slice(&3u32.to_le_bytes());
    payload[8..12].copy_from_slice(&7u32.to_le_bytes());
    payload[60..64].copy_from_slice(&0x1234_5678u32.to_le_bytes());
    append_subchunk(&mut payload, b"TVCM", vec![0; MCVT_COUNT * 4]);
    append_subchunk(&mut payload, b"RNCM", vec![0; MCVT_COUNT * 3]);

    let chunk = parse_mcnk(&payload).expect("expected MCNK header to parse");
    assert_eq!(chunk.area_id, 0x1234_5678);
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
    let payload = mcnk_subchunks_payload(true, false, false);

    let (_, _, _, vertex_lighting, shadow_map, sound_emitters) = parse_mcnk_subchunks(
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
}

#[test]
fn parse_mcnk_subchunks_reads_mcse_emitters() {
    let payload = mcnk_subchunks_payload(false, false, true);

    let (_, _, _, vertex_lighting, shadow_map, sound_emitters) =
        parse_mcnk_subchunks(&payload, McnkFlags::default()).expect("expected MCSE emitters");

    assert_eq!(vertex_lighting, None);
    assert_eq!(shadow_map, None);
    assert_eq!(sound_emitters.len(), 2);
    assert_eq!(sound_emitters[0].sound_entry_id, 42);
    assert_eq!(sound_emitters[0].position, [100.0, 200.0, 300.0]);
    assert_eq!(sound_emitters[0].size_min, [10.0, 20.0, 30.0]);
    assert_eq!(sound_emitters[1].sound_entry_id, 7);
    assert_eq!(sound_emitters[1].position, [1.0, 2.0, 3.0]);
    assert_eq!(sound_emitters[1].size_min, [4.0, 5.0, 6.0]);
}

#[test]
fn parse_mcnk_subchunks_requires_mcsh_when_flagged() {
    let payload = mcnk_subchunks_payload(false, false, false);

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
    let payload = mcnk_subchunks_payload(false, true, false);

    let (_, _, _, vertex_lighting, shadow_map, sound_emitters) =
        parse_mcnk_subchunks(&payload, McnkFlags::default())
            .expect("expected vertex lighting to be parsed");

    let vertex_lighting = vertex_lighting.expect("expected parsed MCLV");
    assert_eq!(vertex_lighting[0], [1.0, 0.5, 0.0, 1.0]);
    assert_eq!(shadow_map, None);
    assert!(sound_emitters.is_empty());
}

fn mcnk_subchunks_payload(include_mcsh: bool, include_mclv: bool, include_mcse: bool) -> Vec<u8> {
    let mut payload = Vec::new();
    append_subchunk(&mut payload, b"TVCM", vec![0; MCVT_COUNT * 4]);
    append_subchunk(&mut payload, b"RNCM", vec![0; MCVT_COUNT * 3]);
    if include_mcsh {
        let mut shadow_map = vec![0; 512];
        shadow_map[0] = 0b1000_0000;
        shadow_map[1] = 0b0100_0000;
        append_subchunk(&mut payload, b"HSCM", shadow_map);
    }
    if include_mcsh {
        append_subchunk(&mut payload, b"VCCM", vec![0x7F; MCVT_COUNT * 4]);
    }
    if include_mclv {
        let mut vertex_lighting = vec![0; MCVT_COUNT * 4];
        vertex_lighting[0..4].copy_from_slice(&[0x00, 0x40, 0x80, 0xFF]);
        append_subchunk(&mut payload, b"VLCM", vertex_lighting);
    }
    if include_mcse {
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
        append_subchunk(&mut payload, b"MCSE", sound_emitters);
    }
    payload
}

fn append_subchunk(payload: &mut Vec<u8>, tag: &[u8; 4], chunk_payload: Vec<u8>) {
    payload.extend_from_slice(tag);
    payload.extend_from_slice(&(chunk_payload.len() as u32).to_le_bytes());
    payload.extend_from_slice(&chunk_payload);
}
