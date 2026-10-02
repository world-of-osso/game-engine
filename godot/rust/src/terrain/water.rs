//! Authored MH2O surfaces with the retail liquid material of their LiquidType.
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use game_engine_core::{
    adt, blp,
    liquid_data::{LiquidShader, TEXTURE_SLOTS, WaterColorSource},
};
use godot::{
    classes::{
        ArrayMesh, Image, ImageTexture, MeshInstance3D, Node3D, ResourceLoader, Shader,
        ShaderMaterial, image, mesh,
    },
    prelude::*,
};

use super::assets::{LiquidFrame, NativeLiquidMaterial};
use crate::lighting::TerrainLight;

type LiquidKey = (u16, u16);
type Materials = BTreeMap<LiquidKey, Result<Arc<NativeLiquidMaterial>, String>>;

#[derive(Default)]
pub(super) struct WaterMaterials {
    shaders: HashMap<LiquidShader, Gd<Shader>>,
    materials: HashMap<LiquidKey, LiquidSurface>,
    textures: HashMap<u32, Gd<ImageTexture>>,
    black: Option<Gd<ImageTexture>>,
    light: Option<TerrainLight>,
}

/// One LiquidType's material and the frames of its texture slots.
struct LiquidSurface {
    material: Gd<ShaderMaterial>,
    slots: [Vec<Gd<ImageTexture>>; TEXTURE_SLOTS],
    /// Crossfaded slot and its period in milliseconds (`Float[index] * 1000`).
    crossfade: Option<(usize, f64)>,
}

impl WaterMaterials {
    pub fn material_count(&self) -> usize {
        self.materials.len()
    }

    /// A layer whose liquid material is unavailable is reported and left out, as an
    /// unbuildable tile is.
    pub fn build(
        &mut self,
        root: &adt::Root,
        materials: &Materials,
    ) -> Result<Option<Gd<Node3D>>, String> {
        let Some(water) = &root.water else {
            return Ok(None);
        };
        let mut meshes = Vec::new();
        for (index, chunk) in water.chunks.iter().enumerate() {
            let position = root
                .chunk_positions
                .get(index)
                .ok_or("MH2O chunk has no authored position")?;
            for layer in &chunk.layers {
                if let Some(surface) = self.layer_mesh(index, *position, layer, materials)? {
                    meshes.push(surface);
                }
            }
        }
        if meshes.is_empty() {
            return Ok(None);
        }
        let mut node = Node3D::new_alloc();
        node.set_name("Water");
        for (mesh, material) in meshes {
            node.add_child(&water_instance(&mesh, &material));
        }
        Ok(Some(node))
    }

    fn layer_mesh(
        &mut self,
        chunk: usize,
        position: [f32; 3],
        layer: &adt::WaterLayer,
        materials: &Materials,
    ) -> Result<Option<(Gd<ArrayMesh>, Gd<ShaderMaterial>)>, String> {
        let key = (layer.liquid_type, layer.liquid_object);
        let native = match materials.get(&key) {
            Some(Ok(native)) => native,
            Some(Err(error)) => {
                godot_error!("MH2O chunk {chunk} liquid layer {key:?}: {error}");
                return Ok(None);
            }
            None => return Err(format!("MH2O layer {key:?} has no resolved material")),
        };
        let geometry = adt::build_water_geometry(position, layer);
        if geometry.indices.is_empty() {
            return Ok(None);
        }
        let material = self.material(key, native)?;
        Ok(Some((build_mesh(geometry), material)))
    }

    pub fn sample_clock(&mut self, parent: &Gd<Node3D>) -> Result<(), String> {
        if self.materials.is_empty() {
            return Ok(());
        }
        let mut clock = parent
            .get_node_or_null("/root/M2MaterialClock")
            .ok_or("ADT liquids require the shared material clock")?;
        let milliseconds = clock
            .call("elapsed_time_ms", &[])
            .try_to::<f64>()
            .map_err(|error| format!("Cannot read liquid material time: {error}"))?;
        for surface in self.materials.values_mut() {
            surface.set_time(milliseconds);
        }
        Ok(())
    }

    pub fn update_lighting(&mut self, light: &TerrainLight) {
        for surface in self.materials.values_mut() {
            light.bind_water(&mut surface.material);
        }
        self.light = Some(light.clone());
    }

    /// One lit material outside any tile, for the liquid material fixture. The fixture drives
    /// `animation_time_ms`; texture frames stay at time 0.
    pub fn standalone(
        mut self,
        native: &NativeLiquidMaterial,
        light: &TerrainLight,
    ) -> Result<Gd<ShaderMaterial>, String> {
        self.light = Some(light.clone());
        Ok(self.create_surface(native)?.material)
    }

    /// A WMO group's liquid surface (`game_engine_core::wmo_liquid`), in a set of materials
    /// of WMO liquids only, keyed by (LiquidType, interior): the mesh carries its texture
    /// coordinates, and an interior group's procedural WMO water is white
    /// (`liquidWaterMat.slang:162-176`).
    pub fn wmo_surface(
        &mut self,
        liquid_type: u16,
        interior: bool,
        native: &NativeLiquidMaterial,
        geometry: adt::WaterGeometry,
    ) -> Result<Gd<MeshInstance3D>, String> {
        let mut material = self.material((liquid_type, u16::from(interior)), native)?;
        material.set_shader_parameter("mesh_uv", &true.to_variant());
        material.set_shader_parameter("interior", &interior.to_variant());
        Ok(water_instance(&build_mesh(geometry), &material))
    }

    fn material(
        &mut self,
        key: LiquidKey,
        native: &NativeLiquidMaterial,
    ) -> Result<Gd<ShaderMaterial>, String> {
        if let Some(surface) = self.materials.get(&key) {
            return Ok(surface.material.clone());
        }
        let surface = self.create_surface(native)?;
        let material = surface.material.clone();
        self.materials.insert(key, surface);
        Ok(material)
    }

    fn create_surface(&mut self, native: &NativeLiquidMaterial) -> Result<LiquidSurface, String> {
        let shader = self.shader(native.params.shader)?;
        let span = crate::profile::span(|| "water.set_shader".to_owned());
        let mut material = ShaderMaterial::new_gd();
        material.set_shader(&shader);
        drop(span);
        let _span = crate::profile::span(|| "water.textures".to_owned());
        bind_liquid_material(&mut material, native);
        for (name, image) in &native.globals {
            let texture = mipmapped_texture(image.width, image.height, &image.pixels)
                .map_err(|error| format!("Liquid texture {name}: {error}"))?;
            material.set_shader_parameter(*name, &texture.to_variant());
        }
        if let Some(light) = &self.light {
            light.bind_water(&mut material);
        }
        let mut slots: [Vec<Gd<ImageTexture>>; TEXTURE_SLOTS] = Default::default();
        for &slot in native.params.shader.texture_slots() {
            slots[slot] = self.frames(&native.slots[slot])?;
        }
        let crossfade = native
            .params
            .shader
            .crossfade()
            .map(|(slot, float)| (slot, f64::from(native.params.floats[float]) * 1000.0));
        let mut surface = LiquidSurface {
            material,
            slots,
            crossfade,
        };
        surface.set_time(0.0);
        Ok(surface)
    }

    /// Frames of one texture slot; an empty slot is the reference's one black pixel.
    fn frames(&mut self, frames: &[LiquidFrame]) -> Result<Vec<Gd<ImageTexture>>, String> {
        if frames.is_empty() {
            return Ok(vec![self.black()?]);
        }
        frames
            .iter()
            .map(|(fdid, image)| match image {
                Some(image) => self.texture(*fdid, image),
                None => self.black(),
            })
            .collect()
    }

    fn texture(&mut self, fdid: u32, image: &blp::RgbaImage) -> Result<Gd<ImageTexture>, String> {
        if let Some(texture) = self.textures.get(&fdid) {
            return Ok(texture.clone());
        }
        let texture = mipmapped_texture(image.width, image.height, &image.pixels)
            .map_err(|error| format!("Liquid texture FDID {fdid}: {error}"))?;
        self.textures.insert(fdid, texture.clone());
        Ok(texture)
    }

    fn black(&mut self) -> Result<Gd<ImageTexture>, String> {
        if let Some(black) = &self.black {
            return Ok(black.clone());
        }
        let black = mipmapped_texture(1, 1, &[0, 0, 0, 255])?;
        self.black = Some(black.clone());
        Ok(black)
    }

    fn shader(&mut self, kind: LiquidShader) -> Result<Gd<Shader>, String> {
        if let Some(shader) = self.shaders.get(&kind) {
            return Ok(shader.clone());
        }
        let path = shader_path(kind);
        let shader = ResourceLoader::singleton()
            .load(path)
            .ok_or_else(|| format!("Cannot load liquid shader {path}"))?
            .try_cast::<Shader>()
            .map_err(|_| format!("Liquid shader {path} has wrong type"))?;
        self.shaders.insert(kind, shader.clone());
        Ok(shader)
    }
}

fn shader_path(kind: LiquidShader) -> &'static str {
    match kind {
        LiquidShader::Water => "res://shaders/water.gdshader",
        LiquidShader::Magma => "res://shaders/liquid_magma.gdshader",
        LiquidShader::Mercury => "res://shaders/liquid_mercury.gdshader",
        LiquidShader::Fog => "res://shaders/liquid_fog.gdshader",
        LiquidShader::LeyLine => "res://shaders/liquid_ley_line.gdshader",
        LiquidShader::Fel => "res://shaders/liquid_fel.gdshader",
        LiquidShader::Swamp => "res://shaders/liquid_swamp.gdshader",
        LiquidShader::Azerite => "res://shaders/liquid_azerite.gdshader",
    }
}

const SLOT_UNIFORMS: [&str; TEXTURE_SLOTS] = [
    "texture_0",
    "texture_1",
    "texture_2",
    "texture_3",
    "texture_4",
    "texture_5",
];

impl LiquidSurface {
    /// LiquidMaterialManager.cpp `updateLiquidDataAnimatedTextures` advances each slot one
    /// frame per second; a crossfaded slot binds frame k and k+1 as `texture_next`
    /// (`resolveAnimatedTextures`), the shader blending by the fractional frame position.
    fn set_time(&mut self, milliseconds: f64) {
        let wrapped = (milliseconds % 3_600_000.0) as f32;
        self.material
            .set_shader_parameter("animation_time_ms", &wrapped.to_variant());
        let second = (milliseconds * 0.001) as usize;
        for (slot, frames) in self.slots.iter().enumerate() {
            if frames.is_empty() {
                continue;
            }
            let crossfade = self.crossfade.filter(|(fade_slot, _)| *fade_slot == slot);
            let frame = match crossfade {
                Some((_, interval)) if interval > 0.0 && frames.len() > 1 => {
                    (frames.len() as f64 * ((milliseconds % interval) / interval)) as usize
                }
                Some(_) => 0,
                None => second,
            };
            self.material.set_shader_parameter(
                SLOT_UNIFORMS[slot],
                &frames[frame % frames.len()].to_variant(),
            );
            if crossfade.is_some() {
                let next = &frames[(frame + 1) % frames.len()];
                self.material
                    .set_shader_parameter("texture_next", &next.to_variant());
            }
        }
    }
}

/// The LiquidType/LiquidObject inputs every liquid shader reads (the per-material
/// `create*LiquidData` packings select from these): Float, Coefficient, Color and Int arrays,
/// flow, the water colour source and wave periods, and slot frame counts.
fn bind_liquid_material(material: &mut Gd<ShaderMaterial>, native: &NativeLiquidMaterial) {
    let params = &native.params;
    let f = &params.floats;
    for (name, start) in [
        ("floats_0", 0),
        ("floats_4", 4),
        ("floats_8", 8),
        ("floats_12", 12),
    ] {
        let value = Vector4::new(f[start], f[start + 1], f[start + 2], f[start + 3]);
        material.set_shader_parameter(name, &value.to_variant());
    }
    material.set_shader_parameter("floats_16", &Vector2::new(f[16], f[17]).to_variant());
    material.set_shader_parameter(
        "depth_coefficients",
        &Vector4::from_array(params.depth_coefficients).to_variant(),
    );
    for (index, color) in params.colors.iter().enumerate() {
        material.set_shader_parameter(
            &format!("liquid_color_{index}"),
            &Vector3::from_array(*color).to_variant(),
        );
    }
    let [a, b, c, d] = params.ints;
    material.set_shader_parameter("liquid_ints", &Vector4i::new(a, b, c, d).to_variant());
    material.set_shader_parameter(
        "wave_periods",
        &Vector2::from_array(params.wave_periods).to_variant(),
    );
    material.set_shader_parameter(
        "flow",
        &Vector2::new(params.flow_speed, params.flow_direction).to_variant(),
    );
    let color_source = match params.color_source {
        WaterColorSource::Ocean => 0,
        WaterColorSource::River => 1,
        WaterColorSource::Wmo => 2,
    };
    material.set_shader_parameter("color_source", &color_source.to_variant());
    let frame_counts = native
        .slots
        .each_ref()
        .map(|frames| frames.len().max(1) as f32);
    material.set_shader_parameter(
        "frame_counts_0",
        &Vector3::new(frame_counts[0], frame_counts[1], frame_counts[2]).to_variant(),
    );
    material.set_shader_parameter(
        "frame_counts_3",
        &Vector3::new(frame_counts[3], frame_counts[4], frame_counts[5]).to_variant(),
    );
}

fn mipmapped_texture(width: u32, height: u32, pixels: &[u8]) -> Result<Gd<ImageTexture>, String> {
    let bytes = PackedByteArray::from(pixels);
    let mut image = Image::create_from_data(
        width as i32,
        height as i32,
        false,
        image::Format::RGBA8,
        &bytes,
    )
    .ok_or("Godot rejected water RGBA8 pixels")?;
    if image.generate_mipmaps() != godot::global::Error::OK {
        return Err("Cannot generate water texture mipmaps".into());
    }
    ImageTexture::create_from_image(&image).ok_or_else(|| "Cannot upload water texture".into())
}

fn water_instance(mesh: &Gd<ArrayMesh>, material: &Gd<ShaderMaterial>) -> Gd<MeshInstance3D> {
    let mut surface = MeshInstance3D::new_alloc();
    surface.set_mesh(mesh);
    surface.set_surface_override_material(0, material);
    surface
        .set_cast_shadows_setting(godot::classes::geometry_instance_3d::ShadowCastingSetting::OFF);
    surface
}

fn build_mesh(geometry: adt::WaterGeometry) -> Gd<ArrayMesh> {
    let positions: Vec<_> = geometry
        .positions
        .into_iter()
        .map(Vector3::from_array)
        .collect();
    let normals: Vec<_> = geometry
        .normals
        .into_iter()
        .map(Vector3::from_array)
        .collect();
    let uvs: Vec<_> = geometry.uvs.into_iter().map(Vector2::from_array).collect();
    let colors: Vec<_> = geometry
        .colors
        .into_iter()
        .map(|[r, g, b, a]| Color::from_rgba(r, g, b, a))
        .collect();
    // Godot ArrayMesh front faces are clockwise; the geometry's indices are counterclockwise.
    let indices: Vec<_> = geometry
        .indices
        .chunks_exact(3)
        .flat_map(|triangle| [triangle[0] as i32, triangle[2] as i32, triangle[1] as i32])
        .collect();
    let mut arrays = VarArray::new();
    arrays.resize(mesh::ArrayType::MAX.ord() as usize, &Variant::nil());
    for (kind, value) in [
        (
            mesh::ArrayType::VERTEX,
            PackedVector3Array::from(positions.as_slice()).to_variant(),
        ),
        (
            mesh::ArrayType::NORMAL,
            PackedVector3Array::from(normals.as_slice()).to_variant(),
        ),
        (
            mesh::ArrayType::TEX_UV,
            PackedVector2Array::from(uvs.as_slice()).to_variant(),
        ),
        (
            mesh::ArrayType::COLOR,
            PackedColorArray::from(colors.as_slice()).to_variant(),
        ),
        (
            mesh::ArrayType::INDEX,
            PackedInt32Array::from(indices.as_slice()).to_variant(),
        ),
    ] {
        arrays.set(kind.ord() as usize, &value);
    }
    let mut mesh = ArrayMesh::new_gd();
    mesh.add_surface_from_arrays(mesh::PrimitiveType::TRIANGLES, &arrays);
    mesh
}
