//! CPU simulation of M2 particle emitters (`ParticleSystem2`), engine-free.
//!
//! Follows WebWowViewerCpp `managers/particles/particleEmitter.cpp` (update, emission,
//! forces, lifetime appearance, quad building) and its `CPlaneGenerator` /
//! `CSphereGenerator` spawn rules; pool sizing follows solarityclient
//! `particle_system2/simulation.rs`. Values stay in the emitter's authored units: a
//! model-space emitter (flag 0x10) simulates in its generator frame, any other in
//! the caller's world axes, into which the caller converts WoW gravity and wind.
use glam::{Mat3, Mat4, Vec2, Vec3, Vec4};

use crate::asset::m2_format::m2_anim;
use crate::m2::{AnimTime, ParticleEmitter};

const FLAG_VELOCITY_ORIENTED: u32 = 0x4;
const FLAG_MODEL_SPACE: u32 = 0x10;
const FLAG_INHERIT_SCALE: u32 = 0x20;
const FLAG_SPHERE_IMPLODE: u32 = 0x80;
const FLAG_SPHERE_UP: u32 = 0x100;
const FLAG_SPIN_FLIP: u32 = 0x200;
const FLAG_LOCAL_ORIENTATION: u32 = 0x1000;
const FLAG_RANDOM_CELL: u32 = 0x1_0000;
const FLAG_HEAD: u32 = 0x2_0000;
const FLAG_SCALE_VARY_2D: u32 = 0x8_0000;
/// Emission ignores the particle-density setting (Bevy `PARTICLE_FLAG_NO_GLOBAL_SCALE`).
pub const FLAG_NO_GLOBAL_SCALE: u32 = 0x0200_0000;
const FLAG_MULTITEXTURE: u32 = 0x1000_0000;
/// With multitexturing, all three textures modulate colour (material flag 0x20).
const FLAG_THREE_COLOR: u32 = 0x4000_0000;

/// WebWowViewerCpp renders at most 500 quads per emitter (`particleEmitter.cpp:462-464`,
/// `MAX_PARTICLES_PER_EMITTER` vertices / 4).
pub const EMITTER_CAPACITY_CAP: usize = 500;
/// Build 12340 overprovisions pools by 1.15 (`0x009F23CC`, solarityclient
/// `STOCK_CAPACITY_HEADROOM`).
const CAPACITY_HEADROOM: f64 = 1.149_999_976_158_142;
/// Updates advance in slices of at most 0.1 s (`particleEmitter.cpp:541-568`).
const STEP_SECONDS: f32 = 0.1;
const SEED_SCALE: f32 = 0.000_030_518_509;
const MIN_LIFESPAN: f32 = 0.001;
const MIN_SCALE_MULTIPLIER: f32 = 0.000_099_999_997;
const DEGENERATE_LENGTH_SQUARED: f32 = 0.000_000_238_418_58;

/// `CRndSeed::uint32t` noise bytes (WebWowViewerCpp `CRndSeed.cpp:6-33`).
const NOISE: [u8; 256] = [
    0x8E, 0x14, 0x27, 0x99, 0xFD, 0xAA, 0xC7, 0x08, 0xD5, 0xE6, 0x3E, 0x1F, 0xF6, 0xBB, 0x55, 0xDA,
    0x75, 0xA0, 0x4A, 0x6A, 0xE8, 0xBD, 0x97, 0xFF, 0xDE, 0x9B, 0xBC, 0x9F, 0x81, 0x8A, 0xA1, 0x46,
    0x6E, 0x0B, 0xE3, 0x63, 0x76, 0x7A, 0x6C, 0x5D, 0x88, 0xD3, 0x69, 0xCA, 0xC3, 0x47, 0xB9, 0x25,
    0x83, 0xAB, 0xA2, 0x3F, 0xA6, 0x41, 0x7C, 0xBA, 0xE5, 0xAC, 0x95, 0x01, 0x7E, 0xCF, 0x09, 0xC1,
    0xD9, 0x62, 0x70, 0x71, 0x8D, 0xDB, 0x05, 0x02, 0x24, 0x87, 0xEF, 0x54, 0xC6, 0xD4, 0x37, 0x30,
    0xD0, 0x1B, 0xCB, 0x7B, 0xB8, 0xE4, 0xD8, 0xEC, 0x49, 0xCE, 0xAD, 0xDC, 0x13, 0xA9, 0x94, 0xC4,
    0x8F, 0x39, 0xAE, 0x0D, 0x18, 0x52, 0xDD, 0x0E, 0x78, 0xFA, 0xF5, 0x85, 0x58, 0xD2, 0xAF, 0x6D,
    0xA4, 0xB2, 0x53, 0x3B, 0x51, 0xA5, 0x50, 0xBE, 0xFC, 0x2D, 0xF4, 0x11, 0x48, 0x98, 0x16, 0xF1,
    0x86, 0xDF, 0x3D, 0x66, 0x5E, 0x44, 0x2E, 0x2F, 0x36, 0x07, 0x6B, 0x17, 0x8B, 0x29, 0x4C, 0xB6,
    0xE2, 0x89, 0x5F, 0xE7, 0xCD, 0xA7, 0x21, 0xE1, 0x4D, 0xC9, 0x65, 0xED, 0xFE, 0xEE, 0x9C, 0x23,
    0x33, 0x7D, 0xB7, 0x04, 0x9E, 0x9A, 0x2A, 0x40, 0xB3, 0x10, 0x5B, 0xF3, 0x82, 0x77, 0x1C, 0x92,
    0x20, 0x4E, 0x1E, 0x57, 0x22, 0x72, 0x06, 0x8C, 0x67, 0x2C, 0x73, 0xFB, 0x59, 0xC2, 0x0A, 0xBF,
    0x79, 0x5C, 0xF9, 0x0C, 0x28, 0x1A, 0x12, 0x68, 0x74, 0x34, 0x19, 0x42, 0xB1, 0xC0, 0x84, 0xF8,
    0x38, 0xF0, 0x15, 0x9D, 0x60, 0xF2, 0x3A, 0x6F, 0xB4, 0x90, 0xEB, 0x91, 0x1D, 0x7F, 0x35, 0x61,
    0x5A, 0x32, 0x03, 0x56, 0xA3, 0xC5, 0x2B, 0x93, 0x80, 0x0F, 0x4B, 0x43, 0xF7, 0xA8, 0xE0, 0x3C,
    0x96, 0xD1, 0x64, 0x26, 0xD7, 0x45, 0xCC, 0x4F, 0xC8, 0xB0, 0xE9, 0xB5, 0x00, 0xD6, 0x31, 0xEA,
];

/// The client's particle random stream (`CRndSeed`).
#[derive(Clone, Copy, Debug)]
pub struct Rng {
    current: u32,
    accumulator: u32,
}

impl Rng {
    pub fn new(seed: u32) -> Self {
        Self {
            current: seed,
            accumulator: 0x0FEE_FDFA ^ seed,
        }
    }

    pub fn next_u32(&mut self) -> u32 {
        let [byte0, byte1, byte2, byte3] = self.current.to_le_bytes();
        let wrap = |byte: u8, minus: i32, modulo: i32| {
            let index = i32::from(byte) - minus;
            (if index < 0 { index + modulo } else { index }) as usize
        };
        let idx4 = wrap(byte0, 28, 244);
        let idx1 = wrap(byte3, 4, 188);
        let idx2 = wrap(byte2, 12, 212);
        let idx3 = wrap(byte1, 24, 236);
        let word = |at: usize| {
            u32::from_le_bytes([NOISE[at], NOISE[at + 1], NOISE[at + 2], NOISE[at + 3]])
        };
        // C++ precedence: (accumulator + val4) ^ rotl(val1, 1) ^ rotl(val2, 2) ^ rotl(val3, 3).
        self.accumulator = self.accumulator.wrapping_add(word(idx4))
            ^ word(idx1).rotate_left(1)
            ^ word(idx2).rotate_left(2)
            ^ word(idx3).rotate_left(3);
        self.current = (idx1 as u32) << 24 | (idx2 as u32) << 16 | (idx3 as u32) << 8 | idx4 as u32;
        self.accumulator
    }

    /// In [-1, 1].
    pub fn uniform(&mut self) -> f32 {
        let random = self.next_u32();
        let magnitude = f32::from_bits(0x3F80_0000 | (random & 0x007F_FFFF));
        if random & 0x8000_0000 != 0 {
            2.0 - magnitude
        } else {
            magnitude - 2.0
        }
    }

    /// In [0, 1).
    pub fn uniform_pos(&mut self) -> f32 {
        f32::from_bits(0x3F80_0000 | (self.next_u32() & 0x007F_FFFF)) - 1.0
    }
}

/// Emitter generator, from the authored emitter type byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticleShape {
    Plane,
    Sphere,
}

/// `CPlaneGenerator` for type 1, `CSphereGenerator` for 2; spline (3) and others are
/// not simulated (`particleEmitter.cpp:128-149`).
pub fn shape(emitter: &ParticleEmitter) -> Option<ParticleShape> {
    match emitter.emitter_type {
        1 => Some(ParticleShape::Plane),
        2 => Some(ParticleShape::Sphere),
        _ => None,
    }
}

/// Particle fragment combiner (`m2ParticleShader.frag.slang:53-80`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelShader {
    /// colour x texture 0.
    Mod,
    /// colour x textures 0 and 1, alpha also x texture 2's.
    TwoColorThreeAlpha,
    /// colour x textures 0, 1 and 2.
    ThreeColorThreeAlpha,
}

/// `ParticleEmitter::selectShaderId` for emitters without a TXAC value.
pub fn pixel_shader(emitter: &ParticleEmitter) -> PixelShader {
    if emitter.flags & FLAG_MULTITEXTURE == 0 {
        PixelShader::Mod
    } else if emitter.flags & FLAG_THREE_COLOR != 0 {
        PixelShader::ThreeColorThreeAlpha
    } else {
        PixelShader::TwoColorThreeAlpha
    }
}

/// Opaque and alpha-key particles write depth (`particleEmitter.cpp:275`).
pub fn writes_depth(blend_type: u8) -> bool {
    blend_type <= 1
}

/// Fragment alpha test by blend type (`particleEmitter.cpp:310-316`).
pub fn alpha_test(blend_type: u8) -> f32 {
    match blend_type {
        0 => -1.0,
        1 => 0.501_960_8,
        _ => 0.003_921_569,
    }
}

/// UV scale of multitexture layer `layer` from its packed byte
/// (`paramXTransform`, `particleEmitter.cpp:1269-1271`).
pub fn multitexture_uv_scale(emitter: &ParticleEmitter, layer: usize) -> f32 {
    let Some(multi) = &emitter.multi_texture else {
        return 1.0;
    };
    let packed = u32::from(multi.uv_scale_bytes[layer]);
    (packed & 0x1F) as f32 / 32.0 + (packed >> 5) as f32
}

/// Particle slots for one emitter: authored maximum rate x maximum lifetime x 1.15,
/// capped at [`EMITTER_CAPACITY_CAP`]. Keyframed rate and lifespan size the pool for
/// their highest keys (WebWowViewerCpp M2GpuAnimData.cpp:387-391).
pub fn pool_capacity(emitter: &ParticleEmitter) -> usize {
    let peak = |track: &Option<m2_anim::AnimTrack<f32>>, authored: f32| {
        track
            .iter()
            .flat_map(|track| track.sequences.iter().flat_map(|(_, values)| values))
            .fold(authored, |peak, &value| peak.max(value))
    };
    let tracks = &emitter.tracks;
    let rate = f64::from(peak(&tracks.emission_rate, emitter.emission_rate))
        + f64::from(emitter.emission_rate_variation);
    let life =
        f64::from(peak(&tracks.lifespan, emitter.lifespan)) + f64::from(emitter.lifespan_variation);
    let estimate = (rate * life * CAPACITY_HEADROOM).ceil();
    if estimate.is_finite() && estimate > 0.0 {
        (estimate as usize).min(EMITTER_CAPACITY_CAP)
    } else {
        0
    }
}

/// Values interpolated along a lifetime ramp.
pub trait Lerp: Copy {
    fn lerp(self, to: Self, amount: f32) -> Self;
}

impl Lerp for f32 {
    fn lerp(self, to: Self, amount: f32) -> Self {
        self + (to - self) * amount
    }
}

impl<const N: usize> Lerp for [f32; N] {
    fn lerp(self, to: Self, amount: f32) -> Self {
        std::array::from_fn(|index| self[index].lerp(to[index], amount))
    }
}

impl Lerp for u16 {
    /// `lerpHelper<uint16_t>` truncates the interpolated cell.
    fn lerp(self, to: Self, amount: f32) -> Self {
        f32::from(self).lerp(f32::from(to), amount) as u16
    }
}

/// `animatePartTrack`: linear between the keys around `t`, clamped to the end keys;
/// `default` without keys.
pub fn sample_keys<T: Lerp>(keys: &[(f32, T)], t: f32, default: T) -> T {
    let Some(&(first_time, first)) = keys.first() else {
        return default;
    };
    if t <= first_time {
        return first;
    }
    for pair in keys.windows(2) {
        let ((from_time, from), (to_time, to)) = (pair[0], pair[1]);
        if t <= to_time {
            let span = to_time - from_time;
            let amount = if span > 0.0 {
                (t - from_time) / span
            } else {
                1.0
            };
            return from.lerp(to, amount);
        }
    }
    keys[keys.len() - 1].1
}

/// A particle's lifespan: the generator's current `lifespan` varied by the particle's
/// signed seed (`CParticleGenerator::GetLifeSpan`).
pub fn particle_lifespan(emitter: &ParticleEmitter, lifespan: f32, seed: u16) -> f32 {
    lifespan + f32::from(seed as i16) * SEED_SCALE * emitter.lifespan_variation
}

/// `CParticleGenerator::GetMaxLifeSpan` for the generator's current `lifespan`.
fn max_lifespan(emitter: &ParticleEmitter, lifespan: f32) -> f32 {
    lifespan + emitter.lifespan_variation
}

/// Lifetime colour, opacity, size and atlas cell of one particle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Appearance {
    /// Authored colour / 255 x EXP2 colour multiplier.
    pub color: Vec3,
    pub alpha: f32,
    /// Half extents along the quad axes.
    pub scale: Vec2,
    pub head_cell: u32,
}

/// `ParticleEmitter::fillTimedParticleData` at `age` seconds: ramps sampled at age over
/// the maximum lifespan for the generator's current `lifespan`; random cell and size
/// variation from the particle seed.
pub fn appearance(emitter: &ParticleEmitter, lifespan: f32, age: f32, seed: u16) -> Appearance {
    let max_life = max_lifespan(emitter, lifespan);
    let t = if max_life > 0.0 {
        (age / max_life).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let mut rand = Rng::new(u32::from(seed));
    let color = Vec3::from(sample_keys(&emitter.color_keys, t, [255.0; 3])) / 255.0;
    let mut scale = Vec2::from(sample_keys(&emitter.scale_keys, t, [1.0; 2]));
    let alpha = sample_keys(&emitter.opacity_keys, t, 1.0);
    let cells = u32::from(emitter.tile_rows.max(1)) * u32::from(emitter.tile_cols.max(1));
    let head_cell = if !emitter.head_cell_keys.is_empty() {
        u32::from(sample_keys(&emitter.head_cell_keys, t, 0)) & (cells - 1)
    } else if emitter.flags & FLAG_RANDOM_CELL != 0 {
        ((u64::from(cells) * u64::from(rand.next_u32())) >> 32) as u32
    } else {
        0
    };
    let vary =
        |rand: &mut Rng, amount: f32| (1.0 + rand.uniform() * amount).max(MIN_SCALE_MULTIPLIER);
    if emitter.flags & FLAG_SCALE_VARY_2D != 0 {
        let y = vary(&mut rand, emitter.scale_variation_y);
        scale *= Vec2::new(vary(&mut rand, emitter.scale_variation), y);
    } else {
        scale *= vary(&mut rand, emitter.scale_variation);
    }
    Appearance {
        color: color * emitter.color_mult,
        alpha: alpha * emitter.alpha_mult,
        scale,
        head_cell,
    }
}

/// Process-wide twinkle phases (WebWowViewerCpp fills `RandTable` from `rand()`;
/// a fixed stream keeps runs reproducible).
fn twinkle_table() -> &'static [f32; 128] {
    static TABLE: std::sync::OnceLock<[f32; 128]> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let mut rng = Rng::new(0x5EED_1234);
        std::array::from_fn(|_| rng.uniform_pos())
    })
}

/// Twinkle: `None` hides the particle this frame, else its size multiplier
/// (`CalculateParticlePreRenderData`, `particleEmitter.cpp:1122-1148`).
pub fn twinkle_scale(emitter: &ParticleEmitter, age: f32, seed: u16) -> Option<f32> {
    let vary = emitter.twinkle_scale_max - emitter.twinkle_scale_min;
    let index = if emitter.twinkle_percent < 1.0 || vary != 0.0 {
        ((age * emitter.twinkle_speed) as i32).wrapping_add(i32::from(seed)) as usize & 0x7F
    } else {
        0
    };
    let phase = twinkle_table()[index];
    (emitter.twinkle_percent >= phase).then_some(vary * phase + emitter.twinkle_scale_min)
}

/// One ballistic step: position += v dt + g dt^2 / 2, v += g dt, v *= 1 - min(drag dt, 1)
/// (`UpdateParticle`, `CalculateForces`).
pub fn integrate_particle(
    position: Vec3,
    velocity: Vec3,
    dt: f32,
    gravity: Vec3,
    drag: f32,
) -> (Vec3, Vec3) {
    let position = position + velocity * dt + gravity * (dt * dt * 0.5);
    let velocity = (velocity + gravity * dt) * (1.0 - (drag * dt).min(1.0));
    (position, velocity)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Particle {
    /// Generator frame for model-space emitters, else world.
    pub position: Vec3,
    pub velocity: Vec3,
    pub age: f32,
    pub seed: u16,
    tex_pos: [Vec2; 2],
    tex_vel: [Vec2; 2],
}

/// Camera axes in world space.
#[derive(Clone, Copy, Debug)]
pub struct ViewBasis {
    pub right: Vec3,
    pub up: Vec3,
    /// Opposite the view direction.
    pub back: Vec3,
}

/// One drawn particle: corners at `center +- axis_x +- axis_y` in world space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad {
    pub center: Vec3,
    pub axis_x: Vec3,
    pub axis_y: Vec3,
    /// Authored-space (gamma) RGB and opacity.
    pub color: Vec4,
    /// Atlas offset of the head cell, in UV units.
    pub cell_offset: Vec2,
    /// UV offsets of multitexture layers 1 and 2.
    pub tex_pos: [Vec2; 2],
}

/// Live particles of one placed emitter, its emission remainder and random stream.
#[derive(Clone, Debug)]
pub struct EmitterSim {
    rng: Rng,
    emission: f32,
    particles: Vec<Particle>,
    capacity: usize,
    emitter_to_world: Mat4,
    /// Generator values at the model's playing sequence time.
    props: GeneratorProps,
    /// `enabledIn` at that time.
    enabled: bool,
}

/// The generator's animated values (`CGeneratorAniProp`); gravity in WoW axes.
#[derive(Clone, Copy, Debug, PartialEq)]
struct GeneratorProps {
    emission_speed: f32,
    speed_variation: f32,
    vertical_range: f32,
    horizontal_range: f32,
    gravity: [f32; 3],
    lifespan: f32,
    emission_rate: f32,
    area_length: f32,
    area_width: f32,
    z_source: f32,
}

impl GeneratorProps {
    /// The emitter's static values.
    fn authored(emitter: &ParticleEmitter) -> Self {
        Self {
            emission_speed: emitter.emission_speed,
            speed_variation: emitter.speed_variation,
            vertical_range: emitter.vertical_range,
            horizontal_range: emitter.horizontal_range,
            gravity: emitter.gravity_vector,
            lifespan: emitter.lifespan,
            emission_rate: emitter.emission_rate,
            area_length: emitter.area_length,
            area_width: emitter.area_width,
            z_source: emitter.z_source,
        }
    }

    /// Each keyframed track at `time`; the static value where a track is unkeyed or
    /// has no keys in the sampled sequence.
    fn at(emitter: &ParticleEmitter, time: &AnimTime) -> Self {
        let tracks = &emitter.tracks;
        let f32_at = |track: &Option<m2_anim::AnimTrack<f32>>, authored: f32| {
            track
                .as_ref()
                .and_then(|track| {
                    let (timeline, time_ms) = time.track_time(track);
                    m2_anim::evaluate_f32_track(track, timeline, time_ms)
                })
                .unwrap_or(authored)
        };
        let gravity = tracks
            .gravity
            .as_ref()
            .and_then(|track| {
                let (timeline, time_ms) = time.track_time(track);
                m2_anim::evaluate_vec3_track(track, timeline, time_ms)
            })
            .unwrap_or(emitter.gravity_vector);
        Self {
            emission_speed: f32_at(&tracks.emission_speed, emitter.emission_speed),
            speed_variation: f32_at(&tracks.speed_variation, emitter.speed_variation),
            vertical_range: f32_at(&tracks.vertical_range, emitter.vertical_range),
            horizontal_range: f32_at(&tracks.horizontal_range, emitter.horizontal_range),
            gravity,
            lifespan: f32_at(&tracks.lifespan, emitter.lifespan),
            emission_rate: f32_at(&tracks.emission_rate, emitter.emission_rate),
            area_length: f32_at(&tracks.area_length, emitter.area_length),
            area_width: f32_at(&tracks.area_width, emitter.area_width),
            z_source: f32_at(&tracks.z_source, emitter.z_source),
        }
    }
}

impl EmitterSim {
    pub fn new(emitter: &ParticleEmitter, seed: u32) -> Self {
        let capacity = pool_capacity(emitter);
        Self {
            rng: Rng::new(seed),
            emission: 0.0,
            particles: Vec::with_capacity(capacity),
            capacity,
            emitter_to_world: Mat4::IDENTITY,
            props: GeneratorProps::authored(emitter),
            enabled: true,
        }
    }

    /// Evaluate the emitter's keyframed tracks at the model's animation `time`; used
    /// by the next updates. A disabled emitter keeps its last generator values
    /// (WebWowViewerCpp animationManager.cpp:1349-1470 `calcParticleEmitters`).
    pub fn set_animation(&mut self, emitter: &ParticleEmitter, time: &AnimTime) {
        self.enabled = emitter
            .tracks
            .enabled
            .as_ref()
            .and_then(|track| {
                let (timeline, time_ms) = time.track_time(track);
                m2_anim::evaluate_u8_track(track, timeline, time_ms)
            })
            .is_none_or(|enabled| enabled != 0);
        if self.enabled {
            self.props = GeneratorProps::at(emitter, time);
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }

    /// Advances `dt` seconds with the emitter's generator frame at `emitter_to_world`;
    /// `wow_to_world` rotates WoW-axis gravity and wind into world axes and `density`
    /// scales the emission rate. Long updates replay whole 0.1 s steps only up to
    /// one lifespan (`ParticleEmitter::InternalUpdate`).
    pub fn update(
        &mut self,
        emitter: &ParticleEmitter,
        dt: f32,
        emitter_to_world: Mat4,
        wow_to_world: Mat3,
        density: f32,
    ) {
        if shape(emitter).is_none() || !(dt > 0.0) {
            return;
        }
        self.emitter_to_world = emitter_to_world;
        let mut remainder = dt;
        if dt >= STEP_SECONDS {
            let whole = (dt / STEP_SECONDS).floor();
            remainder = dt - whole * STEP_SECONDS;
            let steps = (self.props.lifespan / STEP_SECONDS)
                .floor()
                .min(whole)
                .max(0.0) as usize;
            for _ in 0..steps {
                self.step(emitter, STEP_SECONDS, wow_to_world, density);
            }
        }
        self.step(emitter, remainder, wow_to_world, density);
    }

    /// `ParticleEmitter::StepUpdate`: age and retire, emit, then move every particle.
    fn step(&mut self, emitter: &ParticleEmitter, dt: f32, wow_to_world: Mat3, density: f32) {
        if dt < 0.0 {
            return;
        }
        let lifespan = self.props.lifespan;
        self.particles.retain_mut(|particle| {
            particle.age += dt;
            particle.age <= particle_lifespan(emitter, lifespan, particle.seed).max(MIN_LIFESPAN)
        });
        let model_space = emitter.flags & FLAG_MODEL_SPACE != 0;
        let to_sim = |vector: [f32; 3]| {
            let vector = Vec3::from(vector);
            if model_space {
                vector
            } else {
                wow_to_world * vector
            }
        };
        let gravity = to_sim(self.props.gravity);
        let wind = to_sim(emitter.wind_vector);
        self.emit(emitter, dt, density);
        let center = if model_space {
            Vec3::ZERO
        } else {
            self.emitter_to_world.w_axis.truncate()
        };
        let implode = shape(emitter) == Some(ParticleShape::Sphere)
            && emitter.flags & FLAG_SPHERE_IMPLODE != 0;
        let multitexture = emitter.multi_texture.is_some();
        self.particles.retain_mut(|particle| {
            if multitexture {
                for layer in 0..2 {
                    let moved = particle.tex_pos[layer] + particle.tex_vel[layer] * dt;
                    particle.tex_pos[layer] = moved - moved.floor();
                }
            }
            // solarityclient `simulation.rs`: wind acts while age < windTime.
            if particle.age < emitter.wind_time {
                particle.velocity += wind * dt;
            }
            let step = particle.velocity * dt;
            (particle.position, particle.velocity) = integrate_particle(
                particle.position,
                particle.velocity,
                dt,
                gravity,
                emitter.drag,
            );
            !(implode && (particle.position - center).dot(step) > 0.0)
        });
    }

    /// `EmitNewParticles`: rate (+ variation) x density accumulates; each whole
    /// particle spawns while the pool has room.
    fn emit(&mut self, emitter: &ParticleEmitter, dt: f32, density: f32) {
        if !self.enabled {
            return;
        }
        let rate = (self.props.emission_rate
            + self.rng.uniform() * emitter.emission_rate_variation)
            * density;
        self.emission += dt * rate;
        while self.emission > 1.0 {
            if self.particles.len() < self.capacity {
                let particle = self.spawn(emitter, dt);
                self.particles.push(particle);
            }
            self.emission -= 1.0;
        }
    }

    fn spawn(&mut self, emitter: &ParticleEmitter, dt: f32) -> Particle {
        let rng = &mut self.rng;
        let dvary = dt * rng.uniform_pos();
        let life = rng.uniform();
        let state = (life * 32767.0 + 0.5).trunc().clamp(-32767.0, 32767.0) as i16;
        let props = &self.props;
        let lifespan = particle_lifespan(emitter, props.lifespan, state as u16).max(MIN_LIFESPAN);
        let age = dvary % lifespan;
        let seed = rng.next_u32() as u16;
        let (position, velocity) = match shape(emitter) {
            Some(ParticleShape::Plane) => spawn_plane(props, rng),
            _ => spawn_sphere(emitter, props, rng),
        };
        let (position, velocity) = if emitter.flags & FLAG_MODEL_SPACE == 0 {
            (
                self.emitter_to_world.transform_point3(position),
                self.emitter_to_world.transform_vector3(velocity),
            )
        } else {
            (position, velocity)
        };
        let (tex_pos, tex_vel) = spawn_texture_motion(emitter, rng);
        Particle {
            position,
            velocity,
            age,
            seed,
            tex_pos,
            tex_vel,
        }
    }

    /// Visible particles as world-space quads (`prepearAndUpdateBuffers`,
    /// `buildVertex1`): head quads only, twinkle-culled.
    pub fn quads(&self, emitter: &ParticleEmitter, view: &ViewBasis, out: &mut Vec<Quad>) {
        if emitter.flags & FLAG_HEAD == 0 {
            return;
        }
        let max_life = max_lifespan(emitter, self.props.lifespan);
        out.extend(
            self.particles
                .iter()
                .filter(|particle| particle.age <= max_life)
                .filter_map(|particle| self.quad(emitter, particle, view)),
        );
    }

    fn quad(
        &self,
        emitter: &ParticleEmitter,
        particle: &Particle,
        view: &ViewBasis,
    ) -> Option<Quad> {
        let twinkle = twinkle_scale(emitter, particle.age, particle.seed)?;
        let matrix = self.emitter_to_world;
        let look = appearance(emitter, self.props.lifespan, particle.age, particle.seed);
        let mut scale = look.scale * twinkle;
        if emitter.flags & FLAG_INHERIT_SCALE != 0 {
            scale *= matrix.x_axis.truncate().length();
        }
        let (center, velocity) = if emitter.flags & FLAG_MODEL_SPACE != 0 {
            (
                matrix.transform_point3(particle.position),
                matrix.transform_vector3(particle.velocity),
            )
        } else {
            (particle.position, particle.velocity)
        };
        let (axis_x, axis_y) = quad_axes(emitter, particle, matrix, velocity, scale, view);
        let cols = u32::from(emitter.tile_cols.max(1));
        let rows = u32::from(emitter.tile_rows.max(1));
        Some(Quad {
            center,
            axis_x,
            axis_y,
            color: look.color.extend(look.alpha),
            cell_offset: Vec2::new(
                (look.head_cell % cols) as f32 / cols as f32,
                (look.head_cell / cols % rows) as f32 / rows as f32,
            ),
            tex_pos: particle.tex_pos,
        })
    }
}

/// Multitexture layer 1 and 2 UV offsets and scroll velocities (`CreateParticle`).
fn spawn_texture_motion(emitter: &ParticleEmitter, rng: &mut Rng) -> ([Vec2; 2], [Vec2; 2]) {
    let mut tex_pos = [Vec2::ZERO; 2];
    let mut tex_vel = [Vec2::ZERO; 2];
    if let Some(multi) = &emitter.multi_texture {
        for layer in 0..2 {
            tex_pos[layer] = Vec2::new(rng.uniform_pos(), rng.uniform_pos());
            tex_vel[layer] = Vec2::from(multi.velocity_ranges[layer]) * rng.uniform()
                + Vec2::from(multi.velocity_midpoints[layer]);
        }
    }
    (tex_pos, tex_vel)
}

/// `CPlaneGenerator::CreateParticle` in the generator frame.
fn spawn_plane(props: &GeneratorProps, rng: &mut Rng) -> (Vec3, Vec3) {
    let position = Vec3::new(
        rng.uniform() * props.area_length * 0.5,
        rng.uniform() * props.area_width * 0.5,
        0.0,
    );
    let speed = props.emission_speed * (1.0 + props.speed_variation * rng.uniform());
    let velocity = if props.z_source < 0.001 {
        let polar = props.vertical_range * rng.uniform();
        let azimuth = props.horizontal_range * rng.uniform();
        Vec3::new(
            azimuth.cos() * polar.sin(),
            azimuth.sin() * polar.sin(),
            polar.cos(),
        ) * speed
    } else {
        aim_from_z_source(position, props.z_source) * speed
    };
    (position, velocity)
}

/// `CSphereGenerator::CreateParticle`: a shell between the authored length and width
/// radii, launched outward (or up with flag 0x100, or from the z source).
fn spawn_sphere(emitter: &ParticleEmitter, props: &GeneratorProps, rng: &mut Rng) -> (Vec3, Vec3) {
    let radius = props.area_length + (props.area_width - props.area_length) * rng.uniform_pos();
    let polar = props.vertical_range * rng.uniform();
    let azimuth = props.horizontal_range * rng.uniform();
    let direction = Vec3::new(
        polar.cos() * azimuth.cos(),
        polar.cos() * azimuth.sin(),
        polar.sin(),
    );
    let position = direction * radius;
    let speed = props.emission_speed * (1.0 + props.speed_variation * rng.uniform());
    let aim = if props.z_source == 0.0 {
        if emitter.flags & FLAG_SPHERE_UP != 0 {
            Vec3::Z
        } else {
            direction
        }
    } else {
        aim_from_z_source(position, props.z_source)
    };
    (position, aim * speed)
}

fn aim_from_z_source(position: Vec3, z_source: f32) -> Vec3 {
    let aim = position - Vec3::new(0.0, 0.0, z_source);
    if aim.length() > 0.0001 {
        aim.normalize()
    } else {
        Vec3::ZERO
    }
}

/// Per-particle base spin and angular speed (`ParticleEmitter::GetSpin`).
fn spin(emitter: &ParticleEmitter, seed: u16) -> (f32, f32) {
    if emitter.base_spin == 0.0 && emitter.spin_variation == 0.0 {
        return (emitter.base_spin, emitter.spin);
    }
    let mut rand = Rng::new(u32::from(seed));
    let base = if emitter.base_spin_variation == 0.0 {
        emitter.base_spin
    } else {
        emitter.base_spin + rand.uniform() * emitter.base_spin_variation
    };
    let speed = if emitter.spin_variation == 0.0 {
        emitter.spin
    } else {
        emitter.spin + rand.uniform() * emitter.spin_variation
    };
    (base, speed)
}

/// Screen-space velocity streak: x along -velocity, y perpendicular on screen.
fn velocity_axes(velocity: Vec3, scale: Vec2, view: &ViewBasis) -> (Vec3, Vec3) {
    let back = -velocity;
    let screen =
        Vec3::new(back.dot(view.right), back.dot(view.up), back.dot(view.back)).normalize_or_zero();
    let to_world = |v: Vec3| view.right * v.x + view.up * v.y + view.back * v.z;
    (
        to_world(screen * scale.x),
        to_world(Vec3::new(screen.y * scale.y, -screen.x * scale.y, 0.0)),
    )
}

/// In-plane rotation at the particle's age; flag 0x200 reverses odd seeds.
fn spin_angle(emitter: &ParticleEmitter, particle: &Particle) -> f32 {
    if emitter.spin == 0.0 && emitter.spin_variation == 0.0 {
        return 0.0;
    }
    let (base, speed) = spin(emitter, particle.seed);
    let theta = base + speed * particle.age;
    if emitter.flags & FLAG_SPIN_FLIP != 0 && particle.seed & 1 != 0 {
        -theta
    } else {
        theta
    }
}

/// Quad half-axes (`buildVertex1`): along the velocity on screen (0x4), in the emitter's
/// XY plane (0x1000), or facing the camera; spun in their plane when authored.
fn quad_axes(
    emitter: &ParticleEmitter,
    particle: &Particle,
    matrix: Mat4,
    velocity: Vec3,
    scale: Vec2,
    view: &ViewBasis,
) -> (Vec3, Vec3) {
    let local = emitter.flags & FLAG_LOCAL_ORIENTATION != 0;
    if emitter.flags & FLAG_VELOCITY_ORIENTED != 0
        && !local
        && velocity.length_squared() > DEGENERATE_LENGTH_SQUARED
    {
        return velocity_axes(velocity, scale, view);
    }
    let (sin, cos) = spin_angle(emitter, particle).sin_cos();
    let (right, up) = if local {
        (
            matrix.x_axis.truncate().normalize_or_zero(),
            matrix.y_axis.truncate().normalize_or_zero(),
        )
    } else {
        (view.right, view.up)
    };
    (
        (right * cos + up * sin) * scale.x,
        (up * cos - right * sin) * scale.y,
    )
}
