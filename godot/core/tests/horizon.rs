//! The WDL horizon of Azeroth (`azeroth.wdl`, FDID 775970) against the Northshire ADT it
//! summarises (azeroth_32_48).
use game_engine_core::{adt, horizon::parse_wdl};

fn cached(path: &str) -> Vec<u8> {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/");
    std::fs::read(format!("{root}{path}")).unwrap_or_else(|error| panic!("{path}: {error}"))
}

#[test]
fn azeroth_wdl_covers_northshire_where_its_adt_lies() {
    let tiles = parse_wdl(&cached("terrain/775970.wdl")).unwrap();
    assert_eq!(tiles.len(), 1176);
    let northshire = tiles.iter().find(|tile| tile.tile == (32, 48)).unwrap();
    assert_eq!(northshire.positions.len(), 545);
    assert_eq!(northshire.indices.len(), 16 * 16 * 12);
    let root =
        adt::parse_root_for_tile(&cached("terrain/azeroth_32_48.adt"), 32, 48, None).unwrap();
    // Each WDL corner (row, column) is vertex 0 of MCNK (index_x column, index_y row).
    for chunk in &root.chunks {
        let (row, column) = (chunk.index_y as usize, chunk.index_x as usize);
        let wdl = northshire.positions[row * 17 + column];
        let vertex = adt::chunk_geometry(chunk, Some((32, 48))).positions[0];
        assert!(
            (wdl[0] - vertex[0]).abs() < 0.01 && (wdl[2] - vertex[2]).abs() < 0.01,
            "corner {row},{column}: {wdl:?} vs {vertex:?}"
        );
        assert!(
            (wdl[1] - vertex[1]).abs() <= 1.0,
            "corner {row},{column} height: {wdl:?} vs {vertex:?}"
        );
    }
}
