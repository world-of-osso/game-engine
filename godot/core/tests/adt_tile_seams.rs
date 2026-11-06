//! Two adjacent ADTs must describe the same edge: every vertex they share in world X/Z has the
//! same height. Vertices carry absolute chunk positions, so a tile loaded into the wrong slot
//! shows up as a gap or an overlap here, not as a seam mismatch.

use std::collections::HashMap;

use game_engine_core::adt::{chunk_geometry, parse_root};

fn edge_heights(name: &str) -> HashMap<(i64, i64), f32> {
    let path = format!("{}/../../data/terrain/{name}", env!("CARGO_MANIFEST_DIR"));
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("{path}: {error}"));
    let root = parse_root(&bytes).unwrap_or_else(|error| panic!("{path}: {error}"));
    let mut heights = HashMap::new();
    for chunk in &root.chunks {
        for position in chunk_geometry(chunk, None).positions {
            heights.insert(
                (
                    (position[0] * 100.0).round() as i64,
                    (position[2] * 100.0).round() as i64,
                ),
                position[1],
            );
        }
    }
    heights
}

#[test]
fn adjacent_azeroth_tiles_share_their_edge_heights() {
    // `azeroth_32_48` is cached under its name, `azeroth_31_48` under FDID 777827; they are
    // neighbours one row apart, so the shared edge runs along a constant Z.
    let north = edge_heights("azeroth_32_48.adt");
    let south = edge_heights("777827.adt");
    let mut shared = 0;
    let mut worst = 0.0_f32;
    for (key, height) in &north {
        if let Some(other) = south.get(key) {
            shared += 1;
            worst = worst.max((height - other).abs());
        }
    }
    assert!(
        shared >= 128,
        "expected a full chunk row of shared edge vertices, found {shared}"
    );
    assert_eq!(worst, 0.0, "shared edge heights differ by {worst}");
}
