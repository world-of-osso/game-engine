//! Authored MH2O surfaces with the retail water material of their LiquidType.
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use game_engine_core::{adt, blp, liquid_data::WaterColorSource};
use godot::{
    classes::{
        ArrayMesh, Image, ImageTexture, MeshInstance3D, Node3D, ResourceLoader, Shader,
        ShaderMaterial, image, mesh,
    },
    prelude::*,
};

use super::assets::NativeWaterMaterial;
use crate::lighting::TerrainLight;

type LiquidKey = (u16, u16);

#[derive(Default)]
pub(super) struct WaterMaterials {
    shader: Option<Gd<Shader>>,
    materials: HashMap<LiquidKey, WaterSurface>,
    textures: HashMap<u32, Gd<ImageTexture>>,
    black: Option<Gd<ImageTexture>>,
    light: Option<TerrainLight>,
}

/// One LiquidType's material and its texture frames, cycled once per second
/// (LiquidMaterialManager.cpp `updateLiquidDataAnimatedTextures`).
struct WaterSurface {
    material: Gd<ShaderMaterial>,
    bump: Vec<Gd<ImageTexture>>,
    foam: Vec<Gd<ImageTexture>>,
}

impl WaterMaterials {
    /// A layer whose water material is unavailable is reported and left out, as an
    /// unbuildable tile is.
    pub fn build(
        &mut self,
        root: &adt::Root,
        materials: &BTreeMap<LiquidKey, Result<Arc<NativeWaterMaterial>, String>>,
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
        materials: &BTreeMap<LiquidKey, Result<Arc<NativeWaterMaterial>, String>>,
    ) -> Result<Option<(Gd<ArrayMesh>, Gd<ShaderMaterial>)>, String> {
        let key = (layer.liquid_type, layer.liquid_object);
        let native = match materials.get(&key) {
            Some(Ok(native)) => native,
            Some(Err(error)) => {
                godot_error!("MH2O chunk {chunk} water layer {key:?}: {error}");
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
            .ok_or("ADT water requires the shared material clock")?;
        let milliseconds = clock
            .call("elapsed_time_ms", &[])
            .try_to::<f64>()
            .map_err(|error| format!("Cannot read water material time: {error}"))?;
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

    /// One lit material outside any tile, for the water material fixture.
    pub fn standalone(
        mut self,
        native: &NativeWaterMaterial,
        light: &TerrainLight,
    ) -> Result<Gd<ShaderMaterial>, String> {
        self.light = Some(light.clone());
        Ok(self.create_surface(native)?.material)
    }

    fn material(
        &mut self,
        key: LiquidKey,
        native: &NativeWaterMaterial,
    ) -> Result<Gd<ShaderMaterial>, String> {
        if let Some(surface) = self.materials.get(&key) {
            return Ok(surface.material.clone());
        }
        let surface = self.create_surface(native)?;
        let material = surface.material.clone();
        self.materials.insert(key, surface);
        Ok(material)
    }

    fn create_surface(&mut self, native: &NativeWaterMaterial) -> Result<WaterSurface, String> {
        let shader = self.shader()?;
        let mut material = ShaderMaterial::new_gd();
        material.set_shader(&shader);
        bind_water_material(&mut material, native);
        if let Some(light) = &self.light {
            light.bind_water(&mut material);
        }
        let bump = self.frames(&native.bump)?;
        let foam = self.frames(&native.foam)?;
        let mut surface = WaterSurface {
            material,
            bump,
            foam,
        };
        surface.set_time(0.0);
        Ok(surface)
    }

    /// Frames of one texture slot; an empty slot is the reference's one black pixel.
    fn frames(
        &mut self,
        frames: &[(u32, Option<Arc<blp::RgbaImage>>)],
    ) -> Result<Vec<Gd<ImageTexture>>, String> {
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
            .map_err(|error| format!("Water texture FDID {fdid}: {error}"))?;
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

    fn shader(&mut self) -> Result<Gd<Shader>, String> {
        if let Some(shader) = &self.shader {
            return Ok(shader.clone());
        }
        let shader = ResourceLoader::singleton()
            .load("res://shaders/water.gdshader")
            .ok_or("Cannot load ADT water shader")?
            .try_cast::<Shader>()
            .map_err(|_| "ADT water shader resource has wrong type")?;
        self.shader = Some(shader.clone());
        Ok(shader)
    }
}

impl WaterSurface {
    fn set_time(&mut self, milliseconds: f64) {
        let wrapped = (milliseconds % 3_600_000.0) as f32;
        self.material
            .set_shader_parameter("animation_time_ms", &wrapped.to_variant());
        let second = (milliseconds * 0.001) as usize;
        for (name, frames) in [("bump_map", &self.bump), ("foam_map", &self.foam)] {
            let frame = &frames[second % frames.len()];
            self.material
                .set_shader_parameter(name, &frame.to_variant());
        }
    }
}

/// LiquidWater.cpp `createWaterLiquidData` packing of the LiquidType/LiquidObject inputs.
fn bind_water_material(material: &mut Gd<ShaderMaterial>, native: &NativeWaterMaterial) {
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
