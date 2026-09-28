use bevy::asset::RenderAssetUsages;
use bevy::image::{Image, ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, ShaderType, TextureDimension, TextureFormat,
};
use bevy::shader::ShaderRef;

#[path = "../../water_material_data.rs"]
mod water_material_data;

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
    pub sky_color: Vec4,
    /// Vertex displacement wave amplitude (0 = flat).
    pub wave_amplitude: f32,
    /// Wave frequency (higher = more waves per unit).
    pub wave_frequency: f32,
    /// Wave propagation speed multiplier.
    pub wave_speed: f32,
    /// Shoreline foam intensity (0 = no foam).
    pub foam_intensity: f32,
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
            sky_color: Vec4::new(0.6, 0.75, 0.9, 1.0),
            wave_amplitude: 0.15,
            wave_frequency: 2.0,
            wave_speed: 1.5,
            foam_intensity: 0.6,
        }
    }
}

/// Compute the vertex displacement for a water surface point at a given time.
/// Uses two overlapping sine waves for a more natural look.
pub fn wave_displacement(x: f32, z: f32, time: f32, settings: &WaterSettings) -> f32 {
    if settings.wave_amplitude <= 0.0 {
        return 0.0;
    }
    let t = time * settings.wave_speed;
    let wave1 = (x * settings.wave_frequency + t).sin();
    let wave2 = (z * settings.wave_frequency * 0.7 + t * 1.3).sin();
    settings.wave_amplitude * (wave1 * 0.6 + wave2 * 0.4)
}

/// Compute shoreline foam factor (0.0–1.0) based on water depth.
/// Foam appears where depth is shallow (near terrain edges).
pub fn shoreline_foam_factor(depth: f32, settings: &WaterSettings) -> f32 {
    if settings.foam_intensity <= 0.0 || depth < 0.0 {
        return 0.0;
    }
    let foam_depth_threshold = 1.5;
    let raw = (1.0 - (depth / foam_depth_threshold).min(1.0)) * settings.foam_intensity;
    raw.clamp(0.0, 1.0)
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

#[cfg(test)]
#[path = "water_material_clock_tests.rs"]
mod clock_tests;

/// Water animation uses Bevy's shared virtual shader clock. Its default one-hour
/// wrap preserves pause/speed behavior, but restarts UV phase at the wrap boundary.
pub struct WaterMaterialPlugin;

impl Plugin for WaterMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<WaterMaterial>::default());
    }
}

/// Generate a 256x256 tileable procedural normal map using value noise.
pub fn generate_water_normal_map() -> Image {
    let mut img = Image::new(
        Extent3d {
            width: water_material_data::WATER_NORMAL_SIZE,
            height: water_material_data::WATER_NORMAL_SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        water_material_data::generate_water_normal_rgba(),
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

#[cfg(test)]
mod tests {
    use super::*;

    // --- Wave displacement ---

    #[test]
    fn wave_zero_amplitude_returns_zero() {
        let settings = WaterSettings {
            wave_amplitude: 0.0,
            ..Default::default()
        };
        assert_eq!(wave_displacement(5.0, 3.0, 1.0, &settings), 0.0);
    }

    #[test]
    fn wave_displacement_bounded_by_amplitude() {
        let settings = WaterSettings::default();
        for x in 0..10 {
            for z in 0..10 {
                for t in 0..20 {
                    let d = wave_displacement(x as f32, z as f32, t as f32 * 0.1, &settings);
                    assert!(
                        d.abs() <= settings.wave_amplitude + 0.01,
                        "displacement {d} exceeds amplitude {}",
                        settings.wave_amplitude
                    );
                }
            }
        }
    }

    #[test]
    fn wave_displacement_varies_with_time() {
        let settings = WaterSettings::default();
        let d0 = wave_displacement(5.0, 5.0, 0.0, &settings);
        let d1 = wave_displacement(5.0, 5.0, 1.0, &settings);
        assert_ne!(d0, d1, "displacement should change over time");
    }

    #[test]
    fn wave_displacement_varies_with_position() {
        let settings = WaterSettings::default();
        let d_a = wave_displacement(0.0, 0.0, 0.5, &settings);
        let d_b = wave_displacement(5.0, 5.0, 0.5, &settings);
        assert_ne!(d_a, d_b, "displacement should vary across surface");
    }

    // --- Shoreline foam ---

    #[test]
    fn foam_zero_depth_is_max() {
        let settings = WaterSettings::default();
        let foam = shoreline_foam_factor(0.0, &settings);
        assert!((foam - settings.foam_intensity).abs() < 0.01);
    }

    #[test]
    fn foam_deep_water_is_zero() {
        let settings = WaterSettings::default();
        let foam = shoreline_foam_factor(10.0, &settings);
        assert_eq!(foam, 0.0);
    }

    #[test]
    fn foam_at_threshold_depth() {
        let settings = WaterSettings::default();
        let foam = shoreline_foam_factor(1.5, &settings);
        assert!(foam.abs() < 0.01, "foam at threshold should be ~0");
    }

    #[test]
    fn foam_mid_depth() {
        let settings = WaterSettings::default();
        let foam = shoreline_foam_factor(0.75, &settings);
        assert!(foam > 0.0 && foam < settings.foam_intensity);
    }

    #[test]
    fn foam_negative_depth_returns_zero() {
        let settings = WaterSettings::default();
        assert_eq!(shoreline_foam_factor(-1.0, &settings), 0.0);
    }

    #[test]
    fn foam_disabled_returns_zero() {
        let settings = WaterSettings {
            foam_intensity: 0.0,
            ..Default::default()
        };
        assert_eq!(shoreline_foam_factor(0.5, &settings), 0.0);
    }

    #[test]
    fn foam_clamped_to_one() {
        let settings = WaterSettings {
            foam_intensity: 5.0, // very high
            ..Default::default()
        };
        let foam = shoreline_foam_factor(0.0, &settings);
        assert!(foam <= 1.0, "foam should clamp to 1.0");
    }

    // --- Default settings ---

    #[test]
    fn default_settings_have_waves_and_foam() {
        let s = WaterSettings::default();
        assert!(s.wave_amplitude > 0.0);
        assert!(s.wave_frequency > 0.0);
        assert!(s.wave_speed > 0.0);
        assert!(s.foam_intensity > 0.0);
    }

    #[test]
    fn normal_map_generates_valid_image() {
        let img = generate_water_normal_map();
        assert_eq!(img.width(), 256);
        assert_eq!(img.height(), 256);
    }
}
