//! M2 particle emitter parser.
//!
//! Parses particle emitter blocks from the MD20 header at offset 0x128.
//! Each emitter has static properties + AnimBlock tracks for dynamic values.
//!
//! Cata+ (version >= 272) layout per emitter — see wowdev.wiki/M2#Particle_emitters.

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
    pub tile_rows: u16,
    pub tile_cols: u16,
    pub emission_speed: f32,
    pub speed_variation: f32,
    pub vertical_range: f32,
    pub horizontal_range: f32,
    pub gravity: f32,
    pub lifespan: f32,
    pub emission_rate: f32,
    pub area_length: f32,
    pub area_width: f32,
    /// Color over lifetime: start, mid, end (RGB 0–255).
    pub colors: [[f32; 3]; 3],
    /// Opacity over lifetime: start, mid, end (0–1).
    pub opacity: [f32; 3],
    /// Scale over lifetime: start, mid, end (x,y pairs).
    pub scales: [[f32; 2]; 3],
    /// Midpoint (0–1) between start→mid vs mid→end interpolation.
    pub mid_point: f32,
}

use super::m2::{read_f32, read_u16, read_u32};

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
    for i in 0..count.min(3) {
        let o = base + i * 12;
        colors[i] = [
            read_f32(md20, o).unwrap_or(0.0),
            read_f32(md20, o + 4).unwrap_or(0.0),
            read_f32(md20, o + 8).unwrap_or(0.0),
        ];
    }
    colors
}

/// Read FakeAnimBlock opacity values (3 × u16, mapped to 0–1 range).
fn read_opacity_values(md20: &[u8], emitter: &[u8], off: usize) -> [f32; 3] {
    let mut opacities = [1.0f32; 3];
    let count = read_u32(emitter, off + 8).unwrap_or(0) as usize;
    let base = read_u32(emitter, off + 12).unwrap_or(0) as usize;
    for i in 0..count.min(3) {
        opacities[i] = read_u16(md20, base + i * 2)
            .map(|v| v as f32 / 32767.0)
            .unwrap_or(1.0);
    }
    opacities
}

/// Read FakeAnimBlock scale values (3 × [f32; 2]).
fn read_scale_values(md20: &[u8], emitter: &[u8], off: usize) -> [[f32; 2]; 3] {
    let mut scales = [[1.0f32; 2]; 3];
    let count = read_u32(emitter, off + 8).unwrap_or(0) as usize;
    let base = read_u32(emitter, off + 12).unwrap_or(0) as usize;
    for i in 0..count.min(3) {
        let o = base + i * 8;
        scales[i] = [
            read_f32(md20, o).unwrap_or(1.0),
            read_f32(md20, o + 4).unwrap_or(1.0),
        ];
    }
    scales
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
    Ok(M2ParticleEmitter {
        flags: read_u32(em, 0x04)?,
        position: [
            read_f32(em, 0x08)?,
            read_f32(em, 0x0C)?,
            read_f32(em, 0x10)?,
        ],
        bone_index: read_u16(em, 0x14)?,
        texture_index: read_u16(em, 0x16)? & 0x1F,
        texture_fdid: None,
        blend_type: em[0x28],
        emitter_type: em[0x29],
        tile_rows: read_u16(em, 0x30)?,
        tile_cols: read_u16(em, 0x32)?,
        emission_speed: 0.0,
        speed_variation: 0.0,
        vertical_range: 0.0,
        horizontal_range: 0.0,
        gravity: 0.0,
        lifespan: 0.0,
        emission_rate: 0.0,
        area_length: 0.0,
        area_width: 0.0,
        colors: [[0.0; 3]; 3],
        opacity: [1.0; 3],
        scales: [[1.0; 2]; 3],
        mid_point: 0.5,
    })
}

/// Fill M2Track-based dynamic values on an emitter.
fn fill_track_values(em: &mut M2ParticleEmitter, md20: &[u8], data: &[u8]) {
    em.emission_speed = read_track_static_f32(md20, data, 0x34);
    em.speed_variation = read_track_static_f32(md20, data, 0x48);
    em.vertical_range = read_track_static_f32(md20, data, 0x5C);
    em.horizontal_range = read_track_static_f32(md20, data, 0x70);
    em.gravity = read_track_static_f32(md20, data, 0x84);
    em.lifespan = read_track_static_f32(md20, data, 0x98);
    em.emission_rate = read_track_static_f32(md20, data, 0xB0);
    em.area_length = read_track_static_f32(md20, data, 0xC8);
    em.area_width = read_track_static_f32(md20, data, 0xDC);
}

/// Fill FakeAnimBlock visual values (color, opacity, scale).
fn fill_visual_values(em: &mut M2ParticleEmitter, md20: &[u8], data: &[u8]) {
    em.mid_point = read_midpoint(md20, data, 0x104);
    em.colors = read_color_values(md20, data, 0x104);
    em.opacity = read_opacity_values(md20, data, 0x114);
    em.scales = read_scale_values(md20, data, 0x124);
}

/// Parse a single particle emitter from the MD20 blob.
fn parse_emitter(md20: &[u8], offset: usize) -> Result<M2ParticleEmitter, String> {
    let data = md20
        .get(offset..)
        .ok_or_else(|| format!("Emitter out of bounds at {offset:#x}"))?;
    if data.len() < 0x134 {
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
    let count = read_u32(md20, 0x128).unwrap_or(0) as usize;
    let offset = read_u32(md20, 0x12C).unwrap_or(0) as usize;
    if count == 0 {
        return Vec::new();
    }
    let stride = 476; // Cata+ emitter struct size
    let mut emitters = Vec::with_capacity(count);
    for i in 0..count {
        match parse_emitter(md20, offset + i * stride) {
            Ok(em) => emitters.push(em),
            Err(e) => eprintln!("Failed to parse particle emitter {i}: {e}"),
        }
    }
    emitters
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_torch_particle_emitter() {
        let path = std::path::Path::new("data/models/club_1h_torch_a_01.m2");
        if !path.exists() {
            return;
        }
        let data = std::fs::read(path).unwrap();
        let md20_size = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
        let md20 = &data[8..8 + md20_size];

        let emitters = parse_particle_emitters(md20);
        assert_eq!(emitters.len(), 1, "torch should have 1 particle emitter");

        let em = &emitters[0];
        assert_eq!(em.bone_index, 10);
        assert_eq!(em.blend_type, 4, "additive blending");
        assert_eq!(em.emitter_type, 1, "sphere emitter");
        assert_eq!(em.tile_rows, 4);
        assert_eq!(em.tile_cols, 4);
        assert!(em.emission_speed > 0.5, "speed={}", em.emission_speed);
        assert!(em.lifespan > 0.7, "lifespan={}", em.lifespan);
        assert!(em.emission_rate > 19.0, "rate={}", em.emission_rate);
        assert!(em.colors[0][0] > 200.0, "start red={}", em.colors[0][0]);
        assert!(em.opacity[1] > 0.9, "mid opacity={}", em.opacity[1]);
    }
}
