//! Cross-tile ADT seam diagnostics.
//!
//! Raw MCNK heights do NOT match at chunk borders — neither within one tile nor
//! across adjacent tiles. Diffs of 15–50 units are typical. The WoW client stitches
//! at runtime. Our existing `stitch_chunk_edges` handles intra-tile seams; cross-tile
//! stitching is still needed.
//!
//! Key finding: `data/terrain/fresh/778027.adt` (azeroth_32_48) and
//! `data/terrain/fresh/778022.adt` (azeroth_32_47) are from the same CASC build.
//! Their raw border heights differ by ~47 units at the shared edge.

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

    /// Intra-tile raw borders differ: chunk(0,0) col=8 vs chunk(0,1) col=0
    /// shows ~35 unit gaps. This is why stitch_chunk_edges exists.
    #[test]
    #[ignore]
    fn raw_intra_tile_borders_differ() {
        let raw = load_raw("data/terrain/fresh/778027.adt");
        let idx = index_grids(&raw);

        let a = idx[&(0, 0)];
        let b = idx[&(0, 1)];
        let diff = col_border_max_diff(a, 8, b, 0);
        println!("Intra-tile (0,0)col=8 vs (0,1)col=0 max_diff: {diff:.2}");
        assert!(diff > 1.0, "expected raw intra-tile border to differ");
    }

    /// Cross-tile raw borders differ by similar amounts as intra-tile.
    /// 32_48(iy=0, col=0) vs 32_47(iy=15, col=8) = shared edge at X≈-8533.
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
