//! Retail M2 batch -> material binding, after WebWowViewerCpp 1a8cccb's "Legion logic":
//! `m2Object.cpp` getPixelShaderId / getVertexShaderId / M2ShaderTable /
//! M2BlendingModeToEGxBlendEnum / createM2Material, `M2MeshBufferUpdater.cpp` texture
//! weights and texture-matrix slots, and `animationManager.cpp`
//! calcTextureAnimationTransform. The shader side is `godot/shaders/m2.gdshader`.
use crate::asset::m2_format::m2_anim::{AnimTrack, ColorAnimTracks, TextureAnimTracks};
use crate::asset::m2_texture;
use crate::m2::{Model, TextureUnit};

/// M2ShaderTable: (pixel shader, vertex shader) of `shader_id & 0x7FFF` when 0x8000 is set.
const SHADER_TABLE: [(u8, u8); 36] = [
    (12, 3),
    (13, 3),
    (14, 3),
    (15, 6),
    (16, 3),
    (13, 7),
    (16, 7),
    (17, 3),
    (18, 3),
    (19, 6),
    (20, 7),
    (21, 3),
    (22, 3),
    (23, 3),
    (23, 7),
    (20, 2),
    (24, 3),
    (25, 6),
    (26, 0),
    (33, 9),
    (27, 11),
    (6, 12),
    (28, 2),
    (29, 7),
    (25, 11),
    (33, 13),
    (30, 14),
    (31, 2),
    (32, 14),
    (34, 7),
    (35, 15),
    (35, 16),
    (0, 0),
    (7, 12),
    (1, 9),
    (36, 12),
];

/// Vertex shader that takes its second texture matrix from slot 2 (Diffuse_T1_Env_T2).
const VERTEX_T1_ENV_T2: u8 = 11;

fn table_entry(shader_id: u16) -> Result<(u8, u8), String> {
    SHADER_TABLE
        .get(usize::from(shader_id & 0x7fff))
        .copied()
        .ok_or_else(|| format!("M2 shader {shader_id:#x} is outside M2ShaderTable"))
}

/// getPixelShaderId: the calcM2FragMaterial combiner of a batch.
pub fn pixel_shader_id(texture_count: u16, shader_id: u16) -> Result<u8, String> {
    if shader_id & 0x8000 != 0 {
        return Ok(table_entry(shader_id)?.0);
    }
    if texture_count == 1 {
        return Ok(u8::from(shader_id & 0x70 != 0));
    }
    let modulated = shader_id & 0x70 != 0;
    Ok(match (modulated, shader_id & 7) {
        (true, 0) => 11,               // Combiners_Mod_Opaque
        (true, 3) => 8,                // Combiners_Mod_Add
        (true, 4) => 7,                // Combiners_Mod_Mod2x
        (true, 6) => 9,                // Combiners_Mod_Mod2xNA
        (true, 7) => 10,               // Combiners_Mod_AddNA
        (true, _) => 6,                // Combiners_Mod_Mod
        (false, 0) => 5,               // Combiners_Opaque_Opaque
        (false, 3) | (false, 7) => 13, // Combiners_Opaque_AddAlpha
        (false, 4) => 3,               // Combiners_Opaque_Mod2x
        (false, 6) => 4,               // Combiners_Opaque_Mod2xNA
        (false, _) => 2,               // Combiners_Opaque_Mod
    })
}

/// getVertexShaderId: the calcM2VertexMat texture-coordinate generator of a batch.
pub fn vertex_shader_id(texture_count: u16, shader_id: u16) -> Result<u8, String> {
    if shader_id & 0x8000 != 0 {
        return Ok(table_entry(shader_id)?.1);
    }
    let environment = shader_id & 0x80 != 0;
    Ok(match (texture_count == 1, environment) {
        (true, false) if shader_id & 0x4000 != 0 => 10, // Diffuse_T2
        (true, false) => 0,                             // Diffuse_T1
        (true, true) => 1,                              // Diffuse_Env
        (false, false) if shader_id & 8 != 0 => 3,      // Diffuse_T1_Env
        (false, false) if shader_id & 0x4000 != 0 => 2, // Diffuse_T1_T2
        (false, false) => 7,                            // Diffuse_T1_T1
        (false, true) if shader_id & 8 != 0 => 5,       // Diffuse_Env_Env
        (false, true) => 4,                             // Diffuse_Env_T1
    })
}

/// M2BlendingModeToEGxBlendEnum.
pub fn gx_blend(blend_mode: u16) -> Result<u8, String> {
    const GX: [u8; 8] = [0, 1, 2, 10, 3, 4, 5, 13];
    GX.get(usize::from(blend_mode))
        .copied()
        .ok_or_else(|| format!("M2 blend mode {blend_mode} has no GX blend"))
}

/// Everything a batch's material reads from the model besides its geometry.
#[derive(Clone, Debug, PartialEq)]
pub struct BatchBinding {
    pub pixel_shader: u8,
    pub vertex_shader: u8,
    /// Texture of each sampled slot (the first `min(texture_count, 4)`); `None` when a
    /// replaceable slot has no source.
    pub textures: Vec<Option<u32>>,
    /// M2Texture type of each sampled slot: 0 a file, else the replaceable type a
    /// character or creature binds there (WebWowViewerCpp `getBlpTextureData`).
    pub texture_types: Vec<u32>,
    /// M2Texture wrap flags of slot n at bits 2n (U) and 2n + 1 (V).
    pub texture_wrap: u32,
    /// `Model::texture_animations` index of the shader's two texture matrices.
    pub texture_transforms: [Option<usize>; 2],
    /// `Model::transparency_tracks` index of texture weights 0..2.
    pub texture_weights: [Option<usize>; 3],
    /// `Model::color_tracks` index of the batch colour.
    pub color: Option<usize>,
    /// Mesh opacity includes texture weight 0 unless the batch sets flag 0x40.
    pub apply_weight: bool,
}

fn signed_lookup(table: &[i16], index: usize, len: usize) -> Option<usize> {
    table
        .get(index)
        .and_then(|&value| usize::try_from(value).ok())
        .filter(|&value| value < len)
}

fn slot_texture(
    model: &Model,
    unit: &TextureUnit,
    slot: u16,
    skin_texture_fdids: &[u32],
) -> Result<(Option<u32>, u32, u32), String> {
    let lookup = usize::from(unit.texture_id + slot);
    let index = usize::from(*model.texture_lookup.get(lookup).ok_or_else(|| {
        format!("Batch texture lookup {lookup} outside the texture lookup table")
    })?);
    let texture_type = *model
        .texture_types
        .get(index)
        .ok_or_else(|| format!("Texture lookup {lookup} names absent texture {index}"))?;
    let flags = model.texture_flags.get(index).copied().unwrap_or(0);
    let fdid = match model.texture_fdids.get(index).copied() {
        Some(fdid) if texture_type == 0 && fdid != 0 => Some(fdid),
        _ => m2_texture::default_fdid_for_type(
            texture_type,
            model.skeleton_fdid.is_some(),
            skin_texture_fdids,
        ),
    };
    Ok((fdid, flags & 3, texture_type))
}

/// getTextureMatrixIndexes: texture matrices of the first two texture slots, the second
/// taken from transform slot 2 by Diffuse_T1_Env_T2.
fn texture_transforms(model: &Model, unit: &TextureUnit, vertex_shader: u8) -> [Option<usize>; 2] {
    let slots: [u16; 2] = if vertex_shader == VERTEX_T1_ENV_T2 {
        [0, 2]
    } else {
        [0, 1]
    };
    [0, 1].map(|position| {
        (position < unit.texture_count.min(2))
            .then(|| {
                signed_lookup(
                    &model.uv_animation_lookup,
                    usize::from(unit.texture_animation_id + slots[usize::from(position)]),
                    model.texture_animations.len(),
                )
            })
            .flatten()
    })
}

/// getTextureWeightIndex of texture slots 0..2.
fn texture_weights(model: &Model, unit: &TextureUnit) -> [Option<usize>; 3] {
    [0, 1, 2].map(|slot| {
        (slot < unit.texture_count)
            .then(|| {
                signed_lookup(
                    &model.transparency_lookup,
                    usize::from(unit.transparency_index + slot),
                    model.transparency_tracks.len(),
                )
            })
            .flatten()
    })
}

/// The binding of skin batch `unit` (prepearMaterial's texture slots and
/// updateMaterialData's indices).
pub fn batch_binding(
    model: &Model,
    unit: &TextureUnit,
    skin_texture_fdids: &[u32],
) -> Result<BatchBinding, String> {
    let vertex_shader = vertex_shader_id(unit.texture_count, unit.shader_id)?;
    let mut textures = Vec::new();
    let mut texture_types = Vec::new();
    let mut texture_wrap = 0;
    for slot in 0..unit.texture_count.min(4) {
        let (fdid, wrap, texture_type) = slot_texture(model, unit, slot, skin_texture_fdids)?;
        textures.push(fdid);
        texture_types.push(texture_type);
        texture_wrap |= wrap << (slot * 2);
    }
    Ok(BatchBinding {
        pixel_shader: pixel_shader_id(unit.texture_count, unit.shader_id)?,
        vertex_shader,
        textures,
        texture_types,
        texture_wrap,
        texture_transforms: texture_transforms(model, unit, vertex_shader),
        texture_weights: texture_weights(model, unit),
        color: usize::try_from(unit.color_index)
            .ok()
            .filter(|&index| index < model.color_tracks.len()),
        apply_weight: unit.texture_count > 0 && unit.flags & 0x40 == 0,
    })
}

/// A model's material animation tracks and their timelines, apart from its geometry.
pub struct MaterialTracks {
    pub global_sequences: Vec<u32>,
    /// Duration of sequence 0, the Stand whose loop local material tracks follow.
    pub stand_ms: u32,
    pub colors: Vec<ColorAnimTracks>,
    pub weights: Vec<AnimTrack<i16>>,
    pub transforms: Vec<TextureAnimTracks>,
}

impl MaterialTracks {
    pub fn of(model: &Model) -> Self {
        Self {
            global_sequences: model.global_sequences.clone(),
            stand_ms: model
                .sequences
                .first()
                .map_or(0, |sequence| sequence.duration),
            colors: model.color_tracks.clone(),
            weights: model.transparency_tracks.clone(),
            transforms: model.texture_animations.clone(),
        }
    }

    /// Application time on a track's timeline: global tracks wrap their global
    /// sequence; local tracks loop the Stand the reference keeps playing.
    pub fn track_time<T>(&self, track: &AnimTrack<T>, elapsed_ms: u32) -> u32 {
        let duration = match usize::try_from(track.global_sequence) {
            Ok(index) => self.global_sequences.get(index).copied().unwrap_or(0),
            Err(_) => self.stand_ms,
        };
        if duration == 0 {
            0
        } else {
            elapsed_ms % duration
        }
    }
}

/// animateTrack over sequence 0: interpolation 0 holds a key, 1 interpolates linearly
/// (the only types it supports); before the first key it holds the first.
fn animate<T: Copy>(
    track: &AnimTrack<T>,
    tracks: &MaterialTracks,
    elapsed_ms: u32,
    lerp: impl Fn(T, T, f32) -> T,
) -> Option<T> {
    let (times, values) = track.sequences.first()?;
    if times.is_empty() || values.len() < times.len() {
        return None;
    }
    let time = tracks.track_time(track, elapsed_ms);
    let next = times.partition_point(|&key| key <= time);
    if next == 0 {
        return Some(values[0]);
    }
    if next == times.len() || track.interpolation_type != 1 {
        return Some(values[next - 1]);
    }
    let (start, end) = (times[next - 1], times[next]);
    let t = (time - start) as f32 / (end - start) as f32;
    Some(lerp(values[next - 1], values[next], t))
}

/// A fixed16 track (texture weight, colour alpha) as a 0..1 fraction.
pub fn sample_fixed16(
    track: &AnimTrack<i16>,
    tracks: &MaterialTracks,
    elapsed_ms: u32,
) -> Option<f32> {
    animate(track, tracks, elapsed_ms, |a, b, t| {
        (a as f32 + (b as f32 - a as f32) * t) as i16
    })
    .map(|value| value as f32 / 32767.0)
}

pub fn sample_vec3(
    track: &AnimTrack<[f32; 3]>,
    tracks: &MaterialTracks,
    elapsed_ms: u32,
) -> Option<[f32; 3]> {
    animate(track, tracks, elapsed_ms, |a, b, t| {
        [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
    })
}

fn sample_quat(
    track: &AnimTrack<[f32; 4]>,
    tracks: &MaterialTracks,
    elapsed_ms: u32,
) -> Option<[f32; 4]> {
    animate(track, tracks, elapsed_ms, |a, b, t| {
        let sign = if (0..4).map(|i| a[i] * b[i]).sum::<f32>() < 0.0 {
            -1.0
        } else {
            1.0
        };
        let q = [0, 1, 2, 3].map(|i| a[i] + (sign * b[i] - a[i]) * t);
        let length = q.iter().map(|v| v * v).sum::<f32>().sqrt();
        q.map(|v| v / length)
    })
}

/// The (u, v, 0, 1) -> (u', v') part of a texture matrix, `[m00, m10, m01, m11, tx, ty]`:
/// u' = m00 u + m01 v + tx, v' = m10 u + m11 v + ty.
pub type TextureMatrix = [f32; 6];

pub const IDENTITY_MATRIX: TextureMatrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// Rows of a 3D affine transform (the reference's 4x4 without its constant last row).
type Affine = [[f32; 4]; 3];

const AFFINE_IDENTITY: Affine = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
];

fn compose(a: &Affine, b: &Affine) -> Affine {
    let mut out = [[0.0; 4]; 3];
    for row in 0..3 {
        for column in 0..4 {
            out[row][column] = (0..3).map(|k| a[row][k] * b[k][column]).sum::<f32>()
                + if column == 3 { a[row][3] } else { 0.0 };
        }
    }
    out
}

fn translation(x: f32, y: f32, z: f32) -> Affine {
    [[1.0, 0.0, 0.0, x], [0.0, 1.0, 0.0, y], [0.0, 0.0, 1.0, z]]
}

/// `pivot * linear * -pivot` with the texture centre (0.5, 0.5, 0) as pivot.
fn about_center(matrix: &Affine, linear: &Affine) -> Affine {
    let pivoted = compose(
        &compose(&translation(0.5, 0.5, 0.0), linear),
        &translation(-0.5, -0.5, 0.0),
    );
    compose(matrix, &pivoted)
}

/// calcTextureAnimationTransform: rotation then scale about the texture centre, then
/// translation, each right-multiplied.
pub fn texture_matrix(
    transform: &TextureAnimTracks,
    tracks: &MaterialTracks,
    elapsed_ms: u32,
) -> TextureMatrix {
    let mut matrix = AFFINE_IDENTITY;
    if let Some([x, y, z, w]) = sample_quat(&transform.rotation, tracks, elapsed_ms) {
        let rotation = [
            [
                1.0 - 2.0 * (y * y + z * z),
                2.0 * (x * y - z * w),
                2.0 * (x * z + y * w),
                0.0,
            ],
            [
                2.0 * (x * y + z * w),
                1.0 - 2.0 * (x * x + z * z),
                2.0 * (y * z - x * w),
                0.0,
            ],
            [
                2.0 * (x * z - y * w),
                2.0 * (y * z + x * w),
                1.0 - 2.0 * (x * x + y * y),
                0.0,
            ],
        ];
        matrix = about_center(&matrix, &rotation);
    }
    if let Some([sx, sy, sz]) = sample_vec3(&transform.scale, tracks, elapsed_ms) {
        let scale = [
            [sx, 0.0, 0.0, 0.0],
            [0.0, sy, 0.0, 0.0],
            [0.0, 0.0, sz, 0.0],
        ];
        matrix = about_center(&matrix, &scale);
    }
    if let Some([tx, ty, tz]) = sample_vec3(&transform.translation, tracks, elapsed_ms) {
        matrix = compose(&matrix, &translation(tx, ty, tz));
    }
    [
        matrix[0][0],
        matrix[1][0],
        matrix[0][1],
        matrix[1][1],
        matrix[0][3],
        matrix[1][3],
    ]
}

/// The batch's animated material inputs at `elapsed_ms`.
#[derive(Debug, PartialEq)]
pub struct MaterialSample {
    pub mesh_color: [f32; 3],
    /// meshOpacity: colour alpha times texture weight 0 when the batch applies it.
    pub opacity: f32,
    pub texture_weights: [f32; 3],
    pub texture_matrices: [TextureMatrix; 2],
}

pub fn sample_material(
    tracks: &MaterialTracks,
    binding: &BatchBinding,
    elapsed_ms: u32,
) -> MaterialSample {
    let color = binding.color.map(|index| &tracks.colors[index]);
    let weights = binding.texture_weights.map(|index| {
        index
            .and_then(|index| sample_fixed16(&tracks.weights[index], tracks, elapsed_ms))
            .unwrap_or(1.0)
    });
    let mut opacity = color
        .and_then(|color| sample_fixed16(&color.opacity, tracks, elapsed_ms))
        .unwrap_or(1.0);
    if binding.apply_weight {
        opacity *= weights[0];
    }
    MaterialSample {
        mesh_color: color
            .and_then(|color| sample_vec3(&color.color, tracks, elapsed_ms))
            .unwrap_or([1.0; 3]),
        opacity: opacity.clamp(0.0, 1.0),
        texture_weights: weights,
        texture_matrices: binding.texture_transforms.map(|index| {
            index.map_or(IDENTITY_MATRIX, |index| {
                texture_matrix(&tracks.transforms[index], tracks, elapsed_ms)
            })
        }),
    }
}

/// Some input of the material has more than one key: it changes with time.
pub fn material_animates(tracks: &MaterialTracks, binding: &BatchBinding) -> bool {
    fn keyed<T>(track: &AnimTrack<T>) -> bool {
        track
            .sequences
            .first()
            .is_some_and(|(times, _)| times.len() > 1)
    }
    binding.color.is_some_and(|index| {
        let color = &tracks.colors[index];
        keyed(&color.color) || keyed(&color.opacity)
    }) || binding
        .texture_weights
        .iter()
        .flatten()
        .any(|&index| keyed(&tracks.weights[index]))
        || binding.texture_transforms.iter().flatten().any(|&index| {
            let transform = &tracks.transforms[index];
            keyed(&transform.translation) || keyed(&transform.rotation) || keyed(&transform.scale)
        })
}
