//! Terrain detail doodads (ground clutter): the grass and flower cards a terrain layer's
//! GroundEffectTexture scatters over its MCNK cells.
//!
//! Port of solarityclient `crates/rendering/src/terrain/detail_doodad` (build 12340
//! functions 7D3390 scatter, 7B1B50 vertex expansion, 7B31E0 texture buckets), checked
//! against its native fixtures in `tests/ground_detail.rs`. Positions and normals are WoW
//! axes relative to the chunk's first vertex (x north, y west, z up).

use std::collections::HashMap;

use glam::{Mat3, Vec3};

use crate::{
    adt,
    lighting_assets::{CsvRow, csv_rows},
    m2,
};

/// One MCNK cell (and doodad quad) edge, in yards.
const UNIT: f32 = 4.166_666_5;
const HALF_UNIT: f32 = 2.083_333_3;
/// The two outer corners of each of a cell's four faces, relative to the cell's first vertex.
const CORNERS: [(usize, usize); 4] = [(17, 0), (0, 1), (18, 17), (1, 18)];

/// Stock `groundEffectDensity`: cells selected per chunk (16..=256, default 64).
pub const DEFAULT_DENSITY: u16 = 64;
/// Stock `groundEffectDist` (yards): detail doodads fade out over its last 15%.
pub const DEFAULT_DISTANCE: f32 = 140.0;

/// GroundEffectTexture: how densely a layer scatters which doodads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectTexture {
    pub density: u32,
    pub doodads: [u32; 4],
    pub weights: [u32; 4],
}

/// GroundEffectDoodad: a detail model and its flags (1 aligns to the terrain face, 2 keeps
/// the model's own colour instead of the terrain tint).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectDoodad {
    pub model_fdid: u32,
    pub flags: u32,
}

#[derive(Default)]
pub struct GroundEffects {
    textures: HashMap<u32, EffectTexture>,
    doodads: HashMap<u32, EffectDoodad>,
}

impl GroundEffects {
    pub fn new(textures: HashMap<u32, EffectTexture>, doodads: HashMap<u32, EffectDoodad>) -> Self {
        Self { textures, doodads }
    }

    /// `GroundEffectTexture.csv` and `GroundEffectDoodad.csv` (scripts/export_db2_csv.py).
    pub fn parse(texture_csv: &str, doodad_csv: &str) -> Result<Self, String> {
        let (columns, lines) = csv_rows(texture_csv)?;
        let mut textures = HashMap::new();
        for (number, line) in lines {
            let row = CsvRow::new(&columns, line, number + 2);
            let field = |name: &str, index: usize| row.parse::<u32>(&format!("{name}_{index}"));
            let mut doodads = [0; 4];
            let mut weights = [0; 4];
            for index in 0..4 {
                doodads[index] = field("DoodadID", index)?;
                weights[index] = field("DoodadWeight", index)?;
            }
            textures.insert(
                row.parse("ID")?,
                EffectTexture {
                    density: row.parse("Density")?,
                    doodads,
                    weights,
                },
            );
        }
        let (columns, lines) = csv_rows(doodad_csv)?;
        let mut doodads = HashMap::new();
        for (number, line) in lines {
            let row = CsvRow::new(&columns, line, number + 2);
            doodads.insert(
                row.parse("ID")?,
                EffectDoodad {
                    model_fdid: row.parse("ModelFileID")?,
                    flags: row.parse("Flags")?,
                },
            );
        }
        Ok(Self { textures, doodads })
    }

    pub fn texture(&self, id: u32) -> Option<&EffectTexture> {
        self.textures.get(&id)
    }

    pub fn doodad(&self, id: u32) -> Option<&EffectDoodad> {
        self.doodads.get(&id)
    }
}

/// The MCNK inputs of the scatter.
pub struct DetailChunk<'a> {
    /// MCVT heights relative to the chunk base, outer 9 and inner 8 per row.
    pub heights: &'a [f32; 145],
    /// MCCV bytes in file order (BGRA), when authored.
    pub vertex_colors_bgra: Option<[[u8; 4]; 145]>,
    /// MCSH, 64 rows of 64 bits, least significant bit first.
    pub shadow: Option<&'a [u8; 512]>,
    pub holes_low_res: u16,
    /// Per-cell holes of chunks flagged 0x10000; they replace `holes_low_res`.
    pub holes_high_res: Option<u64>,
    /// MCNK header 0x40: per cell row, 2-bit MCLY indices.
    pub texture_selection: [u16; 8],
    /// MCNK header 0x50: per cell row, detail-doodad disable bits.
    pub detail_exclusion: [u8; 8],
    /// MCLY effect ids, in layer order.
    pub layer_effects: &'a [u32],
}

impl<'a> DetailChunk<'a> {
    /// The scatter inputs of a parsed root chunk and its `_tex0` layers.
    pub fn from_adt(chunk: &'a adt::Chunk, layer_effects: &'a [u32]) -> Self {
        let vertex_colors_bgra = chunk.has_vertex_colors.then(|| {
            chunk.vertex_colors.map(|[red, green, blue, alpha]| {
                [
                    (blue * 127.0).round() as u8,
                    (green * 127.0).round() as u8,
                    (red * 127.0).round() as u8,
                    (alpha * 255.0).round() as u8,
                ]
            })
        });
        Self {
            heights: &chunk.heights,
            vertex_colors_bgra,
            shadow: chunk.shadow_map.as_ref(),
            holes_low_res: chunk.holes_low_res,
            holes_high_res: chunk.holes_high_res,
            texture_selection: chunk.texture_selection,
            detail_exclusion: chunk.detail_exclusion,
            layer_effects,
        }
    }

    /// The ground effect of cell `(x, y)`: holes and header 0x50 exclude it, header 0x40
    /// selects its layer (client 0x007A0530).
    /// The ground effect of cell `(x, y)`: holes and header 0x50 exclude it, header 0x40
    /// selects its layer (client 0x007A0530). Retail's per-cell high-resolution holes
    /// exclude their cell as the 2x2-cell low-resolution ones do.
    fn detail_effect_at(&self, x: u8, y: u8) -> Option<u32> {
        let hole = crate::asset::adt_format::adt_geometry::terrain_hole_at(
            self.holes_low_res,
            self.holes_high_res,
            usize::from(x),
            usize::from(y),
        );
        if hole || self.detail_exclusion[usize::from(y)] & (1 << x) != 0 {
            return None;
        }
        let layer = (self.texture_selection[usize::from(y)] >> (x * 2)) & 3;
        self.layer_effects.get(usize::from(layer)).copied()
    }

    fn shadowed(&self, column: usize, row: usize) -> bool {
        self.shadow.is_some_and(|shadow| {
            column < 64 && row < 64 && shadow[row * 8 + column / 8] & (1 << (column & 7)) != 0
        })
    }
}

/// The scatter seed of chunk `(chunk_x, chunk_y)` (MCNK index x/y) of the tile
/// `{map}_{tile_y}_{tile_x}.adt`: the chunk's global row in the high word, column in the low.
pub fn chunk_seed(tile: (u32, u32), chunk_x: u32, chunk_y: u32) -> u32 {
    let (tile_y, tile_x) = tile;
    ((tile_x * 16 + chunk_y) << 16) | (tile_y * 16 + chunk_x)
}

/// Engine-space position of the chunk's first vertex at the chunk base height: the frame
/// of placements and detail vertices (`adt::chunk_geometry` places vertex 0 there plus its
/// height). `tile` is `(tile_y, tile_x)` as there.
pub fn chunk_origin(chunk: &adt::Chunk, tile: (u32, u32)) -> [f32; 3] {
    let (x, z) = crate::asset::adt_format::adt::chunk_origin_from_parts(
        chunk.index_x,
        chunk.index_y,
        chunk.position,
        Some(tile),
    );
    [x, chunk.position[2], z]
}

/// WoW axes (x north, y west, z up) to engine axes (x, y up, z).
pub fn wow_to_engine([x, y, z]: [f32; 3]) -> [f32; 3] {
    [x, z, -y]
}

/// One scattered doodad, before mesh expansion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    /// GroundEffectDoodad id.
    pub doodad: u32,
    pub position: [f32; 3],
    pub angle: f32,
    pub scale: f32,
    /// The geometric normal of the terrain face it stands on.
    pub normal: [f32; 3],
    /// `(cell column + cell row * 8) * 4 + face`.
    pub face: u16,
    /// RGBA terrain tint; alpha 0 where MCSH shadows it.
    pub color: [u8; 4],
}

/// Client 7D3390: `density` cell selections, then each selected cell's effect density of
/// weighted doodads, admitted where the face normal is steep enough.
pub fn scatter(
    chunk: &DetailChunk<'_>,
    effects: &GroundEffects,
    seed: u32,
    density: u16,
) -> Result<Vec<Placement>, String> {
    let mut random = BlizzardRand::new(seed);
    let cells: Vec<[u8; 2]> = (0..density)
        .map(|_| [(random.next_u32() & 7) as u8, (random.next_u32() & 7) as u8])
        .collect();
    let mut placements = Vec::new();
    for (attempt, [column, row]) in cells.into_iter().enumerate() {
        let effect = chunk
            .detail_effect_at(column, row)
            .and_then(|id| effects.texture(id));
        if let Some(effect) = effect {
            let cell = CellScatter {
                chunk,
                effects,
                column,
                row,
                attempt,
                models: distribution(effect),
                planes: std::array::from_fn(|face| plane(chunk.heights, column, row, face)),
            };
            // 7D3390 places eight doodads per cell for an effect density of zero.
            let per_cell = if effect.density == 0 {
                8
            } else {
                effect.density
            };
            for instance in 0..per_cell {
                placements.extend(cell.place(instance as usize, &mut random)?);
            }
        }
    }
    Ok(placements)
}

/// One selected cell's doodads.
struct CellScatter<'a> {
    chunk: &'a DetailChunk<'a>,
    effects: &'a GroundEffects,
    column: u8,
    row: u8,
    /// Index of this cell selection; offsets the weighted slot of each instance.
    attempt: usize,
    models: [u32; 16],
    planes: [(Vec3, f32); 4],
}

impl CellScatter<'_> {
    /// Instance `instance`: two draws place it in the cell; an admitted doodad draws its
    /// angle and scale.
    fn place(
        &self,
        instance: usize,
        random: &mut BlizzardRand,
    ) -> Result<Option<Placement>, String> {
        let signed = [next_signed(random), next_signed(random)];
        let offsets = signed.map(|value| value * HALF_UNIT + HALF_UNIT);
        let mut position = [-offsets[1], -offsets[0], 0.0];
        let face = usize::from(position[1] - position[0] < 0.0)
            + 2 * usize::from((-position[1] - UNIT) - position[0] > 0.0);
        let doodad = self.models[(instance + self.attempt) & 15];
        let (normal, distance) = self.planes[face];
        if doodad == 0 || normal.z < 0.4 {
            return Ok(None);
        }
        let definition = self
            .effects
            .doodad(doodad)
            .ok_or_else(|| format!("GroundEffectDoodad {doodad} has no row"))?;
        let (column, row) = (f32::from(self.column), f32::from(self.row));
        let mut color = if definition.flags & 2 == 0 {
            tint(self.chunk, self.column, self.row, face, signed, position)
        } else {
            [255; 4]
        };
        let sample = [offsets[0] + column * UNIT, offsets[1] + row * UNIT];
        if self.chunk.shadowed(
            (sample[0] * 1.92).floor() as usize,
            (sample[1] * 1.92).floor() as usize,
        ) {
            color[3] = 0;
        }
        position[0] -= row * UNIT;
        position[1] -= column * UNIT;
        position[2] = -((position[1] * normal.y + normal.x * position[0] + distance) / normal.z);
        Ok(Some(Placement {
            doodad,
            position,
            angle: (next_signed(random) + 1.0) * std::f32::consts::PI,
            scale: next_signed(random) * 0.33 + 1.0,
            normal: normal.to_array(),
            face: ((usize::from(self.column) + usize::from(self.row) * 8) * 4 + face) as u16,
            color,
        }))
    }
}

/// Sixteen weighted doodad slots, written with the client's thirteen-step permutation and
/// padded with the four doodads in order.
fn distribution(effect: &EffectTexture) -> [u32; 16] {
    let mut result = [0; 16];
    let mut slot = 0;
    let mut count = 0;
    for (doodad, weight) in effect.doodads.into_iter().zip(effect.weights) {
        for _ in 0..weight {
            result[slot & 15] = doodad;
            slot += 13;
            count += 1;
        }
    }
    for index in count..16 {
        result[slot & 15] = effect.doodads[index & 3];
        slot += 13;
    }
    result
}

/// A value in (-1, 1) built from one random word's mantissa and sign.
fn next_signed(random: &mut BlizzardRand) -> f32 {
    let word = random.next_u32();
    let magnitude = f32::from_bits(word & 0x7f_ffff | 0x3f80_0000);
    if word & 0x8000_0000 != 0 {
        2.0 - magnitude
    } else {
        magnitude - 2.0
    }
}

/// The plane through a cell's centre vertex and the two outer corners of `face`.
fn plane(heights: &[f32; 145], column: u8, row: u8, face: usize) -> (Vec3, f32) {
    let base = usize::from(row) * 17 + usize::from(column);
    let origin = Vec3::new(-f32::from(row) * UNIT, -f32::from(column) * UNIT, 0.0);
    let center = origin + Vec3::new(-HALF_UNIT, -HALF_UNIT, heights[base + 9]);
    let (first, second) = CORNERS[face];
    let vertex = |index: usize| {
        origin
            + Vec3::new(
                -((index / 17) as f32) * UNIT,
                -((index % 17) as f32) * UNIT,
                heights[base + index],
            )
    };
    let normal = (vertex(second) - center)
        .cross(vertex(first) - center)
        .normalize();
    (
        normal,
        -(center.y * normal.y + center.x * normal.x + center.z * normal.z),
    )
}

/// The MCCV tint interpolated over the face, in the doubled byte range (127 is white).
fn tint(
    chunk: &DetailChunk<'_>,
    column: u8,
    row: u8,
    face: usize,
    signed: [f32; 2],
    position: [f32; 3],
) -> [u8; 4] {
    let Some(colors) = chunk.vertex_colors_bgra.as_ref() else {
        return [255; 4];
    };
    let base = usize::from(row) * 17 + usize::from(column);
    let (first, second) = CORNERS[face];
    let (major, minor) = if signed[1].abs() < signed[0].abs() {
        (signed[0].abs(), signed[1])
    } else {
        (signed[1].abs(), signed[0])
    };
    let mut factor = 0.5 - minor * 0.5;
    if position[1] - position[0] < 0.0 {
        factor = 1.0 - factor;
    }
    factor *= major;
    let mut result = [255; 4];
    for channel in 0..3 {
        let center = f32::from(colors[base + 9][channel]);
        let first = f32::from(colors[base + first][channel]);
        let second = f32::from(colors[base + second][channel]);
        let value = center * 2.0 + (first - center) * 2.0 * major + (second - first) * 2.0 * factor;
        result[2 - channel] = (value.min(255.0) - 0.5).round_ties_even() as u8;
    }
    result
}

/// A detail model: the first texture and the first skin's vertices and triangles.
#[derive(Clone, Debug, PartialEq)]
pub struct DetailModel {
    pub texture_fdid: u32,
    /// Position and first UV, in skin vertex order.
    pub vertices: Vec<([f32; 3], [f32; 2])>,
    pub indices: Vec<u16>,
}

impl DetailModel {
    /// The vertices the skin's triangles use, in first-use order.
    pub fn from_model(model: &m2::Model) -> Result<Self, String> {
        let texture_fdid = model
            .texture_fdids
            .first()
            .copied()
            .filter(|&fdid| fdid != 0)
            .ok_or("Detail model has no first texture FDID")?;
        let mut remap = HashMap::new();
        let mut vertices = Vec::new();
        let mut indices = Vec::with_capacity(model.indices.len());
        for &global in &model.indices {
            let local = match remap.get(&global) {
                Some(&local) => local,
                None => {
                    let vertex = model
                        .vertices
                        .get(usize::from(global))
                        .ok_or_else(|| format!("Detail model vertex {global} out of bounds"))?;
                    let local = u16::try_from(vertices.len())
                        .map_err(|_| "Detail model exceeds 16-bit vertices")?;
                    vertices.push((vertex.position, vertex.tex_coords));
                    remap.insert(global, local);
                    local
                }
            };
            indices.push(local);
        }
        if indices.is_empty() {
            return Err("Detail model has no triangles".into());
        }
        Ok(Self {
            texture_fdid,
            vertices,
            indices,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DetailVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [u8; 4],
    pub uv: [f32; 2],
}

/// A texture bucket: one texture and its range of the chunk's indices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DetailBatch {
    pub texture_fdid: u32,
    pub first_index: usize,
    pub index_count: usize,
}

#[derive(Default)]
pub struct DetailMesh {
    pub vertices: Vec<DetailVertex>,
    pub indices: Vec<u16>,
    pub batches: Vec<DetailBatch>,
}

/// Client 7B31E0/7B1B50: placements grouped into at most four texture buckets of at most
/// `min(density * 64, 4096)` vertices and indices each, expanded into one vertex bank.
pub fn build_mesh<'a>(
    placements: &[Placement],
    effects: &GroundEffects,
    density: u16,
    model_of: impl FnMut(u32) -> Option<&'a DetailModel>,
) -> Result<DetailMesh, String> {
    let capacity = (usize::from(density) * 64).min(4096);
    let mut mesh = DetailMesh::default();
    for bucket in fill_buckets(placements, capacity, model_of)? {
        let first_index = mesh.indices.len();
        let mut transforms = FaceTransforms::default();
        for (placement, model) in bucket.placements {
            let flags = effects
                .doodad(placement.doodad)
                .ok_or_else(|| format!("GroundEffectDoodad {} has no row", placement.doodad))?
                .flags;
            mesh.expand(&placement, model, transforms.select(&placement, flags))?;
        }
        mesh.batches.push(DetailBatch {
            texture_fdid: bucket.texture_fdid,
            first_index,
            index_count: mesh.indices.len() - first_index,
        });
    }
    Ok(mesh)
}

impl DetailMesh {
    /// Appends `model` transformed onto `placement`.
    fn expand(
        &mut self,
        placement: &Placement,
        model: &DetailModel,
        transform: Mat3,
    ) -> Result<(), String> {
        if self.vertices.len() + model.vertices.len() > usize::from(u16::MAX) {
            return Err("Detail chunk exceeds its 16-bit index bank".into());
        }
        let first_vertex = self.vertices.len() as u16;
        let offset = Vec3::from_array(placement.position);
        self.vertices
            .extend(model.vertices.iter().map(|&(position, uv)| DetailVertex {
                position:
                    (transform * Vec3::from_array(position) * placement.scale + offset).to_array(),
                normal: placement.normal,
                color: placement.color,
                uv,
            }));
        self.indices
            .extend(model.indices.iter().map(|index| first_vertex + index));
        Ok(())
    }
}

/// Placements in order into the first bucket of their texture with room for the model;
/// a fifth texture bucket is dropped.
fn fill_buckets<'a>(
    placements: &[Placement],
    capacity: usize,
    mut model_of: impl FnMut(u32) -> Option<&'a DetailModel>,
) -> Result<Vec<Bucket<'a>>, String> {
    let mut buckets: Vec<Bucket<'a>> = Vec::with_capacity(4);
    for placement in placements {
        let model = model_of(placement.doodad)
            .ok_or_else(|| format!("GroundEffectDoodad {} has no model", placement.doodad))?;
        let matching = buckets.iter().position(|bucket| {
            bucket.texture_fdid == model.texture_fdid
                && bucket.vertex_count + model.vertices.len() < capacity
                && bucket.index_count + model.indices.len() < capacity
        });
        let index = match matching {
            Some(index) => index,
            None if buckets.len() == 4 => continue,
            None => {
                buckets.push(Bucket {
                    texture_fdid: model.texture_fdid,
                    placements: Vec::new(),
                    vertex_count: 0,
                    index_count: 0,
                });
                buckets.len() - 1
            }
        };
        let bucket = &mut buckets[index];
        bucket.vertex_count += model.vertices.len();
        bucket.index_count += model.indices.len();
        bucket.placements.push((*placement, model));
    }
    Ok(buckets)
}

struct Bucket<'a> {
    texture_fdid: u32,
    placements: Vec<(Placement, &'a DetailModel)>,
    vertex_count: usize,
    index_count: usize,
}

/// Face-aligned doodads (flag 1) of one cell reuse the first rotation of each face.
#[derive(Default)]
struct FaceTransforms {
    cell: Option<u16>,
    faces: [Option<Mat3>; 4],
}

impl FaceTransforms {
    fn select(&mut self, placement: &Placement, flags: u32) -> Mat3 {
        let rotation = Mat3::from_rotation_z(placement.angle);
        if flags & 1 == 0 {
            return rotation;
        }
        let cell = placement.face & 0xfc;
        if self.cell != Some(cell) {
            self.cell = Some(cell);
            self.faces = [None; 4];
        }
        *self.faces[usize::from(placement.face & 3)].get_or_insert_with(|| {
            let normal = Vec3::from_array(placement.normal);
            let tangent = Vec3::new(0.0, normal.z, -normal.y).normalize();
            Mat3::from_cols(normal.cross(tangent), tangent, normal) * rotation
        })
    }
}

/// Blizzard's table generator (client 0x004c1510 seed, 0x00464580 step), as
/// solarityclient `crates/cpu/src/random/blizzard_rand.rs`.
pub struct BlizzardRand {
    accumulator: u32,
    indices: [usize; 4],
}

impl BlizzardRand {
    pub fn new(seed: u32) -> Self {
        Self {
            accumulator: seed,
            indices: [
                (seed % 61) as usize,
                (seed % 59) as usize,
                (seed % 53) as usize,
                (seed % 47) as usize,
            ],
        }
    }

    pub fn next_u32(&mut self) -> u32 {
        let rewind = |index: usize, stride: usize, modulus: usize| {
            if index < stride {
                index + modulus - stride
            } else {
                index - stride
            }
        };
        self.indices[0] = rewind(self.indices[0], 7, 61);
        self.indices[1] = rewind(self.indices[1], 6, 59);
        self.indices[2] = rewind(self.indices[2], 3, 53);
        self.indices[3] = rewind(self.indices[3], 1, 47);
        let mixed = RANDOM_TABLE[self.indices[3]].rotate_left(1)
            ^ RANDOM_TABLE[self.indices[2]].rotate_left(2)
            ^ RANDOM_TABLE[self.indices[1]].rotate_left(3)
            ^ RANDOM_TABLE[self.indices[0]];
        self.accumulator = self.accumulator.wrapping_add(mixed);
        self.accumulator
    }
}

/// The generator's constant table (client address 0x009f1700).
const RANDOM_TABLE: [u32; 61] = [
    0x9927_148e,
    0x08c7_aafd,
    0x1f3e_e6d5,
    0xda55_bbf6,
    0x6a4a_a075,
    0xff97_bde8,
    0x9fbc_9bde,
    0x46a1_8a81,
    0x63e3_0b6e,
    0x5d6c_7a76,
    0xca69_d388,
    0x25b9_47c3,
    0x3fa2_ab83,
    0xba7c_41a6,
    0x0195_ace5,
    0xc109_cf7e,
    0x7170_62d9,
    0x0205_db8d,
    0x54ef_8724,
    0x3037_d4c6,
    0x7bcb_1bd0,
    0xecd8_e4b8,
    0xdcad_ce49,
    0xc494_a913,
    0x0dae_398f,
    0x0edd_5218,
    0x85f5_fa78,
    0x6daf_d258,
    0x3b53_b2a4,
    0xbe50_a551,
    0x11f4_2dfc,
    0xf116_9848,
    0x663d_df86,
    0x2f2e_445e,
    0x176b_0736,
    0xb64c_298b,
    0xe75f_89e2,
    0xe121_a7cd,
    0xed65_c94d,
    0x239c_eefe,
    0x04b7_7d33,
    0x402a_9a9e,
    0xf35b_10b3,
    0x921c_7782,
    0x571e_4e20,
    0x8c06_7222,
    0xfb73_2c67,
    0xbf0a_c259,
    0x0cf9_5c79,
    0x6812_1a28,
    0x4219_3474,
    0xf884_c0b1,
    0x9d15_f038,
    0x6f3a_f260,
    0x91eb_90b4,
    0x6135_7f1d,
    0x5603_325a,
    0x932b_c5a3,
    0x434b_0f80,
    0x3ce0_a8f7,
    0x2664_d196,
];
