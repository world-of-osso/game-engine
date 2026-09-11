//! Pixel regression for finite authored sky geometry in front of scene objects.
use super::{SkyboxM2Material, SkyboxM2MaterialPlugin, SkyboxM2Settings};
use bevy::camera::{RenderTarget, visibility::RenderLayers};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::render::RenderApp;
use bevy::render::render_resource::{CachedPipelineState, PipelineCache, TextureFormat};
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const TARGET_SIZE: u32 = 64;
const SAMPLE_POSITIONS: [(u32, u32); 4] = [(4, 4), (16, 32), (40, 32), (50, 32)];

fn create_scene(app: &mut App, layer: usize, blend_mode: u16) -> Handle<Image> {
    let target = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new_target_texture(
            TARGET_SIZE,
            TARGET_SIZE,
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
        Projection::Perspective(PerspectiveProjection {
            fov: std::f32::consts::FRAC_PI_2,
            ..default()
        }),
        RenderTarget::Image(target.clone().into()),
        Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        RenderLayers::layer(layer),
        Msaa::Off,
        Tonemapping::None,
    ));
    spawn_sky(app, layer, blend_mode);
    spawn_foreground(app, layer);
    target
}

fn sky_material(texture: Handle<Image>, blend_mode: u16) -> SkyboxM2Material {
    SkyboxM2Material {
        settings: sky_settings(blend_mode),
        base_texture: texture.clone(),
        second_texture: texture.clone(),
        third_texture: texture.clone(),
        fourth_texture: texture,
        blend_mode,
        two_sided: true,
        priority_plane: 0,
        material_layer: 0,
        default_sequence_index: 0,
        global_sequences: Vec::new(),
        transparency_anim: None,
        color_opacity_anim: None,
        texture_anim_1: None,
        texture_anim_2: None,
    }
}

fn sky_settings(blend_mode: u16) -> SkyboxM2Settings {
    SkyboxM2Settings {
        color: Vec4::ONE,
        transparency: 1.0,
        alpha_test: 0.0,
        combine_mode: 0,
        blend_mode: u32::from(blend_mode),
        uv_mode_1: 0,
        uv_mode_2: 0,
        uv_mode_3: 0,
        uv_mode_4: 0,
        render_flags: 0,
        has_second_texture: 0,
        has_third_texture: 0,
        has_fourth_texture: 0,
        uv_offset_1: Vec2::ZERO,
        uv_offset_2: Vec2::ZERO,
    }
}

fn spawn_sky(app: &mut App, layer: usize, blend_mode: u16) {
    let texture = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(crate::rgba_image(vec![0, 0, 255, 255], 1, 1));
    let material = app
        .world_mut()
        .resource_mut::<Assets<SkyboxM2Material>>()
        .add(sky_material(texture, blend_mode));
    let mut mesh = Mesh::from(Rectangle::new(8.0, 8.0));
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_1,
        vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    );
    let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh);
    // Deliberately closer to the camera than both foreground cards at z=0.
    app.world_mut().spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, 0.0, 2.0),
        RenderLayers::layer(layer),
    ));
}

fn spawn_foreground(app: &mut App, layer: usize) {
    spawn_card(
        app,
        layer,
        -2.5,
        2.0,
        StandardMaterial {
            base_color: Color::srgb(1.0, 0.0, 0.0),
            unlit: true,
            ..default()
        },
    );
    let mut image = crate::rgba_image(vec![0, 255, 0, 255, 0, 255, 0, 0], 2, 1);
    image.sampler = bevy::image::ImageSampler::nearest();
    let texture = app.world_mut().resource_mut::<Assets<Image>>().add(image);
    spawn_card(
        app,
        layer,
        2.0,
        3.0,
        StandardMaterial {
            base_color_texture: Some(texture),
            alpha_mode: AlphaMode::Mask(0.5),
            unlit: true,
            ..default()
        },
    );
}

fn spawn_card(app: &mut App, layer: usize, x: f32, width: f32, material: StandardMaterial) {
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Rectangle::new(width, 3.0));
    let material = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(material);
    app.world_mut().spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(x, 0.0, 0.0),
        RenderLayers::layer(layer),
    ));
}

fn assert_no_pipeline_errors(app: &App) {
    let cache = app.sub_app(RenderApp).world().resource::<PipelineCache>();
    for pipeline in cache.pipelines() {
        if let CachedPipelineState::Err(error) = &pipeline.state {
            match error {
                bevy::shader::ShaderCacheError::ShaderNotLoaded(_)
                | bevy::shader::ShaderCacheError::ShaderImportNotYetAvailable => {}
                _ => panic!("skybox depth pipeline failed: {error}\n{error:#?}"),
            }
        }
    }
}

fn expected_pixels(pixels: &[[u8; 4]; 4]) -> bool {
    let dominant_channels = [2, 0, 1, 2];
    pixels
        .iter()
        .zip(dominant_channels)
        .all(|(pixel, channel)| {
            pixel[channel] > 180
                && (0..3).all(|other| other == channel || pixel[other] < 40)
                && pixel[3] > 240
        })
}

#[test]
#[ignore = "requires GPU; run explicitly with --ignored --test-threads=1"]
fn skybox_depth_preserves_foreground_and_foliage_cutouts() {
    let deadline = Instant::now() + Duration::from_secs(10);
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
    app.add_plugins(SkyboxM2MaterialPlugin);
    app.finish();
    app.cleanup();
    // Separate cameras/layers exercise opaque and alpha-blended sky pipelines.
    let targets = [create_scene(&mut app, 0, 0), create_scene(&mut app, 1, 2)];
    let (sender, receiver) = mpsc::channel();
    let mut pending = [false; 2];
    let mut consecutive_matches = [0_u8; 2];
    let mut last_pixels = [[[0_u8; 4]; 4]; 2];
    while Instant::now() < deadline {
        app.update();
        assert_no_pipeline_errors(&app);
        for (index, target) in targets.iter().enumerate() {
            if pending[index] {
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
                .expect("readable GPU image")
                .to_rgba8();
            let pixels = SAMPLE_POSITIONS.map(|(x, y)| rgba.get_pixel(x, y).0);
            last_pixels[index] = pixels;
            consecutive_matches[index] = if expected_pixels(&pixels) {
                consecutive_matches[index] + 1
            } else {
                0
            };
        }
        if consecutive_matches.iter().all(|count| *count >= 2) {
            return;
        }
    }
    panic!(
        "skybox must preserve [blue background, red foreground, green foliage, blue cutout]; \
         opaque/blended pixels={last_pixels:?}, consecutive matches={consecutive_matches:?}"
    );
}
