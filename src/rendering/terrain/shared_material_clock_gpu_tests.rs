//! Bounded rendering proof for the actual terrain and water shared-clock shaders.
//! Run the built test executable under an external ten-second process timeout.
use super::{TerrainMaterial, TerrainMaterialPlugin, TerrainMaterialSettings};
use crate::water_material::{WaterMaterial, WaterMaterialPlugin, WaterSettings};
use bevy::asset::{AssetEvent, RenderAssetUsages};
use bevy::camera::{RenderTarget, visibility::RenderLayers};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::RenderApp;
use bevy::render::render_resource::{
    CachedPipelineState, Extent3d, PipelineCache, TextureDimension, TextureFormat,
    TextureViewDescriptor, TextureViewDimension,
};
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[derive(Resource)]
struct FixtureClock(Duration);

fn set_shared_clock(clock: Res<FixtureClock>, mut time: ResMut<Time>) {
    let mut fixed = Time::default();
    fixed.advance_by(clock.0);
    *time = fixed;
}

fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: format!("{}/assets", env!("CARGO_MANIFEST_DIR")),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: bevy::window::ExitCondition::DontExit,
                ..default()
            })
            .disable::<bevy::audio::AudioPlugin>()
            .disable::<bevy::winit::WinitPlugin>()
            .disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>(),
    );
    app.add_plugins((TerrainMaterialPlugin, WaterMaterialPlugin));
    app.insert_resource(FixtureClock(Duration::ZERO));
    app.add_systems(Last, set_shared_clock);
    app.finish();
    app.cleanup();
    app
}

fn camera_target(app: &mut App, layer: usize) -> Handle<Image> {
    let target = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new_target_texture(
            32,
            32,
            TextureFormat::Rgba8UnormSrgb,
            None,
        ));
    app.world_mut().spawn((
        Camera3d::default(),
        Camera {
            order: layer as isize,
            clear_color: Color::BLACK.into(),
            ..default()
        },
        RenderTarget::Image(target.clone().into()),
        RenderLayers::layer(layer),
        Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        Msaa::Off,
        Tonemapping::None,
    ));
    target
}

fn texture(app: &mut App, normals: bool) -> Handle<Image> {
    let mut data = Vec::new();
    for y in 0..8 {
        for x in 0..8 {
            let pixel = match (normals, (x + y / 2) % 3) {
                (true, 0) => [48, 128, 210, 255],
                (true, 1) => [208, 176, 210, 255],
                (true, _) => [128, 48, 240, 255],
                (false, 0) => [230, 35, 25, 255],
                (false, 1) => [25, 210, 50, 255],
                (false, _) => [35, 55, 225, 255],
            };
            data.extend_from_slice(&pixel);
        }
    }
    let format = if normals {
        TextureFormat::Rgba8Unorm
    } else {
        TextureFormat::Rgba8UnormSrgb
    };
    let mut image = Image::new(
        Extent3d {
            width: 8,
            height: 8,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        format,
        RenderAssetUsages::default(),
    );
    image.sampler = crate::rendering::image_sampler::repeat_linear_sampler();
    app.world_mut().resource_mut::<Assets<Image>>().add(image)
}

fn solid_texture(app: &mut App, cube: bool) -> Handle<Image> {
    let layers = if cube { 6 } else { 1 };
    let mut image = Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: layers,
        },
        TextureDimension::D2,
        &[255, 255, 255, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::linear();
    if cube {
        image.texture_view_descriptor = Some(TextureViewDescriptor {
            dimension: Some(TextureViewDimension::Cube),
            ..default()
        });
    }
    app.world_mut().resource_mut::<Assets<Image>>().add(image)
}

fn create_terrain(app: &mut App) -> Handle<TerrainMaterial> {
    let ground = texture(app, false);
    let white = solid_texture(app, false);
    let environment = solid_texture(app, true);
    app.world_mut()
        .resource_mut::<Assets<TerrainMaterial>>()
        .add(TerrainMaterial {
            settings: TerrainMaterialSettings {
                config: Vec4::new(1.0, 0.0, 1.0, 0.0),
                surface: Vec4::new(1.0, 0.0, 0.0, 0.0),
                layer_params_0: Vec4::new(1.0, 0.0, 0.0, 1.0),
                layer_params_1: Vec4::new(1.0, 0.0, 0.0, 1.0),
                layer_params_2: Vec4::new(1.0, 0.0, 0.0, 1.0),
                layer_params_3: Vec4::new(1.0, 0.0, 0.0, 1.0),
                animation_params_0: Vec4::new(0.137, 0.071, 0.0, 0.0),
                animation_params_1: Vec4::ZERO,
                animation_params_2: Vec4::ZERO,
                animation_params_3: Vec4::ZERO,
            },
            ground_0: ground.clone(),
            ground_1: ground.clone(),
            ground_2: ground.clone(),
            ground_3: ground,
            height_0: white.clone(),
            height_1: white.clone(),
            height_2: white.clone(),
            height_3: white.clone(),
            alpha_packed: white.clone(),
            shadow_map: white,
            environment_map: environment,
        })
}

fn create_water(app: &mut App) -> Handle<WaterMaterial> {
    let normal_map = texture(app, true);
    app.world_mut()
        .resource_mut::<Assets<WaterMaterial>>()
        .add(WaterMaterial {
            settings: WaterSettings {
                scroll_speed_1: Vec2::new(0.137, 0.071),
                scroll_speed_2: Vec2::new(-0.083, 0.193),
                normal_scale: 1.5,
                fresnel_power: 1.0,
                specular_strength: 2.0,
                ..default()
            },
            normal_map,
        })
}

fn spawn_surfaces(app: &mut App) -> (Handle<TerrainMaterial>, Handle<WaterMaterial>) {
    let terrain = create_terrain(app);
    let water = create_water(app);
    let mut mesh = Mesh::from(Rectangle::new(4.0, 4.0));
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![[0.5, 0.5, 0.5, 1.0]; 4]);
    let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh);
    app.world_mut().spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(terrain.clone()),
        Transform::default(),
        RenderLayers::layer(0),
    ));
    app.world_mut().spawn((
        Mesh3d(mesh),
        MeshMaterial3d(water.clone()),
        Transform::default(),
        RenderLayers::layer(1),
    ));
    app.world_mut().spawn((
        DirectionalLight {
            illuminance: 10_000.0,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        RenderLayers::from_layers(&[0, 1]),
    ));
    (terrain, water)
}

fn assert_no_pipeline_errors(app: &App) {
    let cache = app.sub_app(RenderApp).world().resource::<PipelineCache>();
    for pipeline in cache.pipelines() {
        if let CachedPipelineState::Err(error) = &pipeline.state {
            match error {
                bevy::shader::ShaderCacheError::ShaderNotLoaded(_)
                | bevy::shader::ShaderCacheError::ShaderImportNotYetAvailable => {}
                _ => panic!("terrain/water shader failed: {error}\n{error:#?}"),
            }
        }
    }
}

fn modified_materials<M: Asset>(app: &mut App) -> usize {
    app.world_mut()
        .resource_mut::<Messages<AssetEvent<M>>>()
        .drain()
        .filter(|event| matches!(event, AssetEvent::Modified { .. }))
        .count()
}

fn rendered_pixels(image: Image) -> Vec<u8> {
    let rgba = image
        .try_into_dynamic()
        .expect("readable GPU image")
        .to_rgba8();
    // Ignore edges/clear color; both surfaces fill this central region.
    let mut pixels = Vec::new();
    for y in 8..24 {
        for x in 8..24 {
            pixels.extend_from_slice(&rgba.get_pixel(x, y).0[..3]);
        }
    }
    pixels
}

fn submit_surface_captures(
    app: &mut App,
    targets: &[Handle<Image>; 2],
    pending: &mut [bool; 2],
    captured: &[Option<Vec<u8>>; 2],
    sender: &mpsc::Sender<(usize, Image)>,
) {
    for (index, target) in targets.iter().enumerate() {
        if pending[index] || captured[index].is_some() {
            continue;
        }
        let sender = sender.clone();
        app.world_mut()
            .spawn(Screenshot::image(target.clone()))
            .observe(move |capture: On<ScreenshotCaptured>| {
                sender
                    .send((index, capture.image.clone()))
                    .expect("capture receiver exists");
            });
        pending[index] = true;
    }
}

fn capture_surfaces(
    app: &mut App,
    targets: &[Handle<Image>; 2],
    deadline: Instant,
    assert_static_assets: bool,
) -> [Vec<u8>; 2] {
    let (sender, receiver) = mpsc::channel();
    let mut pending = [false; 2];
    let mut captured: [Option<Vec<u8>>; 2] = [None, None];
    while Instant::now() < deadline {
        app.update();
        assert_no_pipeline_errors(app);
        if assert_static_assets {
            assert_eq!(
                modified_materials::<TerrainMaterial>(app),
                0,
                "time changed terrain asset"
            );
            assert_eq!(
                modified_materials::<WaterMaterial>(app),
                0,
                "time changed water asset"
            );
        }
        submit_surface_captures(app, targets, &mut pending, &captured, &sender);
        for (index, image) in receiver.try_iter() {
            pending[index] = false;
            let pixels = rendered_pixels(image);
            if pixels.iter().filter(|&&channel| channel > 20).count() > 32 {
                captured[index] = Some(pixels);
            }
        }
        if let [Some(terrain), Some(water)] = &captured {
            return [terrain.clone(), water.clone()];
        }
    }
    panic!("terrain/water GPU capture exceeded total eight-second fixture deadline");
}

#[test]
#[ignore = "requires GPU; run built executable under a ten-second process timeout"]
fn shared_clock_animates_terrain_and_water_without_material_mutations() {
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut app = test_app();
    assert!(
        Instant::now() < deadline,
        "GPU initialization consumed fixture deadline"
    );
    let targets = [camera_target(&mut app, 0), camera_target(&mut app, 1)];
    let _materials = spawn_surfaces(&mut app);
    let before = capture_surfaces(&mut app, &targets, deadline, false);
    modified_materials::<TerrainMaterial>(&mut app);
    modified_materials::<WaterMaterial>(&mut app);
    app.world_mut().resource_mut::<FixtureClock>().0 = Duration::from_secs(1);
    let after = capture_surfaces(&mut app, &targets, deadline, true);
    for (index, name) in ["terrain", "water"].iter().enumerate() {
        let changed = before[index]
            .iter()
            .zip(&after[index])
            .filter(|(a, b)| a.abs_diff(**b) > 3)
            .count();
        assert!(
            changed > 16,
            "{name} did not visibly animate with shared time: {changed} changed channels"
        );
    }
    assert!(
        Instant::now() < deadline,
        "fixture exceeded total eight-second deadline"
    );
}
