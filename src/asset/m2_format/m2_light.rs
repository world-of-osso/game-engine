//! M2 model light parser (MD20 lights block at 0x108).

use super::m2_anim::{AnimTrack, evaluate_f32_track, evaluate_u8_track, evaluate_vec3_track};
use super::{read_f32, read_i16, read_u16, read_u32, read_vec3};
use crate::asset::read_bytes::read_m2_array_header;

const MD20_LIGHTS_OFFSET: usize = 0x108;
const M2_LIGHT_ENTRY_SIZE_WOTLK_PLUS: usize = 0x9C;
const M2_LIGHT_ENTRY_SIZE_CATA_PLUS: usize = 0xA4;
const LIGHT_ENTRY_TYPE_OFFSET: usize = 0x00;
const LIGHT_ENTRY_BONE_INDEX_OFFSET: usize = 0x02;
const LIGHT_ENTRY_POSITION_OFFSET: usize = 0x04;
const LIGHT_ENTRY_AMBIENT_COLOR_OFFSET: usize = 0x10;
const LIGHT_ENTRY_AMBIENT_INTENSITY_OFFSET: usize = 0x24;
const LIGHT_ENTRY_DIFFUSE_COLOR_OFFSET: usize = 0x38;
const LIGHT_ENTRY_DIFFUSE_INTENSITY_OFFSET: usize = 0x4C;
const LIGHT_ENTRY_ATTENUATION_START_OFFSET: usize = 0x60;
const LIGHT_ENTRY_ATTENUATION_END_OFFSET: usize = 0x74;
const LIGHT_ENTRY_VISIBILITY_OFFSET: usize = 0x88;
const ANIM_BLOCK_GLOBAL_SEQUENCE_OFFSET: usize = 2;
const ANIM_BLOCK_TIMESTAMPS_COUNT_OFFSET: usize = 4;
const ANIM_BLOCK_TIMESTAMPS_OFFSET: usize = 8;
const ANIM_BLOCK_VALUES_COUNT_OFFSET: usize = 12;
const ANIM_BLOCK_VALUES_OFFSET: usize = 16;
const M2_ARRAY_ENTRY_SIZE: usize = 8;
const TIMESTAMP_ENTRY_SIZE: usize = 4;

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

fn parse_vec3(data: &[u8], off: usize) -> Option<[f32; 3]> {
    read_vec3(data, off).ok()
}

fn parse_anim_track<T: Copy>(
    md20: &[u8],
    block_offset: usize,
    value_size: usize,
    parse_value: impl Fn(&[u8], usize) -> Option<T>,
) -> Option<AnimTrack<T>> {
    let interpolation_type = read_u16(md20, block_offset).ok()?;
    let global_sequence = read_i16(md20, block_offset + ANIM_BLOCK_GLOBAL_SEQUENCE_OFFSET).ok()?;
    let ts_outer_count =
        read_u32(md20, block_offset + ANIM_BLOCK_TIMESTAMPS_COUNT_OFFSET).ok()? as usize;
    let ts_outer_offset =
        read_u32(md20, block_offset + ANIM_BLOCK_TIMESTAMPS_OFFSET).ok()? as usize;
    let keys_outer_count =
        read_u32(md20, block_offset + ANIM_BLOCK_VALUES_COUNT_OFFSET).ok()? as usize;
    let keys_outer_offset = read_u32(md20, block_offset + ANIM_BLOCK_VALUES_OFFSET).ok()? as usize;
    let seq_count = ts_outer_count.min(keys_outer_count);
    let mut sequences = Vec::with_capacity(seq_count);
    for i in 0..seq_count {
        let ts_inner = ts_outer_offset + i * M2_ARRAY_ENTRY_SIZE;
        let val_inner = keys_outer_offset + i * M2_ARRAY_ENTRY_SIZE;
        let timestamps_count = read_u32(md20, ts_inner).ok()? as usize;
        let timestamps_offset = read_u32(md20, ts_inner + TIMESTAMP_ENTRY_SIZE).ok()? as usize;
        let values_count = read_u32(md20, val_inner).ok()? as usize;
        let values_offset = read_u32(md20, val_inner + TIMESTAMP_ENTRY_SIZE).ok()? as usize;

        let mut timestamps = Vec::with_capacity(timestamps_count);
        for j in 0..timestamps_count {
            timestamps.push(read_u32(md20, timestamps_offset + j * TIMESTAMP_ENTRY_SIZE).ok()?);
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
    let version = read_u32(md20, super::MD20_VERSION_OFFSET).unwrap_or(0);
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
        light_type: read_u16(md20, base + LIGHT_ENTRY_TYPE_OFFSET).ok()?,
        bone_index: read_i16(md20, base + LIGHT_ENTRY_BONE_INDEX_OFFSET).ok()?,
        position: parse_vec3(md20, base + LIGHT_ENTRY_POSITION_OFFSET)?,
        ambient_color: parse_vec3_track(md20, base + LIGHT_ENTRY_AMBIENT_COLOR_OFFSET)?,
        ambient_intensity: parse_f32_track(md20, base + LIGHT_ENTRY_AMBIENT_INTENSITY_OFFSET)?,
        diffuse_color: parse_vec3_track(md20, base + LIGHT_ENTRY_DIFFUSE_COLOR_OFFSET)?,
        diffuse_intensity: parse_f32_track(md20, base + LIGHT_ENTRY_DIFFUSE_INTENSITY_OFFSET)?,
        attenuation_start: parse_f32_track(md20, base + LIGHT_ENTRY_ATTENUATION_START_OFFSET)?,
        attenuation_end: parse_f32_track(md20, base + LIGHT_ENTRY_ATTENUATION_END_OFFSET)?,
        visibility: parse_u8_track(md20, base + LIGHT_ENTRY_VISIBILITY_OFFSET)?,
    })
}

/// Parse all model lights from an MD20 chunk.
pub fn parse_lights(md20: &[u8]) -> Vec<M2Light> {
    let (count, offset) = read_m2_array_header(md20, MD20_LIGHTS_OFFSET).unwrap_or((0, 0));
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
#[path = "../../../tests/unit/asset/m2_light_tests.rs"]
mod tests;
