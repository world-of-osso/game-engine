//! The map's low-detail horizon: WDL heights of every tile (MAOF offsets, MARE 17×17 corner
//! heights then 16×16 cell centres, MAHO face masks), meshed as the client's horizon fan
//! (solarityclient `asset/src/terrain/map_low_detail.rs`, `rendering/src/terrain/low_detail`,
//! client 7CC310/7D5150/7D5240). Positions are engine axes (x, y up, z).

use crate::asset::read_bytes::{read_i16, read_u16, read_u32};

/// One WDL tile's horizon mesh.
pub struct HorizonTile {
    /// `(tile_y, tile_x)`: the tile `{map}_{tile_y}_{tile_x}.adt`.
    pub tile: (u32, u32),
    /// The 17×17 corners then the 16×16 cell centres.
    pub positions: Vec<[f32; 3]>,
    /// Four triangles per cell around its centre, cells unmarked by MAHO first.
    pub indices: Vec<u16>,
}

const CORNERS: usize = 17 * 17;
const HEIGHTS: usize = CORNERS + 16 * 16;
const STEP: f32 = 33.333_332;
const ORIGIN: f32 = 17_066.666;
const CENTRE_INSET: f32 = 16.666_666;

/// Every tile of a `.wdl` file.
pub fn parse_wdl(bytes: &[u8]) -> Result<Vec<HorizonTile>, String> {
    let offsets = find_chunk(bytes, b"FOAM").ok_or("WDL has no MAOF")?.1;
    if offsets.len() < 64 * 64 * 4 {
        return Err(format!("WDL MAOF has {} bytes", offsets.len()));
    }
    let mut tiles = Vec::new();
    for index in 0..64 * 64 {
        let offset = read_u32(offsets, index * 4)? as usize;
        if offset == 0 {
            continue;
        }
        // MAOF is indexed by the second ADT file number, then the first.
        let tile = ((index % 64) as u32, (index / 64) as u32);
        tiles.push(read_tile(bytes, offset, tile)?);
    }
    Ok(tiles)
}

fn read_tile(bytes: &[u8], offset: usize, tile: (u32, u32)) -> Result<HorizonTile, String> {
    let context = |error: String| format!("WDL tile {tile:?} at {offset}: {error}");
    let (tag, mare) = chunk_at(bytes, offset).ok_or_else(|| context("truncated MARE".into()))?;
    if &tag != b"ERAM" || mare.len() < HEIGHTS * 2 {
        return Err(context(format!("expected MARE of {} bytes", HEIGHTS * 2)));
    }
    let heights: Vec<i16> = (0..HEIGHTS)
        .map(|index| read_i16(mare, index * 2))
        .collect::<Result<_, _>>()
        .map_err(context)?;
    // MAHO, when present, follows its tile's MARE.
    let next = offset + 8 + mare.len();
    let masks = match chunk_at(bytes, next) {
        Some((tag, maho)) if &tag == b"OHAM" && maho.len() >= 32 => (0..16)
            .map(|row| read_u16(maho, row * 2))
            .collect::<Result<Vec<_>, _>>()
            .map_err(context)?,
        _ => vec![0; 16],
    };
    Ok(HorizonTile {
        tile,
        positions: positions(tile, &heights),
        indices: indices(&masks),
    })
}

/// 7D5150: corners on the 33.3-yard grid from the tile's north-west corner, centres inset
/// by half a step; WDL heights are whole yards.
fn positions((tile_y, tile_x): (u32, u32), heights: &[i16]) -> Vec<[f32; 3]> {
    let base_x = ORIGIN - tile_x as f32 * 16.0 * STEP;
    let base_y = ORIGIN - tile_y as f32 * 16.0 * STEP;
    let mut positions = Vec::with_capacity(HEIGHTS);
    for (start, width, inset) in [(0, 17, 0.0), (CORNERS, 16, CENTRE_INSET)] {
        for row in 0..width {
            for column in 0..width {
                let wow_x = base_x - inset - row as f32 * STEP;
                let wow_y = base_y - inset - column as f32 * STEP;
                let height = f32::from(heights[start + row * width + column]);
                positions.push([wow_x, height, -wow_y]);
            }
        }
    }
    positions
}

/// 7D5240: every cell's fan, unmarked cells first, then the MAHO-marked ones.
fn indices(masks: &[u16]) -> Vec<u16> {
    let mut indices = Vec::with_capacity(16 * 16 * 12);
    for marked in [false, true] {
        for (row, mask) in masks.iter().enumerate() {
            for column in 0..16 {
                if (mask & (1 << column) != 0) != marked {
                    continue;
                }
                let corner = (row * 17 + column) as u16;
                let centre = (CORNERS + row * 16 + column) as u16;
                indices.extend_from_slice(&[
                    centre,
                    corner + 1,
                    corner,
                    centre,
                    corner + 18,
                    corner + 1,
                    centre,
                    corner + 17,
                    corner + 18,
                    centre,
                    corner,
                    corner + 17,
                ]);
            }
        }
    }
    indices
}

/// The chunk at `offset`: its tag as stored (reversed) and payload.
fn chunk_at(bytes: &[u8], offset: usize) -> Option<([u8; 4], &[u8])> {
    let tag: [u8; 4] = bytes.get(offset..offset + 4)?.try_into().ok()?;
    let size = read_u32(bytes, offset + 4).ok()? as usize;
    Some((tag, bytes.get(offset + 8..offset + 8 + size)?))
}

/// The first top-level chunk tagged `tag` (as stored, reversed).
fn find_chunk<'a>(bytes: &'a [u8], tag: &[u8; 4]) -> Option<(usize, &'a [u8])> {
    let mut offset = 0;
    while let Some((found, payload)) = chunk_at(bytes, offset) {
        if &found == tag {
            return Some((offset, payload));
        }
        offset += 8 + payload.len();
    }
    None
}
