use bevy::asset::RenderAssetUsages;
use bevy::image::{Image, ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, ShaderType, TextureDimension, TextureFormat,
};
use bevy::shader::ShaderRef;

/// Custom water material with scrolling normal maps, fresnel, and specular.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct WaterMaterial {
    #[uniform(0)]
    pub settings: WaterSettings,
    #[texture(1)]
    #[sampler(2)]
    pub normal_map: Handle<Image>,
}

#[derive(ShaderType, Debug, Clone)]
pub struct WaterSettings {
    pub base_color: Vec4,
    pub scroll_speed_1: Vec2,
    pub scroll_speed_2: Vec2,
    pub normal_scale: f32,
    pub fresnel_power: f32,
    pub specular_strength: f32,
    pub time: f32,
}

impl Default for WaterSettings {
    fn default() -> Self {
        Self {
            base_color: Vec4::new(0.05, 0.2, 0.4, 1.0),
            scroll_speed_1: Vec2::new(0.03, 0.02),
            scroll_speed_2: Vec2::new(-0.02, 0.04),
            normal_scale: 0.15,
            fresnel_power: 3.0,
            specular_strength: 1.5,
            time: 0.0,
        }
    }
}

impl Material for WaterMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/water.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

pub struct WaterMaterialPlugin;

impl Plugin for WaterMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<WaterMaterial>::default())
            .add_systems(Update, update_water_time);
    }
}

fn update_water_time(
    time: Res<Time>,
    mut water_materials: ResMut<Assets<WaterMaterial>>,
) {
    let t = time.elapsed_secs();
    for (_id, mat) in water_materials.iter_mut() {
        mat.settings.time = t;
    }
}

/// Generate a 256x256 tileable procedural normal map with sine wave octaves.
pub fn generate_water_normal_map() -> Image {
    let size = 256u32;
    let mut data = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let (nx, ny) = compute_normal_sample(x, y, size);
            let scale = 0.3;
            let nz = (1.0 - (nx * scale).powi(2) - (ny * scale).powi(2))
                .max(0.0)
                .sqrt();
            let idx = ((y * size + x) * 4) as usize;
            data[idx] = ((nx * scale * 0.5 + 0.5) * 255.0) as u8;
            data[idx + 1] = ((ny * scale * 0.5 + 0.5) * 255.0) as u8;
            data[idx + 2] = (nz * 255.0) as u8;
            data[idx + 3] = 255;
        }
    }
    let mut img = Image::new(
        Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..ImageSamplerDescriptor::linear()
    });
    img
}

/// Compute sine-wave normal displacement at a texel coordinate.
fn compute_normal_sample(x: u32, y: u32, size: u32) -> (f32, f32) {
    let u = x as f32 / size as f32;
    let v = y as f32 / size as f32;
    let tau = std::f32::consts::TAU;
    let nx = 0.5 * (tau * u * 4.0).sin()
        + 0.25 * (tau * (u * 8.0 + v * 4.0)).sin()
        + 0.125 * (tau * (u * 16.0 + v * 2.0)).sin();
    let ny = 0.5 * (tau * v * 4.0).sin()
        + 0.25 * (tau * (v * 8.0 + u * 4.0)).sin()
        + 0.125 * (tau * (v * 16.0 + u * 2.0)).sin();
    (nx, ny)
}
