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

use super::m2_format::{
    MD20_ATTACHMENT_LOOKUP_COUNT_OFFSET, MD20_ATTACHMENT_LOOKUP_DATA_OFFSET,
    MD20_ATTACHMENTS_COUNT_OFFSET, MD20_ATTACHMENTS_DATA_OFFSET, read_f32, read_u16, read_u32,
};

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
    if md20.len() < MD20_ATTACHMENTS_DATA_OFFSET + 4 {
        return Ok(Vec::new());
    }
    let count = read_u32(md20, MD20_ATTACHMENTS_COUNT_OFFSET)? as usize;
    let offset = read_u32(md20, MD20_ATTACHMENTS_DATA_OFFSET)? as usize;
    let mut attachments = Vec::with_capacity(count);
    for i in 0..count {
        let base = offset + i * 40;
        if base + 20 > md20.len() {
            return Err(format!("Attachment {i} out of bounds at {base:#x}"));
        }
        attachments.push(M2Attachment {
            id: read_u32(md20, base)?,
            bone: read_u16(md20, base + 4)?,
            position: [
                read_f32(md20, base + 8)?,
                read_f32(md20, base + 12)?,
                read_f32(md20, base + 16)?,
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
        let base = offset + i * 40;
        if base + 20 > ska1.len() {
            return Err(format!("SKA1 attachment {i} out of bounds at {base:#x}"));
        }
        attachments.push(M2Attachment {
            id: read_u32(ska1, base)?,
            bone: read_u16(ska1, base + 4)?,
            position: [
                read_f32(ska1, base + 8)?,
                read_f32(ska1, base + 12)?,
                read_f32(ska1, base + 16)?,
            ],
        });
    }
    Ok(attachments)
}

/// Parse attachment lookup table from MD20 offset 0xE0 (array of i16).
pub fn parse_attachment_lookup(md20: &[u8]) -> Result<Vec<i16>, String> {
    if md20.len() < MD20_ATTACHMENT_LOOKUP_DATA_OFFSET + 4 {
        return Ok(Vec::new());
    }
    let count = read_u32(md20, MD20_ATTACHMENT_LOOKUP_COUNT_OFFSET)? as usize;
    let offset = read_u32(md20, MD20_ATTACHMENT_LOOKUP_DATA_OFFSET)? as usize;
    let mut lookup = Vec::with_capacity(count);
    for i in 0..count {
        let off = offset + i * 2;
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
        let off = offset + i * 2;
        if off + 2 > ska1.len() {
            break;
        }
        lookup.push(read_u16(ska1, off)? as i16);
    }
    Ok(lookup)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_humanmale_hd_attachments() {
        let path = std::path::Path::new("data/models/humanmale_hd.m2");
        if !path.exists() {
            return;
        }
        let data = std::fs::read(path).unwrap();
        // Find MD21 chunk
        let md20 = find_md20(&data).expect("no MD21 chunk");
        let attachments = parse_attachments(md20).unwrap();
        assert!(!attachments.is_empty(), "HD human should have attachments");

        // Should have a right hand attachment (id=0)
        let right_hand = attachments.iter().find(|a| a.id == 0);
        assert!(
            right_hand.is_some(),
            "Should have right hand attachment (id=0)"
        );
        let rh = right_hand.unwrap();
        assert!(rh.bone < 200, "Bone index should be reasonable");

        let lookup = parse_attachment_lookup(md20).unwrap();
        assert!(!lookup.is_empty(), "Should have attachment lookup");
        println!(
            "humanmale_hd MD21 attachments ids={:?} lookup11={:?} lookup20={:?}",
            attachments.iter().map(|a| a.id).collect::<Vec<_>>(),
            lookup.get(11),
            lookup.get(20)
        );
        if let Some(ska1) = find_chunk(&data, b"SKA1") {
            let ska1_attachments = parse_ska1_attachments(ska1).unwrap();
            let ska1_lookup = parse_ska1_attachment_lookup(ska1).unwrap();
            println!(
                "humanmale_hd SKA1 attachments ids={:?} lookup11={:?} lookup20={:?}",
                ska1_attachments.iter().map(|a| a.id).collect::<Vec<_>>(),
                ska1_lookup.get(11),
                ska1_lookup.get(20)
            );
        }
    }

    #[test]
    fn parse_torch_attachments() {
        let path = std::path::Path::new("data/models/club_1h_torch_a_01.m2");
        if !path.exists() {
            return;
        }
        let data = std::fs::read(path).unwrap();
        let md20 = find_md20(&data).expect("no MD21 chunk");
        let attachments = parse_attachments(md20).unwrap();
        // Item models may or may not have attachments — just ensure no crash
        let lookup = parse_attachment_lookup(md20).unwrap();
        let _ = (attachments, lookup);
    }

    fn find_md20(data: &[u8]) -> Option<&[u8]> {
        find_chunk(data, b"MD21")
    }

    fn find_chunk<'a>(data: &'a [u8], needle: &[u8; 4]) -> Option<&'a [u8]> {
        let mut off = 0;
        while off + 8 <= data.len() {
            let tag = &data[off..off + 4];
            let size = u32::from_le_bytes(data[off + 4..off + 8].try_into().unwrap()) as usize;
            let end = off + 8 + size;
            if end > data.len() {
                break;
            }
            if tag == needle {
                return Some(&data[off + 8..end]);
            }
            off = end;
        }
        None
    }
}
