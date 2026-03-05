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
    pub sky_color: Vec4,
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
            sky_color: Vec4::new(0.6, 0.75, 0.9, 1.0),
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

/// Generate a 256x256 tileable procedural normal map using value noise.
pub fn generate_water_normal_map() -> Image {
    let size = 256u32;
    let mut heightmap = vec![0.0f32; (size * size) as usize];
    fbm_tileable_noise(&mut heightmap, size);

    let mut data = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let (nx, ny) = heightmap_normal(&heightmap, x, y, size);
            let nz = (1.0 - nx * nx - ny * ny).max(0.0).sqrt();
            let idx = ((y * size + x) * 4) as usize;
            data[idx] = ((nx * 0.5 + 0.5) * 255.0) as u8;
            data[idx + 1] = ((ny * 0.5 + 0.5) * 255.0) as u8;
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

/// Tileable FBM noise: 5 octaves of value noise, each tileable at `size`.
fn fbm_tileable_noise(out: &mut [f32], size: u32) {
    let octaves = [(4.0, 0.4), (8.0, 0.3), (16.0, 0.15), (32.0, 0.1), (64.0, 0.05)];
    for (freq, amp) in octaves {
        for y in 0..size {
            for x in 0..size {
                let v = tileable_value_noise(x as f32 * freq / size as f32, y as f32 * freq / size as f32, freq);
                out[(y * size + x) as usize] += v * amp;
            }
        }
    }
}

/// Value noise that tiles at integer period `period`.
fn tileable_value_noise(x: f32, y: f32, period: f32) -> f32 {
    let p = period as u32;
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let fx = x - x.floor();
    let fy = y - y.floor();
    let sx = fx * fx * (3.0 - 2.0 * fx); // smoothstep
    let sy = fy * fy * (3.0 - 2.0 * fy);

    let v00 = hash_f32(wrap(ix, p), wrap(iy, p));
    let v10 = hash_f32(wrap(ix + 1, p), wrap(iy, p));
    let v01 = hash_f32(wrap(ix, p), wrap(iy + 1, p));
    let v11 = hash_f32(wrap(ix + 1, p), wrap(iy + 1, p));

    let a = v00 + (v10 - v00) * sx;
    let b = v01 + (v11 - v01) * sx;
    a + (b - a) * sy
}

fn wrap(v: i32, period: u32) -> u32 {
    ((v % period as i32 + period as i32) % period as i32) as u32
}

/// Hash two u32 coords to a float in [-1, 1].
fn hash_f32(x: u32, y: u32) -> f32 {
    let mut h = x.wrapping_mul(374761393).wrapping_add(y.wrapping_mul(668265263));
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^= h >> 16;
    (h & 0xFFFF) as f32 / 32767.5 - 1.0
}

/// Compute normal from heightmap via central differences (wrapping for tileability).
fn heightmap_normal(heightmap: &[f32], x: u32, y: u32, size: u32) -> (f32, f32) {
    let s = size as usize;
    let h = |xi: u32, yi: u32| heightmap[(yi % size) as usize * s + (xi % size) as usize];
    let xp = if x + 1 >= size { 0 } else { x + 1 };
    let xm = if x == 0 { size - 1 } else { x - 1 };
    let yp = if y + 1 >= size { 0 } else { y + 1 };
    let ym = if y == 0 { size - 1 } else { y - 1 };
    let scale = 0.8; // controls normal intensity
    let dx = (h(xp, y) - h(xm, y)) * scale;
    let dy = (h(x, yp) - h(x, ym)) * scale;
    (dx.clamp(-1.0, 1.0), dy.clamp(-1.0, 1.0))
}
