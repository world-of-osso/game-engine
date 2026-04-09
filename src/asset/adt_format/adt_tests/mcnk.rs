use super::fixtures::{mcnk_subchunks_payload, shadow_map_bit};
use super::{
    MCNK_FLAG_DO_NOT_FIX_ALPHA_MAP, MCNK_FLAG_HAS_MCCV, MCNK_FLAG_HAS_MCSH,
    MCNK_FLAG_HIGH_RES_HOLES, MCNK_FLAG_IMPASS, MCVT_COUNT, McnkFlags, TEST_AREA_ID, parse_mccv,
    parse_mclv, parse_mcnk, parse_mcnk_subchunks,
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
    super::fixtures::append_subchunk(&mut payload, b"TVCM", vec![0; MCVT_COUNT * 4]);
    super::fixtures::append_subchunk(&mut payload, b"RNCM", vec![0; MCVT_COUNT * 3]);

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
    assert!(!shadow_map_bit(&shadow_map, 0, 63));
    assert!(shadow_map_bit(&shadow_map, 62, 1));
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
        super::BlendBatch {
            mbmh_index: 1,
            index_count: 3,
            index_first: 5,
            vertex_count: 4,
            vertex_first: 6,
        }
    );
    assert_eq!(
        blend_batches[1],
        super::BlendBatch {
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
