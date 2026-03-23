//! ADT seam diagnostics.
//!
//! The raw parser is internally consistent for intra-tile MCNK borders, but cross-tile
//! borders in the fresh extracts still differ and likely need stitching.
//!
//! Important orientation detail:
//! - `(index_x, index_y + 1)` neighbors share the `row 16 -> row 0` border
//! - `(index_x + 1, index_y)` neighbors share the `col 8 -> col 0` border
//!
//! Using the opposite border orientation can manufacture fake seam diffs even when the
//! parser decoded the chunk data correctly.

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::asset::adt::{self, ChunkHeightGrid, load_adt_raw, vertex_index};

    type GridIndex<'a> = HashMap<(u32, u32), &'a ChunkHeightGrid>;

    fn absolute_height(grid: &ChunkHeightGrid, grid_row: usize, col: usize) -> f32 {
        grid.base_y + grid.heights[vertex_index(grid_row, col)]
    }

    fn index_grids(adt: &adt::AdtData) -> GridIndex<'_> {
        adt.height_grids
            .iter()
            .map(|g| ((g.index_x, g.index_y), g))
            .collect()
    }

    fn load_raw(path: &str) -> adt::AdtData {
        let data = std::fs::read(path).unwrap_or_else(|_| panic!("missing {path}"));
        load_adt_raw(&data).unwrap_or_else(|_| panic!("parse failed: {path}"))
    }

    fn col_border_max_diff(a: &ChunkHeightGrid, ac: usize, b: &ChunkHeightGrid, bc: usize) -> f32 {
        (0..=8)
            .map(|row| {
                let ah = absolute_height(a, row * 2, ac);
                let bh = absolute_height(b, row * 2, bc);
                (ah - bh).abs()
            })
            .fold(0.0f32, f32::max)
    }

    fn row_border_max_diff(a: &ChunkHeightGrid, ar: usize, b: &ChunkHeightGrid, br: usize) -> f32 {
        (0..=8)
            .map(|col| {
                let ah = absolute_height(a, ar, col);
                let bh = absolute_height(b, br, col);
                (ah - bh).abs()
            })
            .fold(0.0f32, f32::max)
    }

    fn parse_raw_binary_chunks(path: &str) -> HashMap<(u32, u32), (f32, [f32; 145])> {
        let data = std::fs::read(path).unwrap_or_else(|_| panic!("missing {path}"));
        let mut chunks = HashMap::new();
        for chunk in adt::ChunkIter::new(&data) {
            let (tag, payload) = chunk.expect("chunk");
            if tag != b"KNCM" {
                continue;
            }
            let index_x =
                u32::from_le_bytes(payload[0x04..0x08].try_into().expect("index_x bytes"));
            let index_y =
                u32::from_le_bytes(payload[0x08..0x0c].try_into().expect("index_y bytes"));
            let base_y = f32::from_le_bytes(payload[0x70..0x74].try_into().expect("base_y bytes"));
            let mut heights = None;
            let mut sub_off = 128usize;
            while sub_off + 8 <= payload.len() {
                let sub_tag = &payload[sub_off..sub_off + 4];
                let sub_size = u32::from_le_bytes(
                    payload[sub_off + 4..sub_off + 8]
                        .try_into()
                        .expect("sub size bytes"),
                ) as usize;
                let sub_payload = &payload[sub_off + 8..sub_off + 8 + sub_size];
                if sub_tag == b"TVCM" {
                    let mut parsed = [0.0f32; 145];
                    for (i, h) in parsed.iter_mut().enumerate() {
                        let start = i * 4;
                        *h = f32::from_le_bytes(
                            sub_payload[start..start + 4]
                                .try_into()
                                .expect("height bytes"),
                        );
                    }
                    heights = Some(parsed);
                    break;
                }
                sub_off += 8 + sub_size;
            }
            chunks.insert(
                (index_x, index_y),
                (base_y, heights.expect("MCNK missing TVCM")),
            );
        }
        chunks
    }

    fn bevy_to_tile_coords(bx: f32, bz: f32) -> (u32, u32) {
        let center = 32.0 * adt::CHUNK_SIZE * 16.0;
        let row = ((center + bz) / (adt::CHUNK_SIZE * 16.0)).floor() as i32;
        let col = ((center - bx) / (adt::CHUNK_SIZE * 16.0)).floor() as i32;
        (row.clamp(0, 63) as u32, col.clamp(0, 63) as u32)
    }

    /// Verify that FDID-named files parse to the expected tile coordinates.
    #[test]
    #[ignore]
    fn fdid_files_map_to_expected_tiles() {
        for (path, expected) in [
            ("data/terrain/azeroth_32_48.adt", (32, 48)),
            ("data/terrain/fresh/778027.adt", (32, 48)),
            ("data/terrain/fresh/778022.adt", (32, 47)),
        ] {
            let raw = load_raw(path);
            let c = raw.center_surface;
            let tile = bevy_to_tile_coords(c[0], c[2]);
            assert_eq!(tile, expected, "tile mismatch for {path}");
        }
    }

    #[test]
    fn raw_parser_matches_direct_binary_chunk_samples() {
        let raw = load_raw("data/terrain/fresh/778027.adt");
        let idx = index_grids(&raw);
        let binary = parse_raw_binary_chunks("data/terrain/fresh/778027.adt");

        for key in [(0, 0), (0, 1), (1, 0), (8, 8), (8, 9), (9, 8)] {
            let parsed = idx[&key];
            let (base_y, heights) = binary[&key];
            assert!(
                (parsed.base_y - base_y).abs() < 0.001,
                "base_y mismatch for {key:?}: {} vs {}",
                parsed.base_y,
                base_y
            );
            for (i, expected) in heights.iter().enumerate() {
                let actual = parsed.heights[i];
                assert!(
                    (actual - expected).abs() < 0.001,
                    "height[{i}] mismatch for {key:?}: {actual} vs {expected}"
                );
            }
        }
    }

    #[test]
    fn raw_intra_tile_borders_match_on_expected_edges() {
        let raw = load_raw("data/terrain/fresh/778027.adt");
        let idx = index_grids(&raw);

        let mut max_y_neighbor_diff = 0.0f32;
        for index_x in 0..16u32 {
            for index_y in 0..15u32 {
                let a = idx[&(index_x, index_y)];
                let b = idx[&(index_x, index_y + 1)];
                max_y_neighbor_diff = max_y_neighbor_diff.max(row_border_max_diff(a, 16, b, 0));
            }
        }

        let mut max_x_neighbor_diff = 0.0f32;
        for index_x in 0..15u32 {
            for index_y in 0..16u32 {
                let a = idx[&(index_x, index_y)];
                let b = idx[&(index_x + 1, index_y)];
                max_x_neighbor_diff = max_x_neighbor_diff.max(col_border_max_diff(a, 8, b, 0));
            }
        }

        assert!(
            max_y_neighbor_diff < 0.001,
            "expected index_y neighbors to match on row edges, got {max_y_neighbor_diff}"
        );
        assert!(
            max_x_neighbor_diff < 0.001,
            "expected index_x neighbors to match on col edges, got {max_x_neighbor_diff}"
        );
    }

    /// Cross-tile raw borders still differ in the fresh extracts even when paired on the
    /// plausible shared edge.
    #[test]
    #[ignore]
    fn raw_cross_tile_borders_differ() {
        let raw_48 = load_raw("data/terrain/fresh/778027.adt");
        let raw_47 = load_raw("data/terrain/fresh/778022.adt");
        let idx_48 = index_grids(&raw_48);
        let idx_47 = index_grids(&raw_47);

        let mut max_diff = 0.0f32;
        for cx in 0..16u32 {
            let a = idx_48[&(cx, 0)];
            let b = idx_47[&(cx, 15)];
            let d = col_border_max_diff(a, 0, b, 8);
            max_diff = max_diff.max(d);
        }
        println!("Cross-tile 48(y=0,col=0) vs 47(y=15,col=8) max: {max_diff:.2}");
        assert!(max_diff > 1.0, "expected raw cross-tile border to differ");
    }
}
