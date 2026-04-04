//! M2 attachment point parser.
//!
//! MD20 header M2Array offsets:
//!   0xD8: attachments (count + offset) → M2Attachment[n] (40 bytes each)
//!   0xE0: attachment_lookup (count + offset) → i16[n]
//!
//! M2Attachment layout (40 bytes):
//!   0x00: id (u32) — attachment lookup ID
//!   0x04: bone (u16) — bone index
//!   0x06: unknown (u16)
//!   0x08: position [f32; 3] — offset relative to bone, WoW coordinates
//!   0x14: AnimBlock (20 bytes, skipped)

use super::{
    MD20_ATTACHMENT_LOOKUP_COUNT_OFFSET, MD20_ATTACHMENTS_COUNT_OFFSET, read_f32, read_u16,
    read_u32,
};
use crate::asset::read_bytes::read_m2_array_header;

const ATTACHMENT_ENTRY_SIZE: usize = 40;
const ATTACHMENT_PARSED_SIZE: usize = 20;
const ATTACHMENT_ID_OFFSET: usize = 0;
const ATTACHMENT_BONE_OFFSET: usize = 4;
const ATTACHMENT_POSITION_OFFSET: usize = 8;
const ATTACHMENT_LOOKUP_ENTRY_SIZE: usize = 2;

/// An attachment point on an M2 model (e.g., hand, back, shoulder).
#[derive(Debug, Clone)]
pub struct M2Attachment {
    /// Attachment lookup ID (0=HandRight, 1=HandLeft, etc.).
    pub id: u32,
    /// Index of the bone this attachment is parented to.
    pub bone: u16,
    /// Position offset relative to the bone, in WoW coordinates.
    pub position: [f32; 3],
}

/// Parse M2Attachment entries from MD20 offset 0xD8.
pub fn parse_attachments(md20: &[u8]) -> Result<Vec<M2Attachment>, String> {
    let (count, offset) = read_m2_array_header(md20, MD20_ATTACHMENTS_COUNT_OFFSET)?;
    let mut attachments = Vec::with_capacity(count);
    for i in 0..count {
        let base = offset + i * ATTACHMENT_ENTRY_SIZE;
        if base + ATTACHMENT_PARSED_SIZE > md20.len() {
            return Err(format!("Attachment {i} out of bounds at {base:#x}"));
        }
        attachments.push(M2Attachment {
            id: read_u32(md20, base + ATTACHMENT_ID_OFFSET)?,
            bone: read_u16(md20, base + ATTACHMENT_BONE_OFFSET)?,
            position: [
                read_f32(md20, base + ATTACHMENT_POSITION_OFFSET)?,
                read_f32(md20, base + ATTACHMENT_POSITION_OFFSET + 4)?,
                read_f32(md20, base + ATTACHMENT_POSITION_OFFSET + 8)?,
            ],
        });
    }
    Ok(attachments)
}

pub fn parse_ska1_attachments(ska1: &[u8]) -> Result<Vec<M2Attachment>, String> {
    if ska1.len() < 16 {
        return Ok(Vec::new());
    }
    let count = read_u32(ska1, 0)? as usize;
    let offset = read_u32(ska1, 4)? as usize;
    let mut attachments = Vec::with_capacity(count);
    for i in 0..count {
        let base = offset + i * ATTACHMENT_ENTRY_SIZE;
        if base + ATTACHMENT_PARSED_SIZE > ska1.len() {
            return Err(format!("SKA1 attachment {i} out of bounds at {base:#x}"));
        }
        attachments.push(M2Attachment {
            id: read_u32(ska1, base + ATTACHMENT_ID_OFFSET)?,
            bone: read_u16(ska1, base + ATTACHMENT_BONE_OFFSET)?,
            position: [
                read_f32(ska1, base + ATTACHMENT_POSITION_OFFSET)?,
                read_f32(ska1, base + ATTACHMENT_POSITION_OFFSET + 4)?,
                read_f32(ska1, base + ATTACHMENT_POSITION_OFFSET + 8)?,
            ],
        });
    }
    Ok(attachments)
}

/// Parse attachment lookup table from MD20 offset 0xE0 (array of i16).
pub fn parse_attachment_lookup(md20: &[u8]) -> Result<Vec<i16>, String> {
    let (count, offset) = read_m2_array_header(md20, MD20_ATTACHMENT_LOOKUP_COUNT_OFFSET)?;
    let mut lookup = Vec::with_capacity(count);
    for i in 0..count {
        let off = offset + i * ATTACHMENT_LOOKUP_ENTRY_SIZE;
        if off + 2 > md20.len() {
            break;
        }
        lookup.push(read_u16(md20, off)? as i16);
    }
    Ok(lookup)
}

pub fn parse_ska1_attachment_lookup(ska1: &[u8]) -> Result<Vec<i16>, String> {
    if ska1.len() < 16 {
        return Ok(Vec::new());
    }
    let count = read_u32(ska1, 8)? as usize;
    let offset = read_u32(ska1, 12)? as usize;
    let mut lookup = Vec::with_capacity(count);
    for i in 0..count {
        let off = offset + i * ATTACHMENT_LOOKUP_ENTRY_SIZE;
        if off + 2 > ska1.len() {
            break;
        }
        lookup.push(read_u16(ska1, off)? as i16);
    }
    Ok(lookup)
}

#[cfg(test)]
#[path = "../../../tests/unit/asset/m2_attach_tests.rs"]
mod tests;
