//! M2 particle emitter parser.
//!
//! Parses particle emitter blocks from the MD20 header at offset 0x128.
//! Each emitter has static properties + AnimBlock tracks for dynamic values.
//!
//! The parsed fields live in the first `0x178` bytes of the 272-era particle
//! emitter struct, but local 272/274 assets use the larger `0x1EC` stride.

#[path = "m2_particle_defaults.rs"]
mod defaults;

/// Parsed M2 particle emitter.
#[derive(Debug, Clone)]
pub struct M2ParticleEmitter {
    pub flags: u32,
    /// Position in WoW coordinates (relative to bone).
    pub position: [f32; 3],
    pub bone_index: u16,
    pub texture_index: u16,
    /// Resolved texture FileDataID (from TXID chunk).
    pub texture_fdid: Option<u32>,
    pub blend_type: u8,
    /// 0 = plane, 1 = sphere, 2 = spline.
    pub emitter_type: u8,
    /// 0 = normal billboard, 1 = origin->position trail quad, others rare/legacy.
    pub particle_type: u8,
    /// Distinguishes head vs tail rendering for some legacy emitters.
    pub head_or_tail: u8,
    pub tile_rows: u16,
    pub tile_cols: u16,
    pub emission_speed: f32,
    pub speed_variation: f32,
    pub vertical_range: f32,
    pub horizontal_range: f32,
    pub gravity: f32,
    pub lifespan: f32,
    /// Symmetric +/- lifetime variation around `lifespan`.
    pub lifespan_variation: f32,
    pub emission_rate: f32,
    pub area_length: f32,
    pub area_width: f32,
    pub drag: f32,
    /// Base in-plane rotation in radians.
    pub base_spin: f32,
    /// Random variation applied to base in-plane rotation.
    pub base_spin_variation: f32,
    /// Angular velocity in radians/second.
    pub spin: f32,
    /// Random variation applied to angular velocity.
    pub spin_variation: f32,
    /// Additional wind acceleration vector in WoW coordinates.
    pub wind_vector: [f32; 3],
    /// Duration in seconds for which wind affects newly spawned particles.
    pub wind_time: f32,
    /// Color over lifetime: start, mid, end (RGB 0–255).
    pub colors: [[f32; 3]; 3],
    /// Full FakeAnimBlock color keys as (normalized time, RGB 0–255).
    pub color_keys: Vec<(f32, [f32; 3])>,
    /// Opacity over lifetime: start, mid, end (0–1).
    pub opacity: [f32; 3],
    /// Full FakeAnimBlock opacity keys as (normalized time, opacity 0–1).
    pub opacity_keys: Vec<(f32, f32)>,
    /// Scale over lifetime: start, mid, end (x,y pairs).
    pub scales: [[f32; 2]; 3],
    /// Full FakeAnimBlock scale keys as (normalized time, [x, y]).
    pub scale_keys: Vec<(f32, [f32; 2])>,
    /// Twinkle pulse frequency in cycles per second.
    pub twinkle_speed: f32,
    /// Chance for a particle to participate in the twinkle pulse.
    pub twinkle_percent: f32,
    /// Minimum twinkle scale multiplier.
    pub twinkle_scale_min: f32,
    /// Maximum twinkle scale multiplier.
    pub twinkle_scale_max: f32,
    /// Simple flipbook cell track used by some emitters for head particles.
    pub head_cell_track: [u16; 3],
    /// Simple flipbook cell track used by some emitters for tail particles.
    pub tail_cell_track: [u16; 3],
    /// Additional size multiplier baked into the emitter definition.
    pub burst_multiplier: f32,
    /// Midpoint (0–1) between start→mid vs mid→end interpolation.
    pub mid_point: f32,
}

use super::m2_format::{read_f32, read_u16, read_u32};

const EMITTER_VISUAL_COLOR_OFFSET: usize = 0x104;
const EMITTER_VISUAL_OPACITY_OFFSET: usize = 0x114;
const EMITTER_VISUAL_SCALE_OFFSET: usize = 0x124;
const EMITTER_LIFESPAN_VARIATION_OFFSET: usize = 0xAC;
const EMITTER_TWINKLE_SPEED_OFFSET: usize = 0x164;
const EMITTER_TWINKLE_PERCENT_OFFSET: usize = 0x168;
const EMITTER_TWINKLE_SCALE_MIN_OFFSET: usize = 0x16C;
const EMITTER_TWINKLE_SCALE_MAX_OFFSET: usize = 0x170;
const EMITTER_HEAD_CELL_TRACK_OFFSET: usize = 0x13C;
const EMITTER_TAIL_CELL_TRACK_OFFSET: usize = 0x14C;
const EMITTER_BURST_MULTIPLIER_OFFSET: usize = 0x174;
const EMITTER_BASE_SPIN_OFFSET: usize = 0x178;
const EMITTER_BASE_SPIN_VARIATION_OFFSET: usize = 0x17C;
const EMITTER_SPIN_OFFSET: usize = 0x180;
const EMITTER_SPIN_VARIATION_OFFSET: usize = 0x184;
const EMITTER_WIND_VECTOR_OFFSET: usize = 0x1A0;
const EMITTER_WIND_TIME_OFFSET: usize = 0x1AC;
const EMITTER_PARSED_PREFIX_SIZE: usize = 0x178;
const EMITTER_272_STRIDE: usize = 0x1EC;

struct EmitterHeaderCore {
    flags: u32,
    position: [f32; 3],
    bone_index: u16,
    texture_index: u16,
    blend_type: u8,
    emitter_type: u8,
    particle_type: u8,
    head_or_tail: u8,
    tile_rows: u16,
    tile_cols: u16,
}

impl Default for EmitterHeaderCore {
    fn default() -> Self {
        Self {
            flags: 0,
            position: [0.0; 3],
            bone_index: 0,
            texture_index: 0,
            blend_type: 0,
            emitter_type: 0,
            particle_type: 0,
            head_or_tail: 0,
            tile_rows: 0,
            tile_cols: 0,
        }
    }
}

fn read_i16(data: &[u8], off: usize) -> Result<i16, String> {
    let bytes: [u8; 2] = data
        .get(off..off + 2)
        .ok_or_else(|| format!("read_i16 out of bounds at offset {off:#x}"))?
        .try_into()
        .unwrap();
    Ok(i16::from_le_bytes(bytes))
}

fn read_u16_values(md20: &[u8], emitter: &[u8], off: usize) -> [u16; 3] {
    let mut values = [0u16; 3];
    let count = read_u32(emitter, off + 8).unwrap_or(0) as usize;
    let base = read_u32(emitter, off + 12).unwrap_or(0) as usize;
    for (i, value) in values.iter_mut().enumerate().take(count.min(3)) {
        *value = read_u16(md20, base + i * 2).unwrap_or(0);
    }
    values
}

fn read_normalized_timestamps(md20: &[u8], emitter: &[u8], off: usize) -> Vec<f32> {
    let count = read_u32(emitter, off).unwrap_or(0) as usize;
    let base = read_u32(emitter, off + 4).unwrap_or(0) as usize;
    let mut timestamps = Vec::with_capacity(count);
    for i in 0..count {
        timestamps.push(
            read_u16(md20, base + i * 2)
                .map(|v| (v as f32 / 32767.0).clamp(0.0, 1.0))
                .unwrap_or(0.0),
        );
    }
    timestamps
}

/// Read the first float from a static M2Track (Cata+ indirect M2Arrays).
/// The outer M2Array points to inner M2Arrays (one per anim sequence).
/// Each inner M2Array's first 4 bytes hold the float value directly.
fn read_track_static_f32(md20: &[u8], emitter: &[u8], track_offset: usize) -> f32 {
    let outer_count = read_u32(emitter, track_offset + 12).unwrap_or(0);
    let outer_offset = read_u32(emitter, track_offset + 16).unwrap_or(0) as usize;
    if outer_count == 0 || outer_offset + 8 > md20.len() {
        return 0.0;
    }
    // Inner M2Array: {count, offset} — dereference to get actual float data.
    let inner_offset = read_u32(md20, outer_offset + 4).unwrap_or(0) as usize;
    if inner_offset == 0 {
        // Static track: float stored directly at outer_offset.
        return read_f32(md20, outer_offset).unwrap_or(0.0);
    }
    read_f32(md20, inner_offset).unwrap_or(0.0)
}

/// Read FakeAnimBlock color values (3 × RGB as C3Vector floats 0–255).
fn read_color_values(md20: &[u8], emitter: &[u8], off: usize) -> [[f32; 3]; 3] {
    let mut colors = [[0.0f32; 3]; 3];
    let count = read_u32(emitter, off + 8).unwrap_or(0) as usize;
    let base = read_u32(emitter, off + 12).unwrap_or(0) as usize;
    for (i, color) in colors.iter_mut().enumerate().take(count.min(3)) {
        let o = base + i * 12;
        *color = [
            read_f32(md20, o).unwrap_or(0.0),
            read_f32(md20, o + 4).unwrap_or(0.0),
            read_f32(md20, o + 8).unwrap_or(0.0),
        ];
    }
    colors
}

fn read_color_keys(md20: &[u8], emitter: &[u8], off: usize) -> Vec<(f32, [f32; 3])> {
    read_fake_animblock_keys(md20, emitter, off, 12, |md20, value_offset| {
        [
            read_f32(md20, value_offset).unwrap_or(0.0),
            read_f32(md20, value_offset + 4).unwrap_or(0.0),
            read_f32(md20, value_offset + 8).unwrap_or(0.0),
        ]
    })
}

/// Read FakeAnimBlock opacity values (3 × signed Fixed16, clamped to 0–1).
fn read_opacity_values(md20: &[u8], emitter: &[u8], off: usize) -> [f32; 3] {
    let mut opacities = [1.0f32; 3];
    let count = read_u32(emitter, off + 8).unwrap_or(0) as usize;
    let base = read_u32(emitter, off + 12).unwrap_or(0) as usize;
    for (i, opacity) in opacities.iter_mut().enumerate().take(count.min(3)) {
        *opacity = read_i16(md20, base + i * 2)
            .map(|v| (v as f32 / 32767.0).clamp(0.0, 1.0))
            .unwrap_or(1.0);
    }
    opacities
}

fn read_opacity_keys(md20: &[u8], emitter: &[u8], off: usize) -> Vec<(f32, f32)> {
    read_fake_animblock_keys(md20, emitter, off, 2, |md20, value_offset| {
        read_i16(md20, value_offset)
            .map(|v| (v as f32 / 32767.0).clamp(0.0, 1.0))
            .unwrap_or(1.0)
    })
}

/// Read FakeAnimBlock scale values (3 × [f32; 2]).
fn read_scale_values(md20: &[u8], emitter: &[u8], off: usize) -> [[f32; 2]; 3] {
    let mut scales = [[1.0f32; 2]; 3];
    let count = read_u32(emitter, off + 8).unwrap_or(0) as usize;
    let base = read_u32(emitter, off + 12).unwrap_or(0) as usize;
    for (i, scale) in scales.iter_mut().enumerate().take(count.min(3)) {
        let o = base + i * 8;
        *scale = [
            read_f32(md20, o).unwrap_or(1.0),
            read_f32(md20, o + 4).unwrap_or(1.0),
        ];
    }
    scales
}

fn read_scale_keys(md20: &[u8], emitter: &[u8], off: usize) -> Vec<(f32, [f32; 2])> {
    read_fake_animblock_keys(md20, emitter, off, 8, |md20, value_offset| {
        [
            read_f32(md20, value_offset).unwrap_or(1.0),
            read_f32(md20, value_offset + 4).unwrap_or(1.0),
        ]
    })
}

fn read_fake_animblock_keys<T>(
    md20: &[u8],
    emitter: &[u8],
    off: usize,
    value_stride: usize,
    read_value: impl Fn(&[u8], usize) -> T,
) -> Vec<(f32, T)> {
    let timestamps = read_normalized_timestamps(md20, emitter, off);
    let count = read_u32(emitter, off + 8).unwrap_or(0) as usize;
    let base = read_u32(emitter, off + 12).unwrap_or(0) as usize;
    let key_count = count.min(timestamps.len());
    let mut keys = Vec::with_capacity(key_count);
    for (i, time) in timestamps.into_iter().enumerate().take(key_count) {
        let value_offset = base + i * value_stride;
        keys.push((time, read_value(md20, value_offset)));
    }
    keys
}

/// Read midpoint from color FakeAnimBlock timestamps (normalized 0–32767 → 0–1).
fn read_midpoint(md20: &[u8], emitter: &[u8], off: usize) -> f32 {
    let count = read_u32(emitter, off).unwrap_or(0);
    let offset = read_u32(emitter, off + 4).unwrap_or(0) as usize;
    if count < 2 {
        return 0.5;
    }
    read_u16(md20, offset + 2)
        .map(|v| v as f32 / 32767.0)
        .unwrap_or(0.5)
}

/// Parse static emitter fields (id through tile dimensions) at offset within MD20.
fn parse_emitter_header(em: &[u8]) -> Result<M2ParticleEmitter, String> {
    parse_emitter_header_core(em).map(defaults::build_emitter_header)
}

fn parse_emitter_header_core(em: &[u8]) -> Result<EmitterHeaderCore, String> {
    Ok(EmitterHeaderCore {
        flags: read_u32(em, 0x04)?,
        position: [
            read_f32(em, 0x08)?,
            read_f32(em, 0x0C)?,
            read_f32(em, 0x10)?,
        ],
        bone_index: read_u16(em, 0x14)?,
        texture_index: read_u16(em, 0x16)? & 0x1F,
        blend_type: em[0x28],
        emitter_type: em[0x29],
        particle_type: em[0x2A],
        head_or_tail: em[0x2B],
        tile_rows: read_u16(em, 0x30)?,
        tile_cols: read_u16(em, 0x32)?,
    })
}

impl Default for M2ParticleEmitter {
    fn default() -> Self {
        defaults::default_emitter_state()
    }
}

/// Fill M2Track-based dynamic values on an emitter.
fn fill_track_values(em: &mut M2ParticleEmitter, md20: &[u8], data: &[u8]) {
    em.emission_speed = read_track_static_f32(md20, data, 0x34);
    em.speed_variation = read_track_static_f32(md20, data, 0x48);
    em.vertical_range = read_track_static_f32(md20, data, 0x5C);
    em.horizontal_range = read_track_static_f32(md20, data, 0x70);
    em.gravity = read_track_static_f32(md20, data, 0x84);
    em.lifespan = read_track_static_f32(md20, data, 0x98);
    em.lifespan_variation = read_f32(data, EMITTER_LIFESPAN_VARIATION_OFFSET).unwrap_or(0.0);
    em.emission_rate = read_track_static_f32(md20, data, 0xB0);
    em.area_length = read_track_static_f32(md20, data, 0xC8);
    em.area_width = read_track_static_f32(md20, data, 0xDC);
    em.drag = read_track_static_f32(md20, data, 0xF0);
    em.base_spin = read_f32(data, EMITTER_BASE_SPIN_OFFSET).unwrap_or(0.0);
    em.base_spin_variation = read_f32(data, EMITTER_BASE_SPIN_VARIATION_OFFSET).unwrap_or(0.0);
    em.spin = read_f32(data, EMITTER_SPIN_OFFSET).unwrap_or(0.0);
    em.spin_variation = read_f32(data, EMITTER_SPIN_VARIATION_OFFSET).unwrap_or(0.0);
    em.wind_vector = [
        read_f32(data, EMITTER_WIND_VECTOR_OFFSET).unwrap_or(0.0),
        read_f32(data, EMITTER_WIND_VECTOR_OFFSET + 4).unwrap_or(0.0),
        read_f32(data, EMITTER_WIND_VECTOR_OFFSET + 8).unwrap_or(0.0),
    ];
    em.wind_time = read_f32(data, EMITTER_WIND_TIME_OFFSET).unwrap_or(0.0);
}

/// Fill FakeAnimBlock visual values (color, opacity, scale).
fn fill_visual_values(em: &mut M2ParticleEmitter, md20: &[u8], data: &[u8]) {
    em.mid_point = read_midpoint(md20, data, EMITTER_VISUAL_COLOR_OFFSET);
    em.colors = read_color_values(md20, data, EMITTER_VISUAL_COLOR_OFFSET);
    em.color_keys = read_color_keys(md20, data, EMITTER_VISUAL_COLOR_OFFSET);
    em.opacity = read_opacity_values(md20, data, EMITTER_VISUAL_OPACITY_OFFSET);
    em.opacity_keys = read_opacity_keys(md20, data, EMITTER_VISUAL_OPACITY_OFFSET);
    em.scales = read_scale_values(md20, data, EMITTER_VISUAL_SCALE_OFFSET);
    em.scale_keys = read_scale_keys(md20, data, EMITTER_VISUAL_SCALE_OFFSET);
    em.twinkle_speed = read_f32(data, EMITTER_TWINKLE_SPEED_OFFSET).unwrap_or(0.0);
    em.twinkle_percent = read_f32(data, EMITTER_TWINKLE_PERCENT_OFFSET).unwrap_or(0.0);
    em.twinkle_scale_min = read_f32(data, EMITTER_TWINKLE_SCALE_MIN_OFFSET).unwrap_or(1.0);
    em.twinkle_scale_max = read_f32(data, EMITTER_TWINKLE_SCALE_MAX_OFFSET).unwrap_or(1.0);
    em.head_cell_track = read_u16_values(md20, data, EMITTER_HEAD_CELL_TRACK_OFFSET);
    em.tail_cell_track = read_u16_values(md20, data, EMITTER_TAIL_CELL_TRACK_OFFSET);
    em.burst_multiplier = match read_f32(data, EMITTER_BURST_MULTIPLIER_OFFSET).unwrap_or(0.0) {
        value if value > 0.0 => value,
        _ => 1.0,
    };
}

/// Parse a single particle emitter from the MD20 blob.
fn parse_emitter(md20: &[u8], offset: usize) -> Result<M2ParticleEmitter, String> {
    let data = md20
        .get(offset..)
        .ok_or_else(|| format!("Emitter out of bounds at {offset:#x}"))?;
    if data.len() < EMITTER_PARSED_PREFIX_SIZE {
        return Err("Emitter data too short".into());
    }
    let mut em = parse_emitter_header(data)?;
    fill_track_values(&mut em, md20, data);
    fill_visual_values(&mut em, md20, data);
    Ok(em)
}

/// Resolve texture FDIDs on parsed emitters from the TXID array.
pub fn resolve_texture_fdids(emitters: &mut [M2ParticleEmitter], txid: &[u32]) {
    for em in emitters {
        let idx = em.texture_index as usize;
        em.texture_fdid = txid.get(idx).copied().filter(|&f| f != 0);
    }
}

/// Parse all particle emitters from the MD20 header (M2Array at offset 0x128).
pub fn parse_particle_emitters(md20: &[u8]) -> Vec<M2ParticleEmitter> {
    if md20.len() < 0x130 {
        return Vec::new();
    }
    let version = read_u32(md20, 0x4).unwrap_or(0);
    let count = read_u32(md20, 0x128).unwrap_or(0) as usize;
    let offset = read_u32(md20, 0x12C).unwrap_or(0) as usize;
    if count == 0 {
        return Vec::new();
    }
    let stride = emitter_stride(version);
    let mut emitters = Vec::with_capacity(count);
    for i in 0..count {
        match parse_emitter(md20, offset + i * stride) {
            Ok(em) => emitters.push(em),
            Err(e) => eprintln!("Failed to parse particle emitter {i}: {e}"),
        }
    }
    emitters
}

fn emitter_stride(version: u32) -> usize {
    if version >= 272 {
        EMITTER_272_STRIDE
    } else {
        EMITTER_PARSED_PREFIX_SIZE
    }
}

#[cfg(test)]
#[path = "../../tests/unit/asset/m2_particle_tests.rs"]
mod tests;
