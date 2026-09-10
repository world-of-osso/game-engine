//! Render the actual sky shader across the spherical longitude wrap.
use super::*;
use bevy::camera::RenderTarget;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
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
        let value = ((0.45 + 0.3 * phase.sin()) * 255.0).round() as u8;
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
            uniforms: cloud_test_uniforms(),
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
