//! Authored far-terrain `_lod.adt` MLLD maps, not WMO MLIQ shoreline data.
//! Retail70688 E3 defines BE run/literal RLE, depth bytes and binary coverage.
//! The 12-byte directory header and LSB-first coverage agree with cached Neltharus
//! LOD4334712/4335067/4335112; do not assign these maps to unrelated WMO surfaces.

pub const MAP_SIDE: usize = 128;
pub const MAP_TEXELS: usize = MAP_SIDE * MAP_SIDE;
const BITMAP_BYTES: usize = MAP_TEXELS / 8;
const HEADER_BYTES: usize = 12;
const HAS_MAPS: u32 = 1;
const COMPRESSED_DEPTH: u32 = 2;
const COMPRESSED_ALPHA: u32 = 4;
const RUN_BIT: u16 = 0x8000;

/// Owned source and decoded texels survive dropping or repacking the mesh container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LodLiquidMaps {
    pub source: Vec<u8>,
    /// Raw byte values; no world-distance unit is inferred here.
    pub depth: Vec<u8>,
    /// One bit per texel expanded to exactly 0 or 255, never an alpha ramp.
    pub coverage: Vec<u8>,
}

/// Consume only the far-terrain container's MLLD payload via its existing chunk parser.
/// Absence means no authored maps, not permission to synthesise a shoreline.
pub fn parse_lod_liquid_maps(container: &[u8]) -> Result<Option<LodLiquidMaps>, String> {
    let lod = crate::adt::parse_lod(container)?;
    lod.liquid_directory
        .map(|directory| decode_directory(&directory.raw))
        .transpose()
        .map(Option::flatten)
}

fn decode_directory(payload: &[u8]) -> Result<Option<LodLiquidMaps>, String> {
    let header = payload
        .get(..HEADER_BYTES)
        .ok_or("MLLD header is shorter than 12 bytes")?;
    let flags = u32::from_le_bytes(header[..4].try_into().unwrap());
    if flags & HAS_MAPS == 0 {
        return Ok(None);
    }
    let depth_size = usize::from(u16::from_le_bytes(header[4..6].try_into().unwrap()));
    let alpha_size = usize::from(u16::from_le_bytes(header[6..8].try_into().unwrap()));
    let depth_end = HEADER_BYTES + depth_size;
    let depth_source = payload
        .get(HEADER_BYTES..depth_end)
        .ok_or("MLLD depth block is truncated")?;
    let alpha_source = payload
        .get(depth_end..depth_end + alpha_size)
        .ok_or("MLLD alpha block is truncated")?;
    let depth = decode_depth(depth_source, flags)?;
    if flags & COMPRESSED_ALPHA == 0 {
        return Err(
            "MLLD alpha encoding flag is absent; Retail E3 does not define raw alpha".into(),
        );
    }
    let bitmap = decode_rle(alpha_source, BITMAP_BYTES)?;
    let coverage = bitmap
        .iter()
        .flat_map(|byte| (0..8).map(move |bit| if byte & (1 << bit) == 0 { 0 } else { 255 }))
        .collect();
    Ok(Some(LodLiquidMaps {
        source: payload.to_vec(),
        depth,
        coverage,
    }))
}

fn decode_depth(source: &[u8], flags: u32) -> Result<Vec<u8>, String> {
    if flags & COMPRESSED_DEPTH != 0 {
        return decode_rle(source, MAP_TEXELS);
    }
    if source.len() != MAP_TEXELS {
        return Err(format!(
            "MLLD raw depth has {} bytes, expected {MAP_TEXELS}",
            source.len()
        ));
    }
    Ok(source.to_vec())
}

/// Big-endian tokens; zero-length literals and runs are no-ops, not terminators.
fn decode_rle(mut source: &[u8], output_size: usize) -> Result<Vec<u8>, String> {
    let mut output = Vec::with_capacity(output_size);
    while output.len() < output_size {
        let token = source.get(..2).ok_or("MLLD RLE token is truncated")?;
        let token = u16::from_be_bytes(token.try_into().unwrap());
        source = &source[2..];
        let count = usize::from(token & !RUN_BIT);
        if count > output_size - output.len() {
            return Err("MLLD RLE run exceeds the map output size".into());
        }
        if token & RUN_BIT != 0 {
            let value = *source.first().ok_or("MLLD RLE repeat value is truncated")?;
            source = &source[1..];
            output.resize(output.len() + count, value);
        } else {
            let literal = source.get(..count).ok_or("MLLD RLE literal is truncated")?;
            output.extend_from_slice(literal);
            source = &source[count..];
        }
    }
    Ok(output)
}

#[cfg(test)]
#[path = "liquid_lod_data_tests.rs"]
mod tests;
