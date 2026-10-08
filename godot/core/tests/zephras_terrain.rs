use game_engine_core::{adt, wdt};

#[test]
fn zephras_sample_parses_authored_terrain_textures_and_placements() {
    let root_bytes = std::fs::read("data/terrain/7199999.adt").unwrap();
    let tex_bytes = std::fs::read("data/terrain/7200002.adt").unwrap();
    let obj_bytes = std::fs::read("data/terrain/7200000.adt").unwrap();
    let wdt_bytes = std::fs::read("data/terrain/7198644.wdt").unwrap();
    let root = adt::parse_root_for_tile(&root_bytes, 29, 26, Some(&tex_bytes)).unwrap();
    assert_eq!(root.chunks.len(), 256);
    assert_eq!(root.height_grids.len(), 256);
    assert!(root.chunks.iter().all(|chunk| chunk.has_vertex_colors));
    assert!(
        root.chunks
            .iter()
            .all(|chunk| chunk.vertex_lighting.is_none())
    );
    let bounds = root.flight_bounds.unwrap();
    assert_eq!(bounds.min_heights, [500; 9]);
    assert_eq!(bounds.max_heights, [1500; 9]);
    assert_eq!(root.chunk_positions[0], [1600.0, 3200.0, 752.71844]);
    assert!(root.water.is_some());
    assert!(root.water_error.is_none(), "{:?}", root.water_error);
    let tex = adt::parse_tex(
        &tex_bytes,
        wdt::parse_wdt_mphd_flags(&wdt_bytes).unwrap(),
        &root,
    )
    .unwrap();
    assert_eq!(
        tex.texture_fdids,
        [
            186823, 188574, 187911, 186967, 186825, 186975, 186985, 186819, 186983, 186821
        ]
    );
    assert_eq!(tex.chunk_layers.len(), 256);
    let obj = adt::parse_obj(&obj_bytes).unwrap();
    assert_eq!(obj.doodads.len(), 218);
    assert_eq!(
        obj.doodads
            .iter()
            .filter(|d| d.flags & 0x40 != 0 && d.fdid.is_some())
            .count(),
        218
    );
    assert_eq!(obj.doodads.iter().filter(|d| d.flags == 0x240).count(), 9);
    assert!(obj.doodads.iter().all(|d| d.path.is_none()));
    assert_eq!(
        obj.wmos.iter().map(|w| w.fdid.unwrap()).collect::<Vec<_>>(),
        [333477, 7704156, 7749471, 7704158]
    );
    assert!(obj.wmos.iter().all(|w| w.flags & 8 != 0));
}
