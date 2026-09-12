//! GPU coverage of authored three-texture particle color and alpha composition.
use bevy::asset::RenderAssetUsages;
use bevy::camera::RenderTarget;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::render::RenderApp;
use bevy::render::render_resource::{
    CachedPipelineState, Extent3d, PipelineCache, TextureDimension, TextureFormat,
};
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use bevy_hanabi::prelude::*;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use super::effect_builder::build_effect_asset_with_mode;
use super::{ParticleSpawnMode, ParticleSpawnSource};
use crate::asset::m2_particle::{M2ParticleEmitter, M2ParticleMultiTexture};

const RAW_MULTITEXTURE: u32 = 0x1000_0000;
const RAW_THREE_COLOR_TEXTURES: u32 = 0x4000_0000;
const ALPHA_BLEND: u8 = 2;
const EMITTER_GRAY: f32 = 128.0;
const TARGET_SIZE: u32 = 32;
const PIXEL_TOLERANCE: u8 = 3;
const TEXTURE_PIXELS: [[u8; 4]; 3] = [
    [255, 128, 192, 255],
    [128, 255, 192, 192],
    [180, 90, 255, 128],
];

#[test]
#[ignore = "requires GPU; run explicitly with --ignored --test-threads=1"]
fn particle_multitexture_multiplies_three_colors_and_alphas() {
    let mut app = create_headless_particle_app();
    let target = spawn_camera_target(&mut app);
    spawn_single_multitexture_particle(&mut app);
    let pixel = capture_settled_center(&mut app, target);
    let expected = expected_composited_pixel();
    println!("Three-texture particle: actual={pixel:?}, expected={expected:?}");
    for channel in 0..3 {
        assert!(
            pixel[channel].abs_diff(expected[channel]) <= PIXEL_TOLERANCE,
            "three texture colors and alphas must contribute to channel {channel}: actual={pixel:?}, expected={expected:?}",
        );
    }
    assert_eq!(pixel[3], 255, "particle composites over opaque black");
}

fn create_headless_particle_app() -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: bevy::window::ExitCondition::DontExit,
                ..default()
            })
            .disable::<bevy::audio::AudioPlugin>()
            .disable::<bevy::winit::WinitPlugin>()
            .disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>(),
    );
    app.add_plugins(HanabiPlugin);
    app.finish();
    app.cleanup();
    app
}

fn spawn_camera_target(app: &mut App) -> Handle<Image> {
    let image = Image::new_target_texture(
        TARGET_SIZE,
        TARGET_SIZE,
        TextureFormat::Rgba8UnormSrgb,
        None,
    );
    let target = app.world_mut().resource_mut::<Assets<Image>>().add(image);
    app.world_mut().spawn((
        Camera3d::default(),
        Camera {
            clear_color: Color::BLACK.into(),
            ..default()
        },
        RenderTarget::Image(target.clone().into()),
        Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        Tonemapping::None,
        Msaa::Off,
    ));
    target
}

fn stationary_multitexture_emitter() -> M2ParticleEmitter {
    M2ParticleEmitter {
        flags: RAW_MULTITEXTURE | RAW_THREE_COLOR_TEXTURES,
        texture_fdid: Some(1),
        multi_texture: Some(M2ParticleMultiTexture {
            texture_indices: [0, 1, 2],
            texture_fdids: [Some(1), Some(2), Some(3)],
            uv_scale_bytes: [32; 2],
            velocity_midpoints: [[0.0; 2]; 2],
            velocity_ranges: [[0.0; 2]; 2],
        }),
        blend_type: ALPHA_BLEND,
        emitter_type: 1,
        tile_rows: 1,
        tile_cols: 1,
        lifespan: 60.0,
        emission_rate: 1.0,
        colors: [[EMITTER_GRAY; 3]; 3],
        opacity: [1.0; 3],
        scales: [[2.0; 2]; 3],
        ..default()
    }
}

fn spawn_single_multitexture_particle(app: &mut App) {
    let mut effect = build_effect_asset_with_mode(
        &stationary_multitexture_emitter(),
        1.0,
        1.0,
        ParticleSpawnMode::BurstOnce,
        ParticleSpawnSource::Standalone,
        &[],
    );
    effect.spawner = SpawnerSettings::once(1.0.into()).with_emit_on_start(true);
    let effect = app
        .world_mut()
        .resource_mut::<Assets<EffectAsset>>()
        .add(effect);
    let images = TEXTURE_PIXELS
        .into_iter()
        .map(|pixel| insert_linear_texture(app, pixel))
        .collect();
    app.world_mut().spawn((
        ParticleEffect::new(effect),
        EffectMaterial { images },
        Transform::IDENTITY,
    ));
}

fn insert_linear_texture(app: &mut App, pixel: [u8; 4]) -> Handle<Image> {
    let image = Image::new(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixel.to_vec(),
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    );
    app.world_mut().resource_mut::<Assets<Image>>().add(image)
}

fn expected_composited_pixel() -> [u8; 4] {
    let alpha = TEXTURE_PIXELS
        .iter()
        .fold(1.0, |value, pixel| value * f32::from(pixel[3]) / 255.0);
    let rgb: [u8; 3] = std::array::from_fn(|channel| {
        let source = TEXTURE_PIXELS
            .iter()
            .fold(EMITTER_GRAY / 255.0, |value, pixel| {
                value * f32::from(pixel[channel]) / 255.0
            });
        encode_srgb_channel(source * alpha)
    });
    [rgb[0], rgb[1], rgb[2], 255]
}

fn encode_srgb_channel(linear: f32) -> u8 {
    // Standard sRGB transfer function; input fixture textures are linear UNORM.
    let encoded = if linear <= 0.003_130_8 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0).round() as u8
}

fn capture_settled_center(app: &mut App, target: Handle<Image>) -> [u8; 4] {
    let (sender, receiver) = mpsc::channel::<Image>();
    let mut last_pixel = [0; 4];
    let mut stable_frames = 0;
    let mut pending = false;
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        app.update();
        assert_no_pipeline_errors(app);
        if !pending {
            request_screenshot(app, &target, sender.clone());
            pending = true;
        }
        for image in receiver.try_iter() {
            pending = false;
            let pixel = center_pixel(image);
            stable_frames = count_stable_nonblack_frames(pixel, last_pixel, stable_frames);
            last_pixel = pixel;
        }
        if stable_frames >= 3 {
            return last_pixel;
        }
    }
    panic!("three-texture particle did not settle within ten seconds: {last_pixel:?}");
}

fn request_screenshot(app: &mut App, target: &Handle<Image>, sender: mpsc::Sender<Image>) {
    app.world_mut()
        .spawn(Screenshot::image(target.clone()))
        .observe(move |capture: On<ScreenshotCaptured>| {
            sender
                .send(capture.image.clone())
                .expect("particle screenshot receiver exists");
        });
}

fn center_pixel(image: Image) -> [u8; 4] {
    image
        .try_into_dynamic()
        .expect("particle target is readable")
        .to_rgba8()
        .get_pixel(TARGET_SIZE / 2, TARGET_SIZE / 2)
        .0
}

fn count_stable_nonblack_frames(pixel: [u8; 4], previous: [u8; 4], stable: u8) -> u8 {
    let visible = pixel[..3].iter().any(|channel| *channel > 0);
    if visible && pixel == previous {
        stable + 1
    } else {
        u8::from(visible)
    }
}

fn assert_no_pipeline_errors(app: &App) {
    let cache = app.sub_app(RenderApp).world().resource::<PipelineCache>();
    for pipeline in cache.pipelines() {
        let CachedPipelineState::Err(error) = &pipeline.state else {
            continue;
        };
        match error {
            bevy::shader::ShaderCacheError::ShaderNotLoaded(_)
            | bevy::shader::ShaderCacheError::ShaderImportNotYetAvailable => {}
            _ => panic!("particle GPU pipeline failed: {error}\n{error:#?}"),
        }
    }
}
