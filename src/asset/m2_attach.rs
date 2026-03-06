//! M2 attachment point parser.
//!
//! MD20 header M2Array offsets:
//!   0xD0: attachments (count + offset) → M2Attachment[n] (40 bytes each)
//!   0xD8: attachment_lookup (count + offset) → i16[n]
//!
//! M2Attachment layout (40 bytes):
//!   0x00: id (u32) — attachment lookup ID
//!   0x04: bone (u16) — bone index
//!   0x06: unknown (u16)
//!   0x08: position [f32; 3] — offset relative to bone, WoW coordinates
//!   0x14: AnimBlock (20 bytes, skipped)

use super::m2::{read_f32, read_u16, read_u32};

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

/// Parse M2Attachment entries from MD20 offset 0xD0.
pub fn parse_attachments(md20: &[u8]) -> Result<Vec<M2Attachment>, String> {
    if md20.len() < 0xD8 {
        return Ok(Vec::new());
    }
    let count = read_u32(md20, 0xD0)? as usize;
    let offset = read_u32(md20, 0xD4)? as usize;
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

/// Parse attachment lookup table from MD20 offset 0xD8 (array of i16).
pub fn parse_attachment_lookup(md20: &[u8]) -> Result<Vec<i16>, String> {
    if md20.len() < 0xE0 {
        return Ok(Vec::new());
    }
    let count = read_u32(md20, 0xD8)? as usize;
    let offset = read_u32(md20, 0xDC)? as usize;
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
        let mut off = 0;
        while off + 8 <= data.len() {
            let tag = &data[off..off + 4];
            let size = u32::from_le_bytes(data[off + 4..off + 8].try_into().unwrap()) as usize;
            let end = off + 8 + size;
            if end > data.len() {
                break;
            }
            if tag == b"MD21" {
                return Some(&data[off + 8..end]);
            }
            off = end;
        }
        None
    }
}
