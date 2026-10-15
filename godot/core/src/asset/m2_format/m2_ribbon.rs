//! Retail M2Ribbon records (WebWowViewerCpp M2FileHeader.h).
//! Four20-byte animation tracks precede edge density/lifetime, not six.
use super::{m2_anim, read_f32, read_i16, read_u16, read_u32};
use crate::asset::read_bytes::read_m2_array_header;
use m2_anim::AnimTrack;

const HEADER_OFFSET: usize = 0x120;
const ENTRY_STRIDE: usize = 176;
const TEXTURES_OFFSET: usize = 0x14;
const MATERIALS_OFFSET: usize = 0x1c;
const COLOR_OFFSET: usize = 0x24;
const ALPHA_OFFSET: usize = 0x38;
const ABOVE_OFFSET: usize = 0x4c;
const BELOW_OFFSET: usize = 0x60;
const DENSITY_OFFSET: usize = 0x74;
const LIFETIME_OFFSET: usize = 0x78;
const ANGLE_OFFSET: usize = 0x7c;
const TILE_ROWS_OFFSET: usize = 0x80;
const TILE_COLS_OFFSET: usize = 0x82;
const SLOT_OFFSET: usize = 0x84;
const VISIBILITY_OFFSET: usize = 0x98;
const PRIORITY_OFFSET: usize = 0xac;
const COLOR_INDEX_OFFSET: usize = 0xae;
const TEXTURE_TRANSFORM_OFFSET: usize = 0xaf;

#[derive(Debug, Clone, PartialEq)]
pub struct M2RibbonEmitter {
    pub id: u32,
    pub bone_index: u32,
    /// Bone-relative WoW coordinates.
    pub position: [f32; 3],
    pub texture_indices: Vec<u16>,
    pub material_indices: Vec<u16>,
    pub color: AnimTrack<[f32; 3]>,
    pub alpha: AnimTrack<i16>,
    pub height_above: AnimTrack<f32>,
    pub height_below: AnimTrack<f32>,
    pub texture_slot: AnimTrack<u16>,
    pub visibility: AnimTrack<u8>,
    pub edges_per_second: f32,
    pub edge_lifetime: f32,
    /// CRibbonEmitter::Update world-Z displacement coefficient: gravity * age².
    pub gravity: f32,
    pub texture_rows: u16,
    pub texture_cols: u16,
    pub priority_plane: i16,
    pub color_index: i8,
    pub texture_transform_lookup: i8,
}

pub fn parse_ribbon_emitters(md20: &[u8]) -> Result<Vec<M2RibbonEmitter>, String> {
    let (count, offset) = read_m2_array_header(md20, HEADER_OFFSET)?;
    let end = count
        .checked_mul(ENTRY_STRIDE)
        .and_then(|size| offset.checked_add(size))
        .ok_or("M2 ribbon array size overflow")?;
    if end > md20.len() {
        return Err(format!(
            "M2 ribbon array ends at {end:#x}, beyond {} bytes",
            md20.len()
        ));
    }
    (0..count)
        .map(|index| parse_ribbon(md20, offset + index * ENTRY_STRIDE))
        .collect()
}

fn read_indices(md20: &[u8], header: usize) -> Result<Vec<u16>, String> {
    let (count, offset) = read_m2_array_header(md20, header)?;
    (0..count)
        .map(|index| read_u16(md20, offset + index * 2))
        .collect()
}

fn parse_ribbon(md20: &[u8], base: usize) -> Result<M2RibbonEmitter, String> {
    Ok(M2RibbonEmitter {
        id: read_u32(md20, base)?,
        bone_index: read_u32(md20, base + 4)?,
        position: [
            read_f32(md20, base + 8)?,
            read_f32(md20, base + 12)?,
            read_f32(md20, base + 16)?,
        ],
        texture_indices: read_indices(md20, base + TEXTURES_OFFSET)?,
        material_indices: read_indices(md20, base + MATERIALS_OFFSET)?,
        color: m2_anim::parse_vec3_track(md20, base + COLOR_OFFSET)?,
        alpha: m2_anim::parse_i16_track(md20, base + ALPHA_OFFSET)?,
        height_above: m2_anim::parse_f32_track(md20, base + ABOVE_OFFSET)?,
        height_below: m2_anim::parse_f32_track(md20, base + BELOW_OFFSET)?,
        texture_slot: m2_anim::parse_u16_track(md20, base + SLOT_OFFSET)?,
        visibility: m2_anim::parse_u8_track(md20, base + VISIBILITY_OFFSET)?,
        edges_per_second: read_f32(md20, base + DENSITY_OFFSET)?,
        edge_lifetime: read_f32(md20, base + LIFETIME_OFFSET)?,
        gravity: read_f32(md20, base + ANGLE_OFFSET)?,
        texture_rows: read_u16(md20, base + TILE_ROWS_OFFSET)?,
        texture_cols: read_u16(md20, base + TILE_COLS_OFFSET)?,
        priority_plane: read_i16(md20, base + PRIORITY_OFFSET)?,
        color_index: md20[base + COLOR_INDEX_OFFSET] as i8,
        texture_transform_lookup: md20[base + TEXTURE_TRANSFORM_OFFSET] as i8,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncated_ribbon_array_is_reported() {
        let mut bytes = vec![0; 0x140];
        bytes[HEADER_OFFSET..HEADER_OFFSET + 4].copy_from_slice(&1u32.to_le_bytes());
        bytes[HEADER_OFFSET + 4..HEADER_OFFSET + 8].copy_from_slice(&0x130u32.to_le_bytes());
        assert!(
            parse_ribbon_emitters(&bytes)
                .unwrap_err()
                .contains("beyond")
        );
    }

    #[test]
    fn absent_header_is_reported() {
        assert!(parse_ribbon_emitters(&[0; 16]).is_err());
    }

    #[test]
    fn valid_empty_ribbon_array() {
        assert!(parse_ribbon_emitters(&[0; 0x130]).unwrap().is_empty());
    }
}
