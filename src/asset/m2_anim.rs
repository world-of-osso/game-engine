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

/// Parse `count` CompBone entries starting at `offset` in `data`.
pub fn parse_bones_at(data: &[u8], offset: usize, count: usize) -> Result<Vec<M2Bone>, String> {
    const BONE_SIZE: usize = 88; // 0x58 bytes per CompBone

    let mut bones = Vec::with_capacity(count);
    for i in 0..count {
        let base = offset + i * BONE_SIZE;
        if base + BONE_SIZE > data.len() {
            return Err(format!("Bone {i} out of bounds at offset {base:#x}"));
        }
        bones.push(M2Bone {
            key_bone_id: read_i32(data, base)?,
            flags: read_u32(data, base + 0x04)?,
            parent_bone_id: read_i16(data, base + 0x08)?,
            submesh_id: read_u16(data, base + 0x0A)?,
            pivot: [
                read_f32(data, base + 0x4C)?,
                read_f32(data, base + 0x50)?,
                read_f32(data, base + 0x54)?,
            ],
        });
    }
    Ok(bones)
}

/// Parse the bones M2Array from the MD20 blob at offset 0x2C.
/// Returns bones with parent indices and pivot points.
pub fn parse_bones(md20: &[u8]) -> Result<Vec<M2Bone>, String> {
    if md20.len() < 0x34 {
        return Ok(Vec::new());
    }
    let count = read_u32(md20, 0x2C)? as usize;
    let offset = read_u32(md20, 0x30)? as usize;
    parse_bones_at(md20, offset, count)
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

pub struct M2AnimSequence {
    pub id: u16,
    pub variation_id: u16,
    pub duration: u32,  // milliseconds
    pub movespeed: f32,
    pub flags: u32,
    pub blend_time: u16,      // milliseconds, for transitions
    pub next_animation: i16,  // -1 = none, else index into sequences
}

/// Parse the sequences M2Array from the MD20 blob at offset 0x1C.
///
/// Each sequence entry is 64 bytes. Relevant fields:
///   0x00 u16  id
///   0x02 u16  variation_id
///   0x04 u32  duration (ms)
///   0x08 f32  movespeed
///   0x0C u32  flags
///   0x1C u16  blend_time
///   0x3C i16  next_animation (-1 = none)
pub fn parse_sequences(md20: &[u8]) -> Result<Vec<M2AnimSequence>, String> {
    if md20.len() < 0x24 {
        return Ok(Vec::new());
    }
    let count = read_u32(md20, 0x1C)? as usize;
    let offset = read_u32(md20, 0x20)? as usize;

    const SEQ_SIZE: usize = 64;

    let mut sequences = Vec::with_capacity(count);
    for i in 0..count {
        let base = offset + i * SEQ_SIZE;
        if base + SEQ_SIZE > md20.len() {
            return Err(format!("Sequence {i} out of bounds at offset {base:#x}"));
        }
        sequences.push(M2AnimSequence {
            id: read_u16(md20, base)?,
            variation_id: read_u16(md20, base + 0x02)?,
            duration: read_u32(md20, base + 0x04)?,
            movespeed: read_f32(md20, base + 0x08)?,
            flags: read_u32(md20, base + 0x0C)?,
            blend_time: read_u16(md20, base + 0x1C)?,
            next_animation: read_i16(md20, base + 0x3C)?,
        });
    }
    Ok(sequences)
}

/// Parse the global sequences M2Array from the MD20 blob at offset 0x14.
///
/// Global sequences are simple u32 durations (milliseconds) used for looping
/// animations that run independently of any specific animation sequence.
pub fn parse_global_sequences(md20: &[u8]) -> Result<Vec<u32>, String> {
    if md20.len() < 0x1C {
        return Ok(Vec::new());
    }
    let count = read_u32(md20, 0x14)? as usize;
    let offset = read_u32(md20, 0x18)? as usize;

    let mut durations = Vec::with_capacity(count);
    for i in 0..count {
        durations.push(read_u32(md20, offset + i * 4)?);
    }
    Ok(durations)
}

/// A single animation track: keyframes for one transform component of one bone.
/// `sequences[i]` holds the keyframe data for animation sequence `i`.
pub struct AnimTrack<T> {
    pub interpolation_type: u16,
    pub global_sequence: i16,
    /// Per-sequence keyframe data: (timestamps_ms, values).
    pub sequences: Vec<(Vec<u32>, Vec<T>)>,
}

/// All animation tracks for one bone.
pub struct BoneAnimTracks {
    pub translation: AnimTrack<[f32; 3]>,
    pub rotation: AnimTrack<[i16; 4]>,
    pub scale: AnimTrack<[f32; 3]>,
}

fn parse_vec3(md20: &[u8], off: usize) -> Result<[f32; 3], String> {
    Ok([read_f32(md20, off)?, read_f32(md20, off + 4)?, read_f32(md20, off + 8)?])
}

fn parse_quat_packed(md20: &[u8], off: usize) -> Result<[i16; 4], String> {
    Ok([
        read_i16(md20, off)?,
        read_i16(md20, off + 2)?,
        read_i16(md20, off + 4)?,
        read_i16(md20, off + 6)?,
    ])
}

/// Parse an AnimBlock's nested M2Array structure.
/// `block_offset` is the offset of the AnimBlock within the MD20 blob.
/// `value_size` is the byte size of each keyframe value.
/// `parse_value` converts raw bytes at a given offset to a value of type T.
fn parse_anim_track<T: Copy>(
    md20: &[u8],
    block_offset: usize,
    value_size: usize,
    parse_value: impl Fn(&[u8], usize) -> Result<T, String>,
) -> Result<AnimTrack<T>, String> {
    let interp = read_u16(md20, block_offset)?;
    let global_seq = read_i16(md20, block_offset + 2)?;

    // Outer M2Array: timestamps
    let ts_outer_count = read_u32(md20, block_offset + 4)? as usize;
    let ts_outer_offset = read_u32(md20, block_offset + 8)? as usize;

    // Outer M2Array: keys
    let keys_outer_count = read_u32(md20, block_offset + 12)? as usize;
    let keys_outer_offset = read_u32(md20, block_offset + 16)? as usize;

    // Both outer arrays should have the same count (one per sequence)
    let count = ts_outer_count.min(keys_outer_count);

    let mut sequences = Vec::with_capacity(count);
    for i in 0..count {
        let timestamps = read_inner_u32_array(md20, ts_outer_offset + i * 8)?;
        let values = read_inner_value_array(md20, keys_outer_offset + i * 8, value_size, &parse_value)?;
        sequences.push((timestamps, values));
    }

    Ok(AnimTrack {
        interpolation_type: interp,
        global_sequence: global_seq,
        sequences,
    })
}

fn read_inner_u32_array(md20: &[u8], inner_off: usize) -> Result<Vec<u32>, String> {
    let count = read_u32(md20, inner_off)? as usize;
    let data_off = read_u32(md20, inner_off + 4)? as usize;
    let mut out = Vec::with_capacity(count);
    for j in 0..count {
        out.push(read_u32(md20, data_off + j * 4)?);
    }
    Ok(out)
}

fn read_inner_value_array<T: Copy>(
    md20: &[u8],
    inner_off: usize,
    value_size: usize,
    parse_value: impl Fn(&[u8], usize) -> Result<T, String>,
) -> Result<Vec<T>, String> {
    let count = read_u32(md20, inner_off)? as usize;
    let data_off = read_u32(md20, inner_off + 4)? as usize;
    let mut out = Vec::with_capacity(count);
    for j in 0..count {
        out.push(parse_value(md20, data_off + j * value_size)?);
    }
    Ok(out)
}

/// Parse animation tracks for all bones from the MD20 blob.
/// Returns one BoneAnimTracks per bone, in the same order as parse_bones.
pub fn parse_bone_animations(md20: &[u8]) -> Result<Vec<BoneAnimTracks>, String> {
    if md20.len() < 0x34 {
        return Ok(Vec::new());
    }
    let count = read_u32(md20, 0x2C)? as usize;
    let offset = read_u32(md20, 0x30)? as usize;

    const BONE_SIZE: usize = 88;

    let mut tracks = Vec::with_capacity(count);
    for i in 0..count {
        let base = offset + i * BONE_SIZE;
        if base + BONE_SIZE > md20.len() {
            return Err(format!("Bone {i} out of bounds at offset {base:#x}"));
        }

        let translation = parse_anim_track(md20, base + 0x10, 12, parse_vec3)?;
        let rotation = parse_anim_track(md20, base + 0x24, 8, parse_quat_packed)?;
        let scale = parse_anim_track(md20, base + 0x38, 12, parse_vec3)?;

        tracks.push(BoneAnimTracks { translation, rotation, scale });
    }
    Ok(tracks)
}

/// Unpack a WoW packed i16 quaternion to a Bevy Quat with coordinate flip.
/// WoW packs rotation as [i16; 4] (x, y, z, w).
/// Unpack: (raw < 0 ? raw + 32768 : raw - 32767) / 32767.0
/// Coordinate flip: Quat(-x, -z, y, w) to match WoW→Bevy transform.
pub fn unpack_rotation(packed: &[i16; 4]) -> [f32; 4] {
    let x = unpack_quat_component(packed[0]);
    let y = unpack_quat_component(packed[1]);
    let z = unpack_quat_component(packed[2]);
    let w = unpack_quat_component(packed[3]);
    // WoW→Bevy coordinate flip: Quat(-x, -z, y, w)
    [-x, -z, y, w]
}

fn unpack_quat_component(raw: i16) -> f32 {
    if raw < 0 {
        (raw as f32 + 32768.0) / 32767.0
    } else {
        (raw as f32 - 32767.0) / 32767.0
    }
}

/// Find the index of the last timestamp <= time_ms.
/// Returns None if timestamps is empty.
fn find_keyframe_pair(timestamps: &[u32], time_ms: u32) -> Option<(usize, f32)> {
    if timestamps.is_empty() {
        return None;
    }
    if timestamps.len() == 1 || time_ms <= timestamps[0] {
        return Some((0, 0.0));
    }
    if time_ms >= *timestamps.last().unwrap() {
        return Some((timestamps.len() - 1, 0.0));
    }
    // Binary search for the interval
    let idx = timestamps.partition_point(|&t| t <= time_ms);
    let i = idx.saturating_sub(1);
    let t0 = timestamps[i];
    let t1 = timestamps[i + 1];
    let t = if t1 > t0 {
        (time_ms - t0) as f32 / (t1 - t0) as f32
    } else {
        0.0
    };
    Some((i, t))
}

fn lerp_vec3(a: &[f32; 3], b: &[f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// Evaluate a Vec3 animation track at the given time.
/// Returns None if the track has no keyframes for this sequence.
pub fn evaluate_vec3_track(
    track: &AnimTrack<[f32; 3]>,
    seq_idx: usize,
    time_ms: u32,
) -> Option<[f32; 3]> {
    let (timestamps, values) = track.sequences.get(seq_idx)?;
    if timestamps.is_empty() || values.is_empty() {
        return None;
    }
    let (i, t) = find_keyframe_pair(timestamps, time_ms)?;
    if t == 0.0 || i + 1 >= values.len() {
        return Some(values[i]);
    }
    Some(lerp_vec3(&values[i], &values[i + 1], t))
}

/// Evaluate a rotation track at the given time, returning an unpacked [f32; 4] quaternion.
/// The quaternion is already in Bevy coordinate space.
pub fn evaluate_rotation_track(
    track: &AnimTrack<[i16; 4]>,
    seq_idx: usize,
    time_ms: u32,
) -> Option<[f32; 4]> {
    let (timestamps, values) = track.sequences.get(seq_idx)?;
    if timestamps.is_empty() || values.is_empty() {
        return None;
    }
    let (i, t) = find_keyframe_pair(timestamps, time_ms)?;
    let q0 = unpack_rotation(&values[i]);
    if t == 0.0 || i + 1 >= values.len() {
        return Some(q0);
    }
    let q1 = unpack_rotation(&values[i + 1]);
    Some(slerp(&q0, &q1, t))
}

fn slerp(a: &[f32; 4], b: &[f32; 4], t: f32) -> [f32; 4] {
    let mut dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];

    // If dot is negative, negate one quaternion to take shortest path
    let mut b = *b;
    if dot < 0.0 {
        b = [-b[0], -b[1], -b[2], -b[3]];
        dot = -dot;
    }

    // If very close, use linear interpolation to avoid division by zero
    if dot > 0.9995 {
        let result = [
            a[0] + (b[0] - a[0]) * t,
            a[1] + (b[1] - a[1]) * t,
            a[2] + (b[2] - a[2]) * t,
            a[3] + (b[3] - a[3]) * t,
        ];
        // Normalize
        let len = (result[0] * result[0]
            + result[1] * result[1]
            + result[2] * result[2]
            + result[3] * result[3])
            .sqrt();
        return [result[0] / len, result[1] / len, result[2] / len, result[3] / len];
    }

    let theta = dot.acos();
    let sin_theta = theta.sin();
    let s0 = ((1.0 - t) * theta).sin() / sin_theta;
    let s1 = (t * theta).sin() / sin_theta;

    [
        a[0] * s0 + b[0] * s1,
        a[1] * s0 + b[1] * s1,
        a[2] * s0 + b[2] * s1,
        a[3] * s0 + b[3] * s1,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Extract MD20 blob from chunked M2 data (test helper).
    fn extract_md20(data: &[u8]) -> &[u8] {
        let mut off = 0;
        while off + 8 <= data.len() {
            let size =
                u32::from_le_bytes(data[off + 4..off + 8].try_into().unwrap()) as usize;
            if &data[off..off + 4] == b"MD21" {
                return &data[off + 8..off + 8 + size];
            }
            off += 8 + size;
        }
        panic!("MD21 chunk not found");
    }

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
        let md20 = extract_md20(&data);
        let bones = parse_bones(md20).unwrap();

        assert!(!bones.is_empty(), "humanmale should have bones, got 0");
        println!("humanmale: {} bones", bones.len());
        assert!(validate_bone_hierarchy(&bones).is_ok());
        assert!(bones.iter().any(|b| b.parent_bone_id == -1), "Should have at least one root bone");
    }

    #[test]
    fn parse_humanmale_hd_bones() {
        let m2_path = "data/models/humanmale_hd.m2";
        let data = match std::fs::read(m2_path) {
            Ok(d) => d,
            Err(_) => { println!("Skipping: {m2_path} not found"); return; }
        };
        let md20 = extract_md20(&data);
        let bones = parse_bones(md20).unwrap();
        println!("humanmale_hd: {} bones", bones.len());
        if !bones.is_empty() {
            assert!(validate_bone_hierarchy(&bones).is_ok());
        }
    }

    #[test]
    fn parse_humanmale_sequences() {
        let m2_path = "data/models/humanmale.m2";
        let data = match std::fs::read(m2_path) {
            Ok(d) => d,
            Err(_) => { println!("Skipping: {m2_path} not found"); return; }
        };
        let md20 = extract_md20(&data);
        let sequences = parse_sequences(md20).unwrap();

        assert!(sequences.len() >= 100, "Expected 100+ sequences, got {}", sequences.len());

        let stand = sequences.iter().find(|s| s.id == 0);
        assert!(stand.is_some(), "Stand animation (id=0) not found");
        let stand = stand.unwrap();
        assert!(stand.duration > 0, "Stand should have non-zero duration");

        let walk = sequences.iter().find(|s| s.id == 4);
        assert!(walk.is_some(), "Walk animation (id=4) not found");
    }

    #[test]
    fn parse_humanmale_global_sequences() {
        let m2_path = "data/models/humanmale.m2";
        let data = match std::fs::read(m2_path) {
            Ok(d) => d,
            Err(_) => { println!("Skipping: {m2_path} not found"); return; }
        };
        let md20 = extract_md20(&data);
        let global_seqs = parse_global_sequences(md20).unwrap();
        println!("humanmale: {} global sequences", global_seqs.len());
        for (i, dur) in global_seqs.iter().enumerate() {
            println!("  global_seq[{i}]: {dur}ms");
        }
    }

    #[test]
    fn parse_humanmale_bone_animations() {
        let m2_path = "data/models/humanmale.m2";
        let data = match std::fs::read(m2_path) {
            Ok(d) => d,
            Err(_) => { println!("Skipping: {m2_path} not found"); return; }
        };
        let md20 = extract_md20(&data);
        let bones = parse_bones(md20).unwrap();
        let tracks = parse_bone_animations(md20).unwrap();

        assert_eq!(tracks.len(), bones.len(), "Should have one track set per bone");

        // Find the Stand animation index (id=0)
        let sequences = parse_sequences(md20).unwrap();
        let stand_idx = sequences.iter().position(|s| s.id == 0).expect("Stand not found");

        // At least some bones should have keyframes for Stand
        let bones_with_stand_keyframes = tracks.iter()
            .filter(|t| {
                t.translation.sequences.get(stand_idx).map_or(false, |(ts, _)| !ts.is_empty())
                || t.rotation.sequences.get(stand_idx).map_or(false, |(ts, _)| !ts.is_empty())
                || t.scale.sequences.get(stand_idx).map_or(false, |(ts, _)| !ts.is_empty())
            })
            .count();

        println!("humanmale: {}/{} bones have Stand keyframes", bones_with_stand_keyframes, bones.len());
        assert!(bones_with_stand_keyframes > 0, "At least some bones should have Stand animation data");
    }

    #[test]
    fn keyframe_binary_search() {
        // Empty
        assert!(find_keyframe_pair(&[], 100).is_none());

        // Single keyframe
        let (i, t) = find_keyframe_pair(&[0], 500).unwrap();
        assert_eq!(i, 0);
        assert_eq!(t, 0.0);

        // Two keyframes, before start
        let (i, t) = find_keyframe_pair(&[100, 200], 50).unwrap();
        assert_eq!(i, 0);
        assert_eq!(t, 0.0);

        // Two keyframes, midpoint
        let (i, t) = find_keyframe_pair(&[100, 200], 150).unwrap();
        assert_eq!(i, 0);
        assert!((t - 0.5).abs() < 0.01);

        // Two keyframes, at end
        let (i, t) = find_keyframe_pair(&[100, 200], 200).unwrap();
        assert_eq!(i, 1);
        assert_eq!(t, 0.0);

        // Multiple keyframes
        let (i, t) = find_keyframe_pair(&[0, 100, 200, 300], 250).unwrap();
        assert_eq!(i, 2);
        assert!((t - 0.5).abs() < 0.01);
    }

    #[test]
    fn vec3_lerp_basic() {
        let a = [0.0, 0.0, 0.0];
        let b = [10.0, 20.0, 30.0];
        let mid = lerp_vec3(&a, &b, 0.5);
        assert!((mid[0] - 5.0).abs() < 0.001);
        assert!((mid[1] - 10.0).abs() < 0.001);
        assert!((mid[2] - 15.0).abs() < 0.001);
    }

    #[test]
    fn rotation_unpack_identity() {
        // raw=-1 (negative path): (-1 + 32768) / 32767 = 32767/32767 = 1.0
        assert!((unpack_quat_component(-1) - 1.0).abs() < 0.001, "raw=-1 should give ~1.0");
        // raw=0 (non-negative path): (0 - 32767) / 32767 = -1.0
        assert!((unpack_quat_component(0) - (-1.0)).abs() < 0.001, "raw=0 should give ~-1.0");
        // raw=-32768 (negative path): (-32768 + 32768) / 32767 = 0.0
        assert!((unpack_quat_component(-32768) - 0.0).abs() < 0.001, "raw=-32768 should give ~0.0");
        // raw=32767 (non-negative path): (32767 - 32767) / 32767 = 0.0
        assert!((unpack_quat_component(32767) - 0.0).abs() < 0.001, "raw=32767 should give 0.0");
    }

    #[test]
    fn slerp_identity() {
        let a = [0.0, 0.0, 0.0, 1.0];
        let b = [0.0, 0.0, 0.0, 1.0];
        let result = slerp(&a, &b, 0.5);
        assert!((result[3] - 1.0).abs() < 0.001);
        assert!(result[0].abs() < 0.001);
    }
}
