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

#[derive(Clone)]
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

/// Verify that all parent_bone_id values are -1 or a valid index.
#[allow(dead_code)]
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

#[allow(dead_code)]
#[derive(Clone)]
pub struct M2AnimSequence {
    pub id: u16,
    pub variation_id: u16,
    pub duration: u32, // milliseconds
    pub movespeed: f32,
    pub flags: u32,
    pub blend_time: u16,     // milliseconds, for transitions
    pub next_animation: i16, // -1 = none, else index into sequences
}

/// Parse `count` sequence entries starting at `offset` in `data`.
pub fn parse_sequences_at(
    data: &[u8],
    offset: usize,
    count: usize,
) -> Result<Vec<M2AnimSequence>, String> {
    const SEQ_SIZE: usize = 64;
    let mut sequences = Vec::with_capacity(count);
    for i in 0..count {
        let base = offset + i * SEQ_SIZE;
        if base + SEQ_SIZE > data.len() {
            return Err(format!("Sequence {i} out of bounds at offset {base:#x}"));
        }
        sequences.push(M2AnimSequence {
            id: read_u16(data, base)?,
            variation_id: read_u16(data, base + 0x02)?,
            duration: read_u32(data, base + 0x04)?,
            movespeed: read_f32(data, base + 0x08)?,
            flags: read_u32(data, base + 0x0C)?,
            blend_time: read_u16(data, base + 0x1C)?,
            next_animation: read_i16(data, base + 0x3C)?,
        });
    }
    Ok(sequences)
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
    parse_sequences_at(md20, offset, count)
}

/// Parse `count` global sequence durations starting at `offset` in `data`.
pub fn parse_global_sequences_at(
    data: &[u8],
    offset: usize,
    count: usize,
) -> Result<Vec<u32>, String> {
    let mut durations = Vec::with_capacity(count);
    for i in 0..count {
        durations.push(read_u32(data, offset + i * 4)?);
    }
    Ok(durations)
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
    parse_global_sequences_at(md20, offset, count)
}

/// A single animation track: keyframes for one transform component of one bone.
/// `sequences[i]` holds the keyframe data for animation sequence `i`.
#[allow(dead_code)]
#[derive(Clone)]
pub struct AnimTrack<T> {
    pub interpolation_type: u16,
    pub global_sequence: i16,
    /// Per-sequence keyframe data: (timestamps_ms, values).
    pub sequences: Vec<(Vec<u32>, Vec<T>)>,
}

/// All animation tracks for one bone.
#[derive(Clone)]
pub struct BoneAnimTracks {
    pub translation: AnimTrack<[f32; 3]>,
    pub rotation: AnimTrack<[i16; 4]>,
    pub scale: AnimTrack<[f32; 3]>,
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct TextureAnimTracks {
    pub translation: AnimTrack<[f32; 3]>,
    pub rotation: AnimTrack<[i16; 4]>,
    pub scale: AnimTrack<[f32; 3]>,
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct ColorAnimTracks {
    pub color: AnimTrack<[f32; 3]>,
    pub opacity: AnimTrack<i16>,
}

fn parse_vec3(md20: &[u8], off: usize) -> Result<[f32; 3], String> {
    Ok([
        read_f32(md20, off)?,
        read_f32(md20, off + 4)?,
        read_f32(md20, off + 8)?,
    ])
}

fn parse_quat_packed(md20: &[u8], off: usize) -> Result<[i16; 4], String> {
    Ok([
        read_i16(md20, off)?,
        read_i16(md20, off + 2)?,
        read_i16(md20, off + 4)?,
        read_i16(md20, off + 6)?,
    ])
}

fn parse_i16_value(md20: &[u8], off: usize) -> Result<i16, String> {
    read_i16(md20, off)
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
        let values =
            read_inner_value_array(md20, keys_outer_offset + i * 8, value_size, &parse_value)?;
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

/// Parse animation tracks for `count` bones starting at `offset` in `data`.
pub fn parse_bone_animations_at(
    data: &[u8],
    offset: usize,
    count: usize,
) -> Result<Vec<BoneAnimTracks>, String> {
    const BONE_SIZE: usize = 88;
    let mut tracks = Vec::with_capacity(count);
    for i in 0..count {
        let base = offset + i * BONE_SIZE;
        if base + BONE_SIZE > data.len() {
            return Err(format!("Bone {i} out of bounds at offset {base:#x}"));
        }
        let translation = parse_anim_track(data, base + 0x10, 12, parse_vec3)?;
        let rotation = parse_anim_track(data, base + 0x24, 8, parse_quat_packed)?;
        let scale = parse_anim_track(data, base + 0x38, 12, parse_vec3)?;
        tracks.push(BoneAnimTracks {
            translation,
            rotation,
            scale,
        });
    }
    Ok(tracks)
}

/// Parse animation tracks for all bones from the MD20 blob.
/// Returns one BoneAnimTracks per bone, in the same order as parse_bones.
pub fn parse_bone_animations(md20: &[u8]) -> Result<Vec<BoneAnimTracks>, String> {
    if md20.len() < 0x34 {
        return Ok(Vec::new());
    }
    let count = read_u32(md20, 0x2C)? as usize;
    let offset = read_u32(md20, 0x30)? as usize;
    parse_bone_animations_at(md20, offset, count)
}

pub fn parse_transparency_tracks(md20: &[u8]) -> Result<Vec<AnimTrack<i16>>, String> {
    if md20.len() < 0x60 {
        return Ok(Vec::new());
    }
    let count = read_u32(md20, 0x58)? as usize;
    let offset = read_u32(md20, 0x5C)? as usize;
    let mut tracks = Vec::with_capacity(count);
    for i in 0..count {
        let base = offset + i * 20;
        if base + 20 > md20.len() {
            return Err(format!(
                "Transparency track {i} out of bounds at offset {base:#x}"
            ));
        }
        tracks.push(parse_anim_track(md20, base, 2, parse_i16_value)?);
    }
    Ok(tracks)
}

pub fn parse_color_tracks(md20: &[u8]) -> Result<Vec<ColorAnimTracks>, String> {
    if md20.len() < 0x50 {
        return Ok(Vec::new());
    }
    let count = read_u32(md20, 0x48)? as usize;
    let offset = read_u32(md20, 0x4C)? as usize;
    let mut tracks = Vec::with_capacity(count);
    for i in 0..count {
        let base = offset + i * 40;
        if base + 40 > md20.len() {
            return Err(format!("Color track {i} out of bounds at offset {base:#x}"));
        }
        tracks.push(ColorAnimTracks {
            color: parse_anim_track(md20, base, 12, parse_vec3)?,
            opacity: parse_anim_track(md20, base + 20, 2, parse_i16_value)?,
        });
    }
    Ok(tracks)
}

pub fn parse_texture_animations(md20: &[u8]) -> Result<Vec<TextureAnimTracks>, String> {
    if md20.len() < 0x68 {
        return Ok(Vec::new());
    }
    let count = read_u32(md20, 0x60)? as usize;
    let offset = read_u32(md20, 0x64)? as usize;
    let mut tracks = Vec::with_capacity(count);
    for i in 0..count {
        let base = offset + i * 60;
        if base + 60 > md20.len() {
            return Err(format!(
                "Texture animation track {i} out of bounds at offset {base:#x}"
            ));
        }
        tracks.push(TextureAnimTracks {
            translation: parse_anim_track(md20, base, 12, parse_vec3)?,
            rotation: parse_anim_track(md20, base + 20, 8, parse_quat_packed)?,
            scale: parse_anim_track(md20, base + 40, 12, parse_vec3)?,
        });
    }
    Ok(tracks)
}

/// Unpack a WoW packed i16 quaternion to a Bevy Quat with coordinate conversion.
/// WoW packs rotation as [i16; 4] (x, y, z, w).
/// Unpack: (raw < 0 ? raw + 32768 : raw - 32767) / 32767.0
/// WoW→Bevy axis permutation: same as positions (x, y, z) → (x, z, -y),
/// applied to quaternion imaginary part: (qx, qy, qz, qw) → (qx, qz, -qy, qw).
pub fn unpack_rotation(packed: &[i16; 4]) -> [f32; 4] {
    let x = unpack_quat_component(packed[0]);
    let y = unpack_quat_component(packed[1]);
    let z = unpack_quat_component(packed[2]);
    let w = unpack_quat_component(packed[3]);
    [x, z, -y, w]
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

pub fn evaluate_i16_track(track: &AnimTrack<i16>, seq_idx: usize, time_ms: u32) -> Option<i16> {
    let (timestamps, values) = track.sequences.get(seq_idx)?;
    if timestamps.is_empty() || values.is_empty() {
        return None;
    }
    let (i, _) = find_keyframe_pair(timestamps, time_ms)?;
    values.get(i).copied()
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

fn normalize_quat(q: [f32; 4]) -> [f32; 4] {
    let len = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    [q[0] / len, q[1] / len, q[2] / len, q[3] / len]
}

fn slerp_linear(a: &[f32; 4], b: &[f32; 4], t: f32) -> [f32; 4] {
    normalize_quat([
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ])
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
        return slerp_linear(a, &b, t);
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

pub fn evaluate_f32_track(track: &AnimTrack<f32>, seq_idx: usize, time_ms: u32) -> Option<f32> {
    let (timestamps, values) = track.sequences.get(seq_idx)?;
    if timestamps.is_empty() || values.is_empty() {
        return None;
    }
    let (i, t) = find_keyframe_pair(timestamps, time_ms)?;
    if t == 0.0 || i + 1 >= values.len() {
        return Some(values[i]);
    }
    Some(values[i] + (values[i + 1] - values[i]) * t)
}

pub fn evaluate_u8_track(track: &AnimTrack<u8>, seq_idx: usize, time_ms: u32) -> Option<u8> {
    let (timestamps, values) = track.sequences.get(seq_idx)?;
    if timestamps.is_empty() || values.is_empty() {
        return None;
    }
    let (i, _) = find_keyframe_pair(timestamps, time_ms)?;
    values.get(i).copied()
}

#[cfg(test)]
#[path = "../../tests/unit/asset/m2_anim_tests.rs"]
mod tests;
