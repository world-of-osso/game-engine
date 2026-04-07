//! M2 ribbon emitter parser.
//!
//! Parses ribbon emitter definitions from the MD20 header at offset 0x120.
//! Ribbon emitters create trailing strip geometry attached to bones — used
//! for sword trails, spell visual ribbons, and similar effects.
//!
//! Each ribbon emitter references a bone and texture, plus AnimBlock tracks
//! for color, alpha, height-above, and height-below the attachment point.

use super::{read_f32, read_u16, read_u32};
use crate::asset::read_bytes::read_m2_array_header;

/// MD20 header offset for ribbon emitter count + offset.
pub(crate) const MD20_RIBBON_EMITTERS_COUNT_OFFSET: usize = 0x120;

/// Stride of a ribbon emitter entry in the M2 file (legacy layout).
const RIBBON_ENTRY_STRIDE: usize = 176;

// Field offsets within a single ribbon emitter entry.
const RIBBON_ID_OFFSET: usize = 0x00;
const RIBBON_BONE_INDEX_OFFSET: usize = 0x04;
const RIBBON_POSITION_OFFSET: usize = 0x08;
const RIBBON_TEXTURE_COUNT_OFFSET: usize = 0x14;
const RIBBON_MATERIAL_COUNT_OFFSET: usize = 0x1C;
const RIBBON_EDGES_PER_SEC_OFFSET: usize = 0x94;
const RIBBON_EDGE_LIFETIME_OFFSET: usize = 0x98;
const RIBBON_GRAVITY_OFFSET: usize = 0x9C;
const RIBBON_TEXTURE_ROWS_OFFSET: usize = 0xA0;
const RIBBON_TEXTURE_COLS_OFFSET: usize = 0xA2;

/// Parsed M2 ribbon emitter definition.
#[derive(Debug, Clone, PartialEq)]
pub struct M2RibbonEmitter {
    /// Ribbon emitter ID (used for lookup).
    pub id: u32,
    /// Bone index this ribbon is attached to.
    pub bone_index: u16,
    /// Position offset relative to the bone, in WoW coordinates.
    pub position: [f32; 3],
    /// Number of texture references.
    pub texture_count: u32,
    /// Number of material references.
    pub material_count: u32,
    /// Edges generated per second.
    pub edges_per_second: f32,
    /// Lifetime of each edge in seconds.
    pub edge_lifetime: f32,
    /// Gravity applied to ribbon edges.
    pub gravity: f32,
    /// Texture tile rows.
    pub texture_rows: u16,
    /// Texture tile columns.
    pub texture_cols: u16,
}

/// Parse all ribbon emitters from the MD20 block.
pub fn parse_ribbon_emitters(md20: &[u8]) -> Vec<M2RibbonEmitter> {
    let Ok((count, offset)) = read_m2_array_header(md20, MD20_RIBBON_EMITTERS_COUNT_OFFSET) else {
        return Vec::new();
    };
    (0..count)
        .filter_map(|i| parse_single_ribbon(md20, offset + i * RIBBON_ENTRY_STRIDE))
        .collect()
}

fn parse_single_ribbon(md20: &[u8], base: usize) -> Option<M2RibbonEmitter> {
    if base + RIBBON_ENTRY_STRIDE > md20.len() {
        return None;
    }
    let id = read_u32(md20, base + RIBBON_ID_OFFSET).ok()?;
    let bone_index = read_u16(md20, base + RIBBON_BONE_INDEX_OFFSET).ok()?;
    let px = read_f32(md20, base + RIBBON_POSITION_OFFSET).ok()?;
    let py = read_f32(md20, base + RIBBON_POSITION_OFFSET + 4).ok()?;
    let pz = read_f32(md20, base + RIBBON_POSITION_OFFSET + 8).ok()?;
    let texture_count = read_u32(md20, base + RIBBON_TEXTURE_COUNT_OFFSET).ok()?;
    let material_count = read_u32(md20, base + RIBBON_MATERIAL_COUNT_OFFSET).ok()?;
    let edges_per_second = read_f32(md20, base + RIBBON_EDGES_PER_SEC_OFFSET).ok()?;
    let edge_lifetime = read_f32(md20, base + RIBBON_EDGE_LIFETIME_OFFSET).ok()?;
    let gravity = read_f32(md20, base + RIBBON_GRAVITY_OFFSET).ok()?;
    let texture_rows = read_u16(md20, base + RIBBON_TEXTURE_ROWS_OFFSET).ok()?;
    let texture_cols = read_u16(md20, base + RIBBON_TEXTURE_COLS_OFFSET).ok()?;
    Some(M2RibbonEmitter {
        id,
        bone_index,
        position: [px, py, pz],
        texture_count,
        material_count,
        edges_per_second,
        edge_lifetime,
        gravity,
        texture_rows,
        texture_cols,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_u32(buf: &mut [u8], offset: usize, val: u32) {
        buf[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
    }

    fn write_u16(buf: &mut [u8], offset: usize, val: u16) {
        buf[offset..offset + 2].copy_from_slice(&val.to_le_bytes());
    }

    fn write_f32(buf: &mut [u8], offset: usize, val: f32) {
        buf[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
    }

    fn make_md20_with_ribbons(ribbon_count: u32) -> Vec<u8> {
        // MD20 header needs at least 0x130 bytes + ribbon data
        let ribbon_data_offset = 0x130u32;
        let ribbon_data_size = ribbon_count as usize * RIBBON_ENTRY_STRIDE;
        let total = ribbon_data_offset as usize + ribbon_data_size;
        let mut buf = vec![0u8; total];
        // Write ribbon emitter array header at offset 0x120
        write_u32(&mut buf, MD20_RIBBON_EMITTERS_COUNT_OFFSET, ribbon_count);
        write_u32(
            &mut buf,
            MD20_RIBBON_EMITTERS_COUNT_OFFSET + 4,
            ribbon_data_offset,
        );
        buf
    }

    fn fill_ribbon(buf: &mut [u8], ribbon_base: usize) {
        write_u32(buf, ribbon_base + RIBBON_ID_OFFSET, 42);
        write_u16(buf, ribbon_base + RIBBON_BONE_INDEX_OFFSET, 5);
        write_f32(buf, ribbon_base + RIBBON_POSITION_OFFSET, 1.0);
        write_f32(buf, ribbon_base + RIBBON_POSITION_OFFSET + 4, 2.0);
        write_f32(buf, ribbon_base + RIBBON_POSITION_OFFSET + 8, 3.0);
        write_u32(buf, ribbon_base + RIBBON_TEXTURE_COUNT_OFFSET, 1);
        write_u32(buf, ribbon_base + RIBBON_MATERIAL_COUNT_OFFSET, 1);
        write_f32(buf, ribbon_base + RIBBON_EDGES_PER_SEC_OFFSET, 15.0);
        write_f32(buf, ribbon_base + RIBBON_EDGE_LIFETIME_OFFSET, 0.5);
        write_f32(buf, ribbon_base + RIBBON_GRAVITY_OFFSET, -9.8);
        write_u16(buf, ribbon_base + RIBBON_TEXTURE_ROWS_OFFSET, 1);
        write_u16(buf, ribbon_base + RIBBON_TEXTURE_COLS_OFFSET, 4);
    }

    #[test]
    fn parse_zero_ribbons() {
        let md20 = make_md20_with_ribbons(0);
        let ribbons = parse_ribbon_emitters(&md20);
        assert!(ribbons.is_empty());
    }

    #[test]
    fn parse_one_ribbon() {
        let mut md20 = make_md20_with_ribbons(1);
        fill_ribbon(&mut md20, 0x130);
        let ribbons = parse_ribbon_emitters(&md20);
        assert_eq!(ribbons.len(), 1);
        let r = &ribbons[0];
        assert_eq!(r.id, 42);
        assert_eq!(r.bone_index, 5);
        assert_eq!(r.position, [1.0, 2.0, 3.0]);
        assert_eq!(r.texture_count, 1);
        assert_eq!(r.material_count, 1);
        assert!((r.edges_per_second - 15.0).abs() < 0.01);
        assert!((r.edge_lifetime - 0.5).abs() < 0.01);
        assert!((r.gravity - (-9.8)).abs() < 0.01);
        assert_eq!(r.texture_rows, 1);
        assert_eq!(r.texture_cols, 4);
    }

    #[test]
    fn parse_multiple_ribbons() {
        let mut md20 = make_md20_with_ribbons(2);
        fill_ribbon(&mut md20, 0x130);
        // Second ribbon at stride offset
        write_u32(
            &mut md20,
            0x130 + RIBBON_ENTRY_STRIDE + RIBBON_ID_OFFSET,
            99,
        );
        write_u16(
            &mut md20,
            0x130 + RIBBON_ENTRY_STRIDE + RIBBON_BONE_INDEX_OFFSET,
            10,
        );
        write_f32(
            &mut md20,
            0x130 + RIBBON_ENTRY_STRIDE + RIBBON_EDGES_PER_SEC_OFFSET,
            30.0,
        );
        let ribbons = parse_ribbon_emitters(&md20);
        assert_eq!(ribbons.len(), 2);
        assert_eq!(ribbons[0].id, 42);
        assert_eq!(ribbons[1].id, 99);
        assert_eq!(ribbons[1].bone_index, 10);
    }

    #[test]
    fn parse_truncated_data_skips() {
        // MD20 header says 1 ribbon but data is too short
        let mut md20 = vec![0u8; 0x130 + 10]; // only 10 bytes of ribbon data
        write_u32(&mut md20, MD20_RIBBON_EMITTERS_COUNT_OFFSET, 1);
        write_u32(&mut md20, MD20_RIBBON_EMITTERS_COUNT_OFFSET + 4, 0x130);
        let ribbons = parse_ribbon_emitters(&md20);
        assert!(ribbons.is_empty());
    }

    #[test]
    fn parse_no_header_returns_empty() {
        let md20 = vec![0u8; 16]; // too small for header
        let ribbons = parse_ribbon_emitters(&md20);
        assert!(ribbons.is_empty());
    }

    #[test]
    fn ribbon_entry_stride_is_176() {
        assert_eq!(RIBBON_ENTRY_STRIDE, 176);
    }

    #[test]
    fn ribbon_struct_default_fields() {
        let r = M2RibbonEmitter {
            id: 0,
            bone_index: 0,
            position: [0.0; 3],
            texture_count: 0,
            material_count: 0,
            edges_per_second: 0.0,
            edge_lifetime: 0.0,
            gravity: 0.0,
            texture_rows: 1,
            texture_cols: 1,
        };
        assert_eq!(r.id, 0);
        assert_eq!(r.position, [0.0, 0.0, 0.0]);
    }
}
