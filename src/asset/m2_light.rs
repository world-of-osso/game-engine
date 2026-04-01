//! M2 model light parser (MD20 lights block at 0x108).

use super::m2::{read_f32, read_u16, read_u32};
use super::m2_anim::{AnimTrack, evaluate_f32_track, evaluate_u8_track, evaluate_vec3_track};

const MD20_LIGHTS_OFFSET: usize = 0x108;
const M2_LIGHT_ENTRY_SIZE_WOTLK_PLUS: usize = 0x9C;
const M2_LIGHT_ENTRY_SIZE_CATA_PLUS: usize = 0xA4;

/// Point light entry type in M2 light records.
pub const M2_LIGHT_TYPE_POINT: u16 = 1;

/// Parsed M2 model light entry.
#[derive(Clone)]
pub struct M2Light {
    pub light_type: u16,
    pub bone_index: i16,
    /// Position in WoW coordinates relative to the light parent (usually a bone).
    pub position: [f32; 3],
    pub ambient_color: AnimTrack<[f32; 3]>,
    pub ambient_intensity: AnimTrack<f32>,
    pub diffuse_color: AnimTrack<[f32; 3]>,
    pub diffuse_intensity: AnimTrack<f32>,
    pub attenuation_start: AnimTrack<f32>,
    pub attenuation_end: AnimTrack<f32>,
    pub visibility: AnimTrack<u8>,
}

#[derive(Debug, Clone, Copy)]
pub struct EvaluatedLight {
    pub visible: bool,
    pub color: [f32; 3],
    pub intensity: f32,
    pub attenuation_start: f32,
    pub attenuation_end: f32,
}

fn read_i16(data: &[u8], off: usize) -> Option<i16> {
    let bytes: [u8; 2] = data.get(off..off + 2)?.try_into().ok()?;
    Some(i16::from_le_bytes(bytes))
}

fn parse_vec3(data: &[u8], off: usize) -> Option<[f32; 3]> {
    Some([
        read_f32(data, off).ok()?,
        read_f32(data, off + 4).ok()?,
        read_f32(data, off + 8).ok()?,
    ])
}

fn parse_anim_track<T: Copy>(
    md20: &[u8],
    block_offset: usize,
    value_size: usize,
    parse_value: impl Fn(&[u8], usize) -> Option<T>,
) -> Option<AnimTrack<T>> {
    let interpolation_type = read_u16(md20, block_offset).ok()?;
    let global_sequence = read_i16(md20, block_offset + 2)?;
    let ts_outer_count = read_u32(md20, block_offset + 4).ok()? as usize;
    let ts_outer_offset = read_u32(md20, block_offset + 8).ok()? as usize;
    let keys_outer_count = read_u32(md20, block_offset + 12).ok()? as usize;
    let keys_outer_offset = read_u32(md20, block_offset + 16).ok()? as usize;
    let seq_count = ts_outer_count.min(keys_outer_count);
    let mut sequences = Vec::with_capacity(seq_count);
    for i in 0..seq_count {
        let ts_inner = ts_outer_offset + i * 8;
        let val_inner = keys_outer_offset + i * 8;
        let timestamps_count = read_u32(md20, ts_inner).ok()? as usize;
        let timestamps_offset = read_u32(md20, ts_inner + 4).ok()? as usize;
        let values_count = read_u32(md20, val_inner).ok()? as usize;
        let values_offset = read_u32(md20, val_inner + 4).ok()? as usize;

        let mut timestamps = Vec::with_capacity(timestamps_count);
        for j in 0..timestamps_count {
            timestamps.push(read_u32(md20, timestamps_offset + j * 4).ok()?);
        }
        let mut values = Vec::with_capacity(values_count);
        for j in 0..values_count {
            values.push(parse_value(md20, values_offset + j * value_size)?);
        }
        sequences.push((timestamps, values));
    }
    Some(AnimTrack {
        interpolation_type,
        global_sequence,
        sequences,
    })
}

fn parse_vec3_track(md20: &[u8], block_offset: usize) -> Option<AnimTrack<[f32; 3]>> {
    parse_anim_track(md20, block_offset, 12, parse_vec3)
}

fn parse_f32_track(md20: &[u8], block_offset: usize) -> Option<AnimTrack<f32>> {
    parse_anim_track(md20, block_offset, 4, |d, off| read_f32(d, off).ok())
}

fn parse_u8_track(md20: &[u8], block_offset: usize) -> Option<AnimTrack<u8>> {
    parse_anim_track(md20, block_offset, 1, |d, off| d.get(off).copied())
}

fn light_entry_stride(md20: &[u8], lights_offset: usize, count: usize) -> Option<usize> {
    let version = read_u32(md20, 4).unwrap_or(0);
    let preferred = if version >= 272 {
        [
            M2_LIGHT_ENTRY_SIZE_CATA_PLUS,
            M2_LIGHT_ENTRY_SIZE_WOTLK_PLUS,
        ]
    } else {
        [
            M2_LIGHT_ENTRY_SIZE_WOTLK_PLUS,
            M2_LIGHT_ENTRY_SIZE_CATA_PLUS,
        ]
    };
    preferred.into_iter().find(|stride| {
        lights_offset
            .checked_add(count.saturating_mul(*stride))
            .is_some_and(|size| size <= md20.len())
    })
}

fn parse_light_entry(
    md20: &[u8],
    base_offset: usize,
    index: usize,
    stride: usize,
) -> Option<M2Light> {
    let base = base_offset.checked_add(index.saturating_mul(stride))?;
    if base + M2_LIGHT_ENTRY_SIZE_WOTLK_PLUS > md20.len() {
        return None;
    }
    Some(M2Light {
        light_type: read_u16(md20, base).ok()?,
        bone_index: read_i16(md20, base + 2)?,
        position: parse_vec3(md20, base + 4)?,
        ambient_color: parse_vec3_track(md20, base + 0x10)?,
        ambient_intensity: parse_f32_track(md20, base + 0x24)?,
        diffuse_color: parse_vec3_track(md20, base + 0x38)?,
        diffuse_intensity: parse_f32_track(md20, base + 0x4C)?,
        attenuation_start: parse_f32_track(md20, base + 0x60)?,
        attenuation_end: parse_f32_track(md20, base + 0x74)?,
        visibility: parse_u8_track(md20, base + 0x88)?,
    })
}

/// Parse all model lights from an MD20 chunk.
pub fn parse_lights(md20: &[u8]) -> Vec<M2Light> {
    if md20.len() < MD20_LIGHTS_OFFSET + 8 {
        return Vec::new();
    }
    let count = read_u32(md20, MD20_LIGHTS_OFFSET).unwrap_or(0) as usize;
    let offset = read_u32(md20, MD20_LIGHTS_OFFSET + 4).unwrap_or(0) as usize;
    if count == 0 {
        return Vec::new();
    }
    let Some(stride) = light_entry_stride(md20, offset, count) else {
        return Vec::new();
    };
    let mut lights = Vec::with_capacity(count);
    for i in 0..count {
        let Some(light) = parse_light_entry(md20, offset, i, stride) else {
            break;
        };
        lights.push(light);
    }
    lights
}

pub fn evaluate_light(light: &M2Light, seq_idx: usize, time_ms: u32) -> EvaluatedLight {
    let ambient_color =
        evaluate_vec3_track(&light.ambient_color, seq_idx, time_ms).unwrap_or([0.0; 3]);
    let diffuse_color =
        evaluate_vec3_track(&light.diffuse_color, seq_idx, time_ms).unwrap_or([1.0; 3]);
    let ambient_intensity =
        evaluate_f32_track(&light.ambient_intensity, seq_idx, time_ms).unwrap_or(0.0);
    let diffuse_intensity =
        evaluate_f32_track(&light.diffuse_intensity, seq_idx, time_ms).unwrap_or(1.0);
    let attenuation_start = evaluate_f32_track(&light.attenuation_start, seq_idx, time_ms)
        .unwrap_or(1.0)
        .max(0.0);
    let attenuation_end = evaluate_f32_track(&light.attenuation_end, seq_idx, time_ms)
        .unwrap_or(attenuation_start + 1.0)
        .max(attenuation_start + 0.01);
    let visibility = evaluate_u8_track(&light.visibility, seq_idx, time_ms).unwrap_or(1);
    let color = [
        (ambient_color[0] * ambient_intensity + diffuse_color[0] * diffuse_intensity).max(0.0),
        (ambient_color[1] * ambient_intensity + diffuse_color[1] * diffuse_intensity).max(0.0),
        (ambient_color[2] * ambient_intensity + diffuse_color[2] * diffuse_intensity).max(0.0),
    ];
    let intensity = (color[0].max(color[1]).max(color[2]) * 2_000.0).max(0.1);
    EvaluatedLight {
        visible: visibility > 0,
        color,
        intensity,
        attenuation_start,
        attenuation_end,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_lights_reads_single_point_light() {
        let mut md20 = vec![0u8; 0x700];
        let light_base = 0x180usize;
        md20[MD20_LIGHTS_OFFSET..MD20_LIGHTS_OFFSET + 4].copy_from_slice(&(1u32).to_le_bytes());
        md20[MD20_LIGHTS_OFFSET + 4..MD20_LIGHTS_OFFSET + 8]
            .copy_from_slice(&(light_base as u32).to_le_bytes());
        md20[light_base..light_base + 2].copy_from_slice(&(M2_LIGHT_TYPE_POINT).to_le_bytes());
        md20[light_base + 2..light_base + 4].copy_from_slice(&(10i16).to_le_bytes());
        md20[light_base + 4..light_base + 8].copy_from_slice(&(1.25f32).to_le_bytes());
        md20[light_base + 8..light_base + 12].copy_from_slice(&(-2.0f32).to_le_bytes());
        md20[light_base + 12..light_base + 16].copy_from_slice(&(3.75f32).to_le_bytes());
        // ambient_color track with one key [0.2, 0.3, 0.4]
        md20[light_base + 0x10..light_base + 0x12].copy_from_slice(&0u16.to_le_bytes());
        md20[light_base + 0x12..light_base + 0x14].copy_from_slice(&(-1i16).to_le_bytes());
        md20[light_base + 0x14..light_base + 0x18].copy_from_slice(&(1u32).to_le_bytes());
        md20[light_base + 0x18..light_base + 0x1C].copy_from_slice(&(0x400u32).to_le_bytes());
        md20[light_base + 0x1C..light_base + 0x20].copy_from_slice(&(1u32).to_le_bytes());
        md20[light_base + 0x20..light_base + 0x24].copy_from_slice(&(0x420u32).to_le_bytes());
        md20[0x400..0x404].copy_from_slice(&(1u32).to_le_bytes());
        md20[0x404..0x408].copy_from_slice(&(0x500u32).to_le_bytes());
        md20[0x420..0x424].copy_from_slice(&(1u32).to_le_bytes());
        md20[0x424..0x428].copy_from_slice(&(0x520u32).to_le_bytes());
        md20[0x500..0x504].copy_from_slice(&0u32.to_le_bytes());
        md20[0x520..0x524].copy_from_slice(&(0.2f32).to_le_bytes());
        md20[0x524..0x528].copy_from_slice(&(0.3f32).to_le_bytes());
        md20[0x528..0x52C].copy_from_slice(&(0.4f32).to_le_bytes());
        // Fill remaining required tracks with static one-key defaults.
        for off in [0x24, 0x38, 0x4C, 0x60, 0x74] {
            md20[light_base + off..light_base + off + 2].copy_from_slice(&0u16.to_le_bytes());
            md20[light_base + off + 2..light_base + off + 4]
                .copy_from_slice(&(-1i16).to_le_bytes());
            md20[light_base + off + 4..light_base + off + 8].copy_from_slice(&(1u32).to_le_bytes());
            md20[light_base + off + 8..light_base + off + 12]
                .copy_from_slice(&(0x430u32).to_le_bytes());
            md20[light_base + off + 12..light_base + off + 16]
                .copy_from_slice(&(1u32).to_le_bytes());
            md20[light_base + off + 16..light_base + off + 20]
                .copy_from_slice(&(0x440u32).to_le_bytes());
        }
        md20[0x430..0x434].copy_from_slice(&(1u32).to_le_bytes());
        md20[0x434..0x438].copy_from_slice(&(0x510u32).to_le_bytes());
        md20[0x440..0x444].copy_from_slice(&(1u32).to_le_bytes());
        md20[0x444..0x448].copy_from_slice(&(0x530u32).to_le_bytes());
        md20[0x510..0x514].copy_from_slice(&0u32.to_le_bytes());
        md20[0x530..0x534].copy_from_slice(&(1.0f32).to_le_bytes());
        // visibility track
        let off = 0x88;
        md20[light_base + off..light_base + off + 2].copy_from_slice(&0u16.to_le_bytes());
        md20[light_base + off + 2..light_base + off + 4].copy_from_slice(&(-1i16).to_le_bytes());
        md20[light_base + off + 4..light_base + off + 8].copy_from_slice(&(1u32).to_le_bytes());
        md20[light_base + off + 8..light_base + off + 12]
            .copy_from_slice(&(0x450u32).to_le_bytes());
        md20[light_base + off + 12..light_base + off + 16].copy_from_slice(&(1u32).to_le_bytes());
        md20[light_base + off + 16..light_base + off + 20]
            .copy_from_slice(&(0x464u32).to_le_bytes());
        md20[0x450..0x454].copy_from_slice(&(1u32).to_le_bytes());
        md20[0x454..0x458].copy_from_slice(&(0x514u32).to_le_bytes());
        md20[0x464..0x468].copy_from_slice(&(1u32).to_le_bytes());
        md20[0x468..0x46C].copy_from_slice(&(0x534u32).to_le_bytes());
        md20[0x514..0x518].copy_from_slice(&0u32.to_le_bytes());
        md20[0x534] = 1u8;

        let lights = parse_lights(&md20);
        assert_eq!(lights.len(), 1);
        assert_eq!(lights[0].light_type, M2_LIGHT_TYPE_POINT);
        assert_eq!(lights[0].bone_index, 10);
        assert_eq!(lights[0].position, [1.25, -2.0, 3.75]);
        let evaluated = evaluate_light(&lights[0], 0, 0);
        assert!(evaluated.visible);
        assert!(evaluated.intensity > 1.0);
    }
}
