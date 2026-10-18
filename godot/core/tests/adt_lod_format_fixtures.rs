use game_engine_core::asset::adt_format::adt::*;

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
