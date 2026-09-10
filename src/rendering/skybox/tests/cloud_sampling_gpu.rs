//! Render the actual sky shader across the spherical longitude wrap.
use super::*;
use bevy::camera::RenderTarget;
use bevy::camera::visibility::RenderLayers;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::material::descriptor::PipelineDescriptor;
use bevy::render::RenderApp;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{CachedPipelineState, PipelineCache, TextureFormat};
use bevy::render::texture::GpuImage;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use bevy::shader::{Shader, ShaderCacheError};
use std::sync::mpsc;
use std::time::{Duration, Instant};

fn render_app() -> App {
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
    app.add_plugins(MaterialPlugin::<SkyMaterial>::default());
    app.finish();
    app.cleanup();
    app
}

fn spawn_seam_camera(app: &mut App, longitude_side: f32, order: isize) -> Handle<Image> {
    let target = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new_target_texture(
            33,
            33,
            TextureFormat::Rgba8UnormSrgb,
            None,
        ));
    let direction = Vec3::new(-0.8660254, 0.5, longitude_side * 0.01);
    app.world_mut().spawn((
        Camera3d::default(),
        Camera {
            order,
            clear_color: Color::srgb(1.0, 0.0, 1.0).into(),
            ..default()
        },
        RenderTarget::Image(target.clone().into()),
        Transform::default().looking_at(direction, Vec3::Y),
        Msaa::Off,
        Tonemapping::None,
    ));
    target
}

fn spawn_periodic_test_sky(app: &mut App) {
    let mut pixels = Vec::new();
    for x in 0..256 {
        let phase = (x as f32 + 0.5) / 256.0 * std::f32::consts::TAU;
        let value = ((0.45 + 0.1 * phase.sin()) * 255.0).round() as u8;
        pixels.extend_from_slice(&[value, value, value, 255]);
    }
    let mut image = crate::rgba_image(pixels, 256, 1);
    image.texture_descriptor.format = TextureFormat::Rgba8Unorm;
    image.sampler = crate::rendering::image_sampler::repeat_linear_sampler();
    let texture = app.world_mut().resource_mut::<Assets<Image>>().add(image);
    let material = app
        .world_mut()
        .resource_mut::<Assets<SkyMaterial>>()
        .add(SkyMaterial {
            uniforms: SkyUniforms {
                // Keep this seam fixture on the soft opacity edge, not saturated overcast.
                cloud_params: Vec4::new(0.65, 0.0, 0.0, 0.0),
                ..cloud_test_uniforms()
            },
            cloud_texture: texture,
        });
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(build_sky_dome_mesh(900.0, 32));
    app.world_mut()
        .spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform::default()));
}

fn cloud_test_uniforms() -> SkyUniforms {
    SkyUniforms {
        sky_top: Vec4::ZERO,
        sky_middle: Vec4::ZERO,
        sky_band1: Vec4::ZERO,
        sky_band2: Vec4::ZERO,
        sky_smog: Vec4::ZERO,
        sun_color: Vec4::ZERO,
        sun_halo_color: Vec4::ZERO,
        cloud_emissive_color: Vec4::ZERO,
        cloud_layer1_ambient_color: Vec4::ONE,
        cloud_layer2_ambient_color: Vec4::ONE,
        sun_direction: Vec4::Y,
        cloud_params: Vec4::X,
    }
}

#[test]
#[ignore = "requires GPU; run explicitly with --ignored --test-threads=1"]
fn cloud_sampling_wraps_longitude_without_seam() {
    let mut app = render_app();
    let targets = [
        spawn_seam_camera(&mut app, -1.0, 0),
        spawn_seam_camera(&mut app, 1.0, 1),
    ];
    spawn_periodic_test_sky(&mut app);
    let (sender, receiver) = mpsc::channel();
    let mut pending = [false; 2];
    let mut pixels = [None; 2];
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline && pixels.iter().any(Option::is_none) {
        app.update();
        for (index, target) in targets.iter().enumerate() {
            if pending[index] || pixels[index].is_some() {
                continue;
            }
            let sender = sender.clone();
            app.world_mut()
                .spawn(Screenshot::image(target.clone()))
                .observe(move |capture: On<ScreenshotCaptured>| {
                    sender
                        .send((index, capture.image.clone()))
                        .expect("test receiver exists");
                });
            pending[index] = true;
        }
        for (index, image) in receiver.try_iter() {
            pending[index] = false;
            let rgba = image
                .try_into_dynamic()
                .expect("readable sky capture")
                .to_rgba8();
            let pixel = rgba.get_pixel(16, 16).0;
            let cloud_visible = pixel[0] > 20
                && pixel[0] < 240
                && pixel[0].abs_diff(pixel[1]) <= 1
                && pixel[1].abs_diff(pixel[2]) <= 1;
            if cloud_visible {
                pixels[index] = Some(pixel);
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let [Some(left), Some(right)] = pixels else {
        panic!("sky shader did not produce both cloud samples within deadline: {pixels:?}");
    };
    println!("Longitude seam pixels: {left:?} / {right:?}");
    // The finite angular separation permits a small smooth gradient, not a tile jump.
    assert!(
        left[0].abs_diff(right[0]) <= 4,
        "opposite sides of longitude wrap must agree, got {left:?} vs {right:?}"
    );
}

const DENSITY_IMAGE_SIZE: u32 = 160;

fn spawn_generated_density_sky(app: &mut App) -> ([Handle<Image>; 3], Handle<Image>) {
    let image = super::cloud_texture::generate_procedural_cloud_image(0);
    let texture = app.world_mut().resource_mut::<Assets<Image>>().add(image);
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(build_sky_dome_mesh(900.0, 32));
    let densities = [0.0, 0.5, 1.0];
    let targets = std::array::from_fn(|index| {
        spawn_density_case(app, &texture, &mesh, densities[index], index)
    });
    (targets, texture)
}

fn spawn_density_case(
    app: &mut App,
    texture: &Handle<Image>,
    mesh: &Handle<Mesh>,
    density: f32,
    index: usize,
) -> Handle<Image> {
    let mut uniforms = cloud_test_uniforms();
    uniforms.cloud_params.x = density;
    let material = app
        .world_mut()
        .resource_mut::<Assets<SkyMaterial>>()
        .add(SkyMaterial {
            uniforms,
            cloud_texture: texture.clone(),
        });
    app.world_mut().spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(material),
        Transform::default(),
        RenderLayers::layer(index),
    ));
    spawn_density_camera(app, index)
}

fn spawn_density_camera(app: &mut App, index: usize) -> Handle<Image> {
    let target = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new_target_texture(
            DENSITY_IMAGE_SIZE,
            DENSITY_IMAGE_SIZE,
            TextureFormat::Rgba8UnormSrgb,
            None,
        ));
    app.world_mut().spawn((
        Camera3d::default(),
        Camera {
            order: index as isize,
            clear_color: Color::srgb(1.0, 0.0, 1.0).into(),
            ..default()
        },
        Projection::Perspective(PerspectiveProjection {
            fov: std::f32::consts::FRAC_PI_2,
            ..default()
        }),
        RenderTarget::Image(target.clone().into()),
        Transform::default().looking_at(Vec3::new(0.0, 0.5, -0.8660254), Vec3::Y),
        RenderLayers::layer(index),
        Msaa::Off,
        Tonemapping::None,
    ));
    target
}

#[derive(Debug)]
struct CloudCoverage {
    bright: usize,
    dark: usize,
    total: usize,
    maximum: u8,
}

fn measure_cloud_coverage(image: Image) -> Option<CloudCoverage> {
    let rgba = image
        .try_into_dynamic()
        .expect("readable density capture")
        .to_rgba8();
    let mut coverage = CloudCoverage {
        bright: 0,
        dark: 0,
        total: 0,
        maximum: 0,
    };
    // Central rays remain roughly 16–44 degrees above the horizon, below the pole mask.
    for y in 60..100 {
        for x in 40..120 {
            let pixel = rgba.get_pixel(x, y).0;
            if pixel[0].abs_diff(pixel[1]) > 1 || pixel[1].abs_diff(pixel[2]) > 1 {
                return None; // Magenta clear pixels mean the sky draw is not ready.
            }
            coverage.bright += usize::from(pixel[0] > 100);
            coverage.dark += usize::from(pixel[0] < 30);
            coverage.total += 1;
            coverage.maximum = coverage.maximum.max(pixel[0]);
        }
    }
    Some(coverage)
}

fn density_sky_pipeline_ready(app: &App, shader: &Handle<Shader>) -> bool {
    let cache = app.sub_app(RenderApp).world().resource::<PipelineCache>();
    for pipeline in cache.pipelines() {
        let PipelineDescriptor::RenderPipelineDescriptor(descriptor) = &pipeline.descriptor else {
            continue;
        };
        if !descriptor
            .fragment
            .as_ref()
            .is_some_and(|fragment| fragment.shader == *shader)
        {
            continue;
        }
        match &pipeline.state {
            CachedPipelineState::Ok(_) => return true,
            CachedPipelineState::Err(
                ShaderCacheError::ShaderNotLoaded(_)
                | ShaderCacheError::ShaderImportNotYetAvailable,
            ) => {}
            CachedPipelineState::Err(error) => panic!("density sky pipeline failed: {error:?}"),
            _ => {}
        }
    }
    false
}

#[test]
#[ignore = "requires GPU; run explicitly with --ignored --test-threads=1"]
fn generated_clouds_have_visible_density_control() {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut app = render_app();
    let (targets, texture) = spawn_generated_density_sky(&mut app);
    let shader = app
        .world()
        .resource::<AssetServer>()
        .load("shaders/sky.wgsl");
    let (sender, receiver) = mpsc::channel();
    let mut pending = [false; 3];
    let mut coverage: [Option<CloudCoverage>; 3] = std::array::from_fn(|_| None);
    while Instant::now() < deadline && coverage.iter().any(Option::is_none) {
        app.update();
        let texture_ready = app
            .sub_app(RenderApp)
            .world()
            .resource::<RenderAssets<GpuImage>>()
            .get(&texture)
            .is_some();
        if !texture_ready || !density_sky_pipeline_ready(&app, &shader) {
            continue;
        }
        for (index, target) in targets.iter().enumerate() {
            if pending[index] || coverage[index].is_some() {
                continue;
            }
            let sender = sender.clone();
            app.world_mut()
                .spawn(Screenshot::image(target.clone()))
                .observe(move |capture: On<ScreenshotCaptured>| {
                    sender
                        .send((index, capture.image.clone()))
                        .expect("test receiver exists");
                });
            pending[index] = true;
        }
        for (index, image) in receiver.try_iter() {
            pending[index] = false;
            coverage[index] = measure_cloud_coverage(image);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let [Some(clear), Some(middle), Some(full)] = coverage else {
        panic!("generated cloud density captures not ready within 10s: {coverage:?}");
    };
    for (density, sample) in [(0.0, &clear), (0.5, &middle), (1.0, &full)] {
        println!(
            "density={density}: bright={}/{} ({:.2}%), dark={}/{} ({:.2}%), max={}",
            sample.bright,
            sample.total,
            100.0 * sample.bright as f32 / sample.total as f32,
            sample.dark,
            sample.total,
            100.0 * sample.dark as f32 / sample.total as f32,
            sample.maximum
        );
    }
    assert!(
        clear.maximum <= 1,
        "zero density must render black: {clear:?}"
    );
    assert!(
        middle.bright * 10 >= middle.total,
        "density 0.5 must show at least 10% bright cloud pixels (>100 sRGB): {middle:?}"
    );
    assert!(
        middle.dark * 10 >= middle.total,
        "density 0.5 must retain at least 10% dark clear pixels (<30 sRGB): {middle:?}"
    );
    assert!(
        full.bright >= middle.bright + middle.total / 5,
        "density 1 must add at least 20 percentage points of cloud coverage: middle={middle:?}, full={full:?}"
    );
}
