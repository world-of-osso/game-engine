//! One ADT tile as one mesh and one `terrain.gdshader` material: every MCNK chunk's
//! geometry is concatenated with its chunk index in UV2, its material inputs become texels
//! of a per-tile data texture, its alpha map a cell of a per-tile atlas, and its layer
//! textures layers of shared per-size texture arrays. Distinct per-chunk materials were
//! one draw each (2,304 at the Trade District pose for 9 tiles).

use std::collections::HashMap;

use game_engine_core::{adt::Geometry, blp};
use godot::{
    classes::{
        ArrayMesh, Image, ImageTexture, Shader, ShaderMaterial, Texture2DArray, image, mesh,
    },
    prelude::*,
};

/// Chunk texture reference of Godot's default white texture (an unbound sampler).
pub(crate) const REF_WHITE: f32 = -1.0;
/// Chunk texture reference of the original renderer's gray texel for layerless chunks.
pub(crate) const REF_UNTEXTURED: f32 = -2.0;
/// `terrain.gdshader` LAYER_CLASS_STRIDE: a reference is `class * stride + layer`.
const CLASS_STRIDE: u32 = 4096;
/// `terrain.gdshader` binds `layers_0..3`.
const MAX_SIZE_CLASSES: usize = 4;
const INITIAL_LAYERS: usize = 16;
pub(crate) const CHUNKS_PER_SIDE: usize = 16;
const DATA_ROWS: usize = 11;
const ALPHA_SIZE: usize = 64;
const ATLAS_SIZE: usize = CHUNKS_PER_SIDE * ALPHA_SIZE;

/// Every layer texture of the built tiles, once: one `Texture2DArray` per image size.
#[derive(Default)]
pub(crate) struct LayerArrays {
    classes: Vec<SizeClass>,
    references: HashMap<u32, f32>,
    /// A class array was replaced by a larger one since `take_replaced`.
    replaced: bool,
}

struct SizeClass {
    size: (u32, u32),
    array: Gd<Texture2DArray>,
    /// Source images by layer, to fill a replacement array.
    images: Vec<Gd<Image>>,
    capacity: usize,
}

impl LayerArrays {
    /// The chunk texture reference of `fdid`, adding its image on first use.
    pub fn reference(&mut self, fdid: u32, pixels: &blp::RgbaImage) -> Result<f32, String> {
        if let Some(&reference) = self.references.get(&fdid) {
            return Ok(reference);
        }
        let image = rgba_image(pixels.width, pixels.height, &pixels.pixels)
            .map_err(|error| format!("Texture FDID {fdid}: {error}"))?;
        let size = (pixels.width, pixels.height);
        let class = match self.classes.iter().position(|class| class.size == size) {
            Some(class) => class,
            None if self.classes.len() < MAX_SIZE_CLASSES => {
                self.classes.push(SizeClass {
                    size,
                    array: Texture2DArray::new_gd(),
                    images: Vec::new(),
                    capacity: 0,
                });
                self.classes.len() - 1
            }
            None => {
                return Err(format!(
                    "Texture FDID {fdid} is a {}x{} terrain layer; the terrain shader binds \
                     {MAX_SIZE_CLASSES} sizes: {:?}",
                    size.0,
                    size.1,
                    self.classes
                        .iter()
                        .map(|class| class.size)
                        .collect::<Vec<_>>()
                ));
            }
        };
        let (layer, replaced) = self.classes[class].push(image)?;
        self.replaced |= replaced;
        let reference = (class as u32 * CLASS_STRIDE + layer as u32) as f32;
        self.references.insert(fdid, reference);
        Ok(reference)
    }

    pub fn bind(&self, material: &mut Gd<ShaderMaterial>) {
        for (slot, class) in self.classes.iter().enumerate() {
            material.set_shader_parameter(&format!("layers_{slot}"), &class.array.to_variant());
        }
    }

    /// Whether a class array was replaced since the last call: built materials still bind
    /// the old one.
    pub fn take_replaced(&mut self) -> bool {
        std::mem::take(&mut self.replaced)
    }
}

impl SizeClass {
    /// Adds `image` as the next layer; `true` when the array had to be replaced to grow.
    fn push(&mut self, image: Gd<Image>) -> Result<(usize, bool), String> {
        let layer = self.images.len();
        self.images.push(image.clone());
        if layer < self.capacity {
            self.array.update_layer(&image, layer as i32);
            return Ok((layer, false));
        }
        let replaced = self.capacity > 0;
        self.capacity = (self.capacity * 2).max(INITIAL_LAYERS);
        // Spare layers repeat the first image until a texture claims them.
        let mut images = Array::<Gd<Image>>::new();
        for index in 0..self.capacity {
            images.push(self.images.get(index).unwrap_or(&self.images[0]));
        }
        let mut array = Texture2DArray::new_gd();
        let error = array.create_from_images(&images);
        if error != godot::global::Error::OK {
            return Err(format!(
                "Godot rejected the {}x{} terrain layer array: {error:?}",
                self.size.0, self.size.1
            ));
        }
        self.array = array;
        Ok((layer, replaced))
    }
}

/// One chunk's `terrain.gdshader` inputs: chunk_data rows and its alpha map.
pub(crate) struct ChunkInputs {
    pub config: [f32; 4],
    pub layer_params: [[f32; 4]; 4],
    pub animation: [[f32; 4]; 4],
    pub ground: [f32; 4],
    pub height: [f32; 4],
    /// 64x64 RGBA, linear.
    pub alpha: Vec<u8>,
}

/// A tile's chunks gathered over frames, before any Godot resource exists.
pub(crate) struct TileAssembly {
    positions: Vec<Vector3>,
    normals: Vec<Vector3>,
    colors: Vec<Color>,
    uvs: Vec<Vector2>,
    chunk_ids: Vec<Vector2>,
    indices: Vec<i32>,
    data: Vec<f32>,
    alpha: Vec<u8>,
}

impl Default for TileAssembly {
    fn default() -> Self {
        Self {
            positions: Vec::new(),
            normals: Vec::new(),
            colors: Vec::new(),
            uvs: Vec::new(),
            chunk_ids: Vec::new(),
            indices: Vec::new(),
            data: vec![0.0; CHUNKS_PER_SIDE * CHUNKS_PER_SIDE * DATA_ROWS * 4],
            alpha: vec![0; ATLAS_SIZE * ATLAS_SIZE * 4],
        }
    }
}

impl TileAssembly {
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// Appends chunk `(x, y)`'s geometry and material inputs.
    pub fn add_chunk(
        &mut self,
        (x, y): (u32, u32),
        geometry: &Geometry,
        colors: &[[f32; 4]; 145],
        inputs: &ChunkInputs,
    ) -> Result<(), String> {
        let (x, y) = (x as usize, y as usize);
        if x >= CHUNKS_PER_SIDE || y >= CHUNKS_PER_SIDE {
            return Err(format!("Chunk index ({x}, {y}) is outside the 16x16 tile"));
        }
        if inputs.alpha.len() != ALPHA_SIZE * ALPHA_SIZE * 4 {
            return Err(format!(
                "Chunk ({x}, {y}) alpha has {} bytes",
                inputs.alpha.len()
            ));
        }
        let base = self.positions.len() as i32;
        self.positions
            .extend(geometry.positions.iter().map(|p| Vector3::from_array(*p)));
        self.normals
            .extend(geometry.normals.iter().map(|n| Vector3::from_array(*n)));
        self.uvs
            .extend(geometry.uvs.iter().map(|uv| Vector2::from_array(*uv)));
        self.colors
            .extend(colors.iter().map(|color| mccv_color(*color)));
        self.chunk_ids.extend(std::iter::repeat_n(
            Vector2::new(x as f32, y as f32),
            geometry.positions.len(),
        ));
        // Godot ArrayMesh front faces are clockwise; the authored indices are counterclockwise.
        self.indices.extend(
            geometry
                .indices
                .chunks_exact(3)
                .flat_map(|triangle| [triangle[0], triangle[2], triangle[1]])
                .map(|index| base + index as i32),
        );
        let column = y * CHUNKS_PER_SIDE + x;
        let rows = [
            inputs.config,
            inputs.layer_params[0],
            inputs.layer_params[1],
            inputs.layer_params[2],
            inputs.layer_params[3],
            inputs.animation[0],
            inputs.animation[1],
            inputs.animation[2],
            inputs.animation[3],
            inputs.ground,
            inputs.height,
        ];
        let width = CHUNKS_PER_SIDE * CHUNKS_PER_SIDE;
        for (row, values) in rows.iter().enumerate() {
            let at = (row * width + column) * 4;
            self.data[at..at + 4].copy_from_slice(values);
        }
        for line in 0..ALPHA_SIZE {
            let from = line * ALPHA_SIZE * 4;
            let to = ((y * ALPHA_SIZE + line) * ATLAS_SIZE + x * ALPHA_SIZE) * 4;
            self.alpha[to..to + ALPHA_SIZE * 4]
                .copy_from_slice(&inputs.alpha[from..from + ALPHA_SIZE * 4]);
        }
        Ok(())
    }

    /// The tile mesh and its material; `hidden` is the material's camera-rejection mask.
    pub fn build(
        self,
        shader: &Gd<Shader>,
        arrays: &LayerArrays,
        hidden: &Gd<ImageTexture>,
    ) -> Result<(Gd<ArrayMesh>, Gd<ShaderMaterial>), String> {
        let _span = crate::profile::span(|| "terrain.tile_build".to_owned());
        let mut material = crate::rebuild_guard::new_shader_material();
        material.set_shader(shader);
        arrays.bind(&mut material);
        let width = CHUNKS_PER_SIDE * CHUNKS_PER_SIDE;
        let data_bytes: Vec<u8> = self
            .data
            .iter()
            .flat_map(|value| value.to_ne_bytes())
            .collect();
        let data = image_texture(
            width as u32,
            DATA_ROWS as u32,
            image::Format::RGBAF,
            &data_bytes,
        )?;
        material.set_shader_parameter("chunk_data", &data.to_variant());
        let alpha = image_texture(
            ATLAS_SIZE as u32,
            ATLAS_SIZE as u32,
            image::Format::RGBA8,
            &self.alpha,
        )?;
        material.set_shader_parameter("alpha_atlas", &alpha.to_variant());
        material.set_shader_parameter("chunk_hidden", &hidden.to_variant());
        Ok((self.into_mesh(), material))
    }

    fn into_mesh(self) -> Gd<ArrayMesh> {
        let _mesh = crate::profile::span(|| "phase.mesh_build.terrain".to_owned());
        let mut arrays = VarArray::new();
        arrays.resize(mesh::ArrayType::MAX.ord() as usize, &Variant::nil());
        let set = |arrays: &mut VarArray, kind: mesh::ArrayType, value: Variant| {
            arrays.set(kind.ord() as usize, &value);
        };
        set(
            &mut arrays,
            mesh::ArrayType::VERTEX,
            PackedVector3Array::from(self.positions.as_slice()).to_variant(),
        );
        set(
            &mut arrays,
            mesh::ArrayType::NORMAL,
            PackedVector3Array::from(self.normals.as_slice()).to_variant(),
        );
        set(
            &mut arrays,
            mesh::ArrayType::COLOR,
            PackedColorArray::from(self.colors.as_slice()).to_variant(),
        );
        set(
            &mut arrays,
            mesh::ArrayType::TEX_UV,
            PackedVector2Array::from(self.uvs.as_slice()).to_variant(),
        );
        set(
            &mut arrays,
            mesh::ArrayType::TEX_UV2,
            PackedVector2Array::from(self.chunk_ids.as_slice()).to_variant(),
        );
        set(
            &mut arrays,
            mesh::ArrayType::INDEX,
            PackedInt32Array::from(self.indices.as_slice()).to_variant(),
        );
        let mut mesh = crate::rebuild_guard::new_array_mesh();
        mesh.add_surface_from_arrays(mesh::PrimitiveType::TRIANGLES, &arrays);
        mesh
    }
}

/// The camera-rejection mask of a tile: one R8 texel per chunk, 255 = rejected.
pub(crate) fn hidden_mask(
    hidden: &[u8; CHUNKS_PER_SIDE * CHUNKS_PER_SIDE],
) -> Result<Gd<Image>, String> {
    Image::create_from_data(
        CHUNKS_PER_SIDE as i32,
        CHUNKS_PER_SIDE as i32,
        false,
        image::Format::R8,
        &PackedByteArray::from(hidden.as_slice()),
    )
    .ok_or_else(|| "Godot rejected the terrain chunk mask".to_owned())
}

/// ArrayMesh packs COLOR into byte/255; the shader restores authored byte/127.
fn mccv_color(color: [f32; 4]) -> Color {
    const MCCV_TO_VERTEX_COLOR: f32 = 127.0 / 255.0;
    Color::from_rgba(
        color[0] * MCCV_TO_VERTEX_COLOR,
        color[1] * MCCV_TO_VERTEX_COLOR,
        color[2] * MCCV_TO_VERTEX_COLOR,
        color[3],
    )
}

fn rgba_image(width: u32, height: u32, pixels: &[u8]) -> Result<Gd<Image>, String> {
    Image::create_from_data(
        width as i32,
        height as i32,
        false,
        image::Format::RGBA8,
        &PackedByteArray::from(pixels),
    )
    .ok_or_else(|| format!("Godot rejected {width}x{height} terrain image"))
}

fn image_texture(
    width: u32,
    height: u32,
    format: image::Format,
    bytes: &[u8],
) -> Result<Gd<ImageTexture>, String> {
    let _upload = crate::profile::span(|| "phase.texture_upload.terrain".to_owned());
    let image = Image::create_from_data(
        width as i32,
        height as i32,
        false,
        format,
        &PackedByteArray::from(bytes),
    )
    .ok_or_else(|| format!("Godot rejected {width}x{height} terrain data image"))?;
    ImageTexture::create_from_image(&image).ok_or_else(|| "Godot rejected terrain texture".into())
}
