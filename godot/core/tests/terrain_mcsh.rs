//! S3 mask DATA encoding and existing split ownership, not shadow brightness.
use game_engine_core::adt;

#[test]
fn mcsh_coverage_preserves_every_bit_in_chunk_uv_row_order() {
    for mask in [
        [0; 512],
        [255; 512],
        std::array::from_fn(|i| if i % 8 < 4 { 255 } else { 0 }),
    ] {
        let pixels = adt::unpack_shadow_coverage(&mask);
        assert_eq!(pixels.len(), 64 * 64);
        for row in 0..64 {
            for col in 0..64 {
                let set = mask[row * 8 + col / 8] & (1 << (col % 8)) != 0;
                assert_eq!(pixels[row * 64 + col], if set { 255 } else { 0 });
            }
        }
    }
    let mut mask = [0; 512];
    mask[0] = 0x81;
    mask[8] = 0x02;
    mask[511] = 0x80;
    let pixels = adt::unpack_shadow_coverage(&mask);
    let set: Vec<_> = pixels
        .iter()
        .enumerate()
        .filter_map(|(i, &v)| (v != 0).then_some(i))
        .collect();
    assert_eq!(set, [0, 7, 65, 4095]);
}

#[test]
fn mcsh_real_mask_is_resolved_from_texture_companion_without_rewriting_it() {
    let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/terrain");
    let root = std::fs::read(data.join("2703_31_36.adt")).unwrap();
    let tex = std::fs::read(data.join("2703_31_36_tex0.adt")).unwrap();
    let failure = adt::parse_root_for_tile(&root, 31, 36, None)
        .err()
        .expect("root declares companion shadows");
    assert!(failure.contains("MCSH"), "{failure}");
    let parsed = adt::parse_root_for_tile(&root, 31, 36, Some(&tex)).unwrap();
    let chunk = &parsed.chunks[7];
    assert_eq!((chunk.index_x, chunk.index_y), (7, 0));
    assert!(chunk.do_not_fix_alpha_map);
    let mask = chunk.shadow_map.as_ref().expect("real nontrivial mask");
    assert_eq!(mask.iter().map(|b| b.count_ones()).sum::<u32>(), 2230);
    let pixels = adt::unpack_shadow_coverage(mask);
    assert_eq!(pixels.iter().filter(|&&v| v == 255).count(), 2230);
    assert_eq!(std::fs::read(data.join("2703_31_36.adt")).unwrap(), root);
    assert_eq!(
        std::fs::read(data.join("2703_31_36_tex0.adt")).unwrap(),
        tex
    );
}
