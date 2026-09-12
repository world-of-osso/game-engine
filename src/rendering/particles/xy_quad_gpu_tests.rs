//! Authored ground-aligned particle quads must not turn into camera-facing walls.
use bevy::asset::RenderAssetUsages;
use bevy::camera::{RenderTarget, visibility::RenderLayers};
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
use crate::asset::m2_particle::M2ParticleEmitter;

const RAW_XY_QUAD: u32 = 0x0000_1000;
const TARGET_SIZE: u32 = 64;
const VIEW_COUNT: usize = 4;
const MIN_VISIBLE_PIXELS: usize = 64;
const MAX_VISIBLE_PIXELS: usize = 2048;
const MAX_EDGE_PIXELS: usize = TARGET_SIZE as usize;
const WHITE_THRESHOLD: u8 = 128;
const STABLE_BATCHES: u8 = 3;

#[test]
#[ignore = "requires GPU; run explicitly with --ignored --test-threads=1"]
fn authored_xy_quad_is_visible_from_above_and_edge_on_from_the_side() {
    let model =
        crate::asset::m2::load_m2_uncached(std::path::Path::new("data/models/2904370.m2"), &[0; 3])
            .expect("authored misty ripple model loads");
    let authored = model
        .particle_emitters
        .first()
        .expect("ripple has an emitter");
    assert_ne!(
        authored.flags & RAW_XY_QUAD,
        0,
        "fixture is an authored XY quad"
    );
    let mut app = create_headless_app();
    let targets = std::array::from_fn(|index| {
        spawn_particle(&mut app, authored.flags, index);
        spawn_camera(&mut app, index)
    });
    let counts = capture_settled_areas(&mut app, &targets);
    println!("White pixel areas [XY top, XY side, billboard top, billboard side]: {counts:?}");
    for index in [0, 2, 3] {
        assert_visible_area(counts[index], index);
    }
    assert!(
        counts[1] <= MAX_EDGE_PIXELS && counts[1] * 8 <= counts[0],
        "ground quad must be edge-on from a level camera, not a facing wall: {counts:?}",
    );
}

fn create_headless_app() -> App {
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

fn stationary_emitter(authored_flags: u32, billboard: bool) -> M2ParticleEmitter {
    let flags = if billboard {
        authored_flags & !RAW_XY_QUAD
    } else {
        authored_flags
    };
    M2ParticleEmitter {
        flags,
        texture_fdid: Some(1),
        blend_type: 2,
        emitter_type: 1,
        tile_rows: 1,
        tile_cols: 1,
        lifespan: 60.0,
        emission_rate: 1.0,
        colors: [[255.0; 3]; 3],
        opacity: [1.0; 3],
        scales: [[1.0; 2]; 3],
        ..default()
    }
}

fn spawn_particle(app: &mut App, authored_flags: u32, index: usize) {
    let emitter = stationary_emitter(authored_flags, index >= 2);
    let mut effect = build_effect_asset_with_mode(
        &emitter,
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
    let texture = insert_white_texture(app);
    app.world_mut().spawn((
        ParticleEffect::new(effect),
        EffectMaterial {
            images: vec![texture],
        },
        Transform::IDENTITY,
        RenderLayers::layer(index),
    ));
}

fn insert_white_texture(app: &mut App) -> Handle<Image> {
    let image = Image::new(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![255; 4],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    );
    app.world_mut().resource_mut::<Assets<Image>>().add(image)
}

fn spawn_camera(app: &mut App, index: usize) -> Handle<Image> {
    let image = Image::new_target_texture(
        TARGET_SIZE,
        TARGET_SIZE,
        TextureFormat::Rgba8UnormSrgb,
        None,
    );
    let target = app.world_mut().resource_mut::<Assets<Image>>().add(image);
    let transform = if index % 2 == 0 {
        Transform::from_xyz(0.0, 5.0, 0.0).looking_at(Vec3::ZERO, Vec3::NEG_Z)
    } else {
        Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y)
    };
    app.world_mut().spawn((
        Camera3d::default(),
        Camera {
            order: index as isize,
            clear_color: Color::BLACK.into(),
            ..default()
        },
        RenderTarget::Image(target.clone().into()),
        transform,
        Tonemapping::None,
        Msaa::Off,
        RenderLayers::layer(index),
    ));
    target
}

fn capture_settled_areas(
    app: &mut App,
    targets: &[Handle<Image>; VIEW_COUNT],
) -> [usize; VIEW_COUNT] {
    let (sender, receiver) = mpsc::channel();
    let mut samples = [None; VIEW_COUNT];
    let mut previous = [0; VIEW_COUNT];
    let mut stable = 0;
    let mut pending = false;
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        app.update();
        assert_no_pipeline_errors(app);
        if !pending {
            request_capture_batch(app, targets, &sender);
            pending = true;
        }
        receive_areas(&receiver, &mut samples);
        if samples.iter().any(Option::is_none) {
            continue;
        }
        let counts = samples.map(|value| value.expect("complete capture batch"));
        stable = next_stable_count(counts, previous, stable);
        previous = counts;
        samples = [None; VIEW_COUNT];
        pending = false;
        if stable >= STABLE_BATCHES {
            return counts;
        }
    }
    panic!(
        "orientation views did not settle with visible controls within ten seconds: {previous:?}"
    );
}

fn request_capture_batch(
    app: &mut App,
    targets: &[Handle<Image>; VIEW_COUNT],
    sender: &mpsc::Sender<(usize, Image)>,
) {
    for (index, target) in targets.iter().enumerate() {
        let sender = sender.clone();
        app.world_mut()
            .spawn(Screenshot::image(target.clone()))
            .observe(move |capture: On<ScreenshotCaptured>| {
                sender
                    .send((index, capture.image.clone()))
                    .expect("orientation receiver exists");
            });
    }
}

fn receive_areas(
    receiver: &mpsc::Receiver<(usize, Image)>,
    samples: &mut [Option<usize>; VIEW_COUNT],
) {
    for (index, image) in receiver.try_iter() {
        let rgba = image
            .try_into_dynamic()
            .expect("readable orientation target")
            .to_rgba8();
        samples[index] = Some(
            rgba.pixels()
                .filter(|pixel| {
                    pixel.0[..3]
                        .iter()
                        .all(|channel| *channel >= WHITE_THRESHOLD)
                })
                .count(),
        );
    }
}

fn next_stable_count(counts: [usize; VIEW_COUNT], previous: [usize; VIEW_COUNT], stable: u8) -> u8 {
    let positives_visible = [0, 2, 3]
        .into_iter()
        .all(|index| counts[index] >= MIN_VISIBLE_PIXELS);
    if positives_visible && counts == previous {
        stable + 1
    } else {
        0
    }
}

fn assert_visible_area(area: usize, index: usize) {
    assert!(
        (MIN_VISIBLE_PIXELS..=MAX_VISIBLE_PIXELS).contains(&area),
        "view {index} must contain a visible unclipped quad, got {area} white pixels",
    );
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
            _ => panic!("XY quad GPU pipeline failed: {error}\n{error:#?}"),
        }
    }
}
