//! M2 bone data parser (Phase 1: hierarchy + pivot only, no AnimBlock content).
//!
//! MD20 header M2Array offsets (8-byte M2Array = count u32 + offset u32):
//!   0x08: model name
//!   0x14: global loops
//!   0x1C: sequences
//!   0x24: sequence lookups
//!   0x2C: bones          <- count at 0x2C, data offset at 0x30
//!   0x34: key bone lookup
//!   0x3C: vertices
//!
//! Bone layout (CompBone, 88 bytes / 0x58):
//!   0x00: key_bone_id (i32)
//!   0x04: flags (u32)
//!   0x08: parent_bone_id (i16)
//!   0x0A: submesh_id (u16)
//!   0x0C: bone_name_crc (u32)
//!   0x10: translation AnimBlock (20 bytes, skipped)
//!   0x24: rotation AnimBlock (20 bytes, skipped)
//!   0x38: scale AnimBlock (20 bytes, skipped)
//!   0x4C: pivot [f32; 3] (12 bytes, WoW coordinates)

fn read_u32(data: &[u8], off: usize) -> Result<u32, String> {
    let bytes: [u8; 4] = data
        .get(off..off + 4)
        .ok_or_else(|| format!("read_u32 out of bounds at offset {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(u32::from_le_bytes(bytes))
}

fn read_u16(data: &[u8], off: usize) -> Result<u16, String> {
    let bytes: [u8; 2] = data
        .get(off..off + 2)
        .ok_or_else(|| format!("read_u16 out of bounds at offset {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(u16::from_le_bytes(bytes))
}

fn read_i16(data: &[u8], off: usize) -> Result<i16, String> {
    let bytes: [u8; 2] = data
        .get(off..off + 2)
        .ok_or_else(|| format!("read_i16 out of bounds at offset {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(i16::from_le_bytes(bytes))
}

fn read_f32(data: &[u8], off: usize) -> Result<f32, String> {
    let bytes: [u8; 4] = data
        .get(off..off + 4)
        .ok_or_else(|| format!("read_f32 out of bounds at offset {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(f32::from_le_bytes(bytes))
}

fn read_i32(data: &[u8], off: usize) -> Result<i32, String> {
    let bytes: [u8; 4] = data
        .get(off..off + 4)
        .ok_or_else(|| format!("read_i32 out of bounds at offset {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(i32::from_le_bytes(bytes))
}

pub struct M2Bone {
    pub key_bone_id: i32,
    pub flags: u32,
    pub parent_bone_id: i16,
    pub submesh_id: u16,
    /// Pivot point in raw WoW coordinates. Caller converts to Bevy: [x, z, -y].
    pub pivot: [f32; 3],
}

/// Parse the bones M2Array from the MD20 blob at offset 0x2C.
/// Returns bones with parent indices and pivot points.
pub fn parse_bones(md20: &[u8]) -> Result<Vec<M2Bone>, String> {
    if md20.len() < 0x34 {
        return Ok(Vec::new());
    }
    let count = read_u32(md20, 0x2C)? as usize;
    let offset = read_u32(md20, 0x30)? as usize;

    const BONE_SIZE: usize = 88; // 0x58 bytes per CompBone

    let mut bones = Vec::with_capacity(count);
    for i in 0..count {
        let base = offset + i * BONE_SIZE;
        if base + BONE_SIZE > md20.len() {
            return Err(format!("Bone {i} out of bounds at offset {base:#x}"));
        }
        bones.push(M2Bone {
            key_bone_id: read_i32(md20, base)?,
            flags: read_u32(md20, base + 0x04)?,
            parent_bone_id: read_i16(md20, base + 0x08)?,
            submesh_id: read_u16(md20, base + 0x0A)?,
            pivot: [
                read_f32(md20, base + 0x4C)?,
                read_f32(md20, base + 0x50)?,
                read_f32(md20, base + 0x54)?,
            ],
        });
    }
    Ok(bones)
}

/// Verify bone parent chain: all parent_bone_id values are either -1 (root) or
/// a valid index less than the total bone count (parent-first ordering expected
/// by WoW M2 files but not strictly enforced here).
pub fn validate_bone_hierarchy(bones: &[M2Bone]) -> Result<(), String> {
    for (i, bone) in bones.iter().enumerate() {
        if bone.parent_bone_id >= 0 {
            let parent = bone.parent_bone_id as usize;
            if parent >= bones.len() {
                return Err(format!(
                    "Bone {i} parent {parent} >= bone count {}",
                    bones.len()
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal MD20 blob with `n` bones at offset 0x34.
    fn md20_with_bones(bones: &[(i32, u32, i16, u16, [f32; 3])]) -> Vec<u8> {
        let bone_offset: u32 = 0x48; // right after the minimum header
        let bone_count = bones.len() as u32;
        let total = bone_offset as usize + bones.len() * 88;
        let mut md20 = vec![0u8; total];

        md20[0..4].copy_from_slice(b"MD20");
        md20[4..8].copy_from_slice(&264u32.to_le_bytes());
        // bones M2Array at 0x2C
        md20[0x2C..0x30].copy_from_slice(&bone_count.to_le_bytes());
        md20[0x30..0x34].copy_from_slice(&bone_offset.to_le_bytes());

        for (i, &(key_bone, flags, parent, submesh, pivot)) in bones.iter().enumerate() {
            let base = bone_offset as usize + i * 88;
            md20[base..base + 4].copy_from_slice(&key_bone.to_le_bytes());
            md20[base + 4..base + 8].copy_from_slice(&flags.to_le_bytes());
            md20[base + 8..base + 10].copy_from_slice(&parent.to_le_bytes());
            md20[base + 10..base + 12].copy_from_slice(&submesh.to_le_bytes());
            // pivot at offset 0x4C within bone
            md20[base + 0x4C..base + 0x50].copy_from_slice(&pivot[0].to_le_bytes());
            md20[base + 0x50..base + 0x54].copy_from_slice(&pivot[1].to_le_bytes());
            md20[base + 0x54..base + 0x58].copy_from_slice(&pivot[2].to_le_bytes());
        }

        md20
    }

    #[test]
    fn parse_zero_bones() {
        // MD20 with count=0 at offset 0x2C
        let mut md20 = vec![0u8; 0x48];
        md20[0..4].copy_from_slice(b"MD20");
        md20[0x2C..0x30].copy_from_slice(&0u32.to_le_bytes());
        md20[0x30..0x34].copy_from_slice(&0x48u32.to_le_bytes());
        let bones = parse_bones(&md20).unwrap();
        assert!(bones.is_empty());
    }

    #[test]
    fn parse_single_root_bone() {
        let md20 = md20_with_bones(&[
            (-1, 0, -1, 0, [1.0, 2.0, 3.0]),
        ]);
        let bones = parse_bones(&md20).unwrap();
        assert_eq!(bones.len(), 1);
        assert_eq!(bones[0].parent_bone_id, -1);
        assert_eq!(bones[0].pivot, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn parse_bone_hierarchy() {
        let md20 = md20_with_bones(&[
            (0, 0, -1, 0, [0.0, 0.0, 0.0]),  // root
            (1, 0, 0, 0, [1.0, 0.0, 0.0]),    // child of root
            (2, 0, 1, 0, [2.0, 0.0, 0.0]),    // child of bone 1
        ]);
        let bones = parse_bones(&md20).unwrap();
        assert_eq!(bones.len(), 3);
        assert_eq!(bones[0].parent_bone_id, -1);
        assert_eq!(bones[1].parent_bone_id, 0);
        assert_eq!(bones[2].parent_bone_id, 1);
        assert!(validate_bone_hierarchy(&bones).is_ok());
    }

    #[test]
    fn validate_detects_invalid_parent() {
        let md20 = md20_with_bones(&[
            (0, 0, 5, 0, [0.0, 0.0, 0.0]),  // parent=5, but only 1 bone
        ]);
        let bones = parse_bones(&md20).unwrap();
        assert!(validate_bone_hierarchy(&bones).is_err());
    }

    #[test]
    fn parse_humanmale_bones() {
        let m2_path = "data/models/humanmale.m2";
        let data = match std::fs::read(m2_path) {
            Ok(d) => d,
            Err(_) => { println!("Skipping: {m2_path} not found"); return; }
        };
        // Parse chunks to get MD20
        let mut md20 = None;
        let mut off = 0;
        while off + 8 <= data.len() {
            let size = u32::from_le_bytes(data[off + 4..off + 8].try_into().unwrap()) as usize;
            if &data[off..off + 4] == b"MD21" {
                md20 = Some(&data[off + 8..off + 8 + size]);
                break;
            }
            off += 8 + size;
        }
        let md20 = md20.expect("MD21 chunk not found");
        let bones = parse_bones(md20).unwrap();

        // humanmale.m2 should have bones
        assert!(!bones.is_empty(), "humanmale should have bones, got 0");
        println!("humanmale: {} bones", bones.len());

        // Validate hierarchy
        assert!(validate_bone_hierarchy(&bones).is_ok());

        // At least one root bone
        assert!(bones.iter().any(|b| b.parent_bone_id == -1), "Should have at least one root bone");
    }

    #[test]
    fn parse_humanmale_hd_bones() {
        let m2_path = "data/models/humanmale_hd.m2";
        let data = match std::fs::read(m2_path) {
            Ok(d) => d,
            Err(_) => { println!("Skipping: {m2_path} not found"); return; }
        };
        let mut md20 = None;
        let mut off = 0;
        while off + 8 <= data.len() {
            let size = u32::from_le_bytes(data[off + 4..off + 8].try_into().unwrap()) as usize;
            if &data[off..off + 4] == b"MD21" {
                md20 = Some(&data[off + 8..off + 8 + size]);
                break;
            }
            off += 8 + size;
        }
        let md20 = md20.expect("MD21 chunk not found");
        let bones = parse_bones(md20).unwrap();
        println!("humanmale_hd: {} bones", bones.len());
        if !bones.is_empty() {
            assert!(validate_bone_hierarchy(&bones).is_ok());
        }
    }
}
