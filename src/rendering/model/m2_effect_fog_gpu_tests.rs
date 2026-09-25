//! Compile and render the actual M2 shader with both camera fog specializations.
use super::{M2EffectMaterial, M2EffectMaterialPlugin, M2EffectSettings};
use bevy::camera::{RenderTarget, visibility::RenderLayers};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::render::RenderApp;
use bevy::render::render_resource::{CachedPipelineState, PipelineCache, TextureFormat};
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use std::sync::mpsc;
use std::time::{Duration, Instant};

fn spawn_camera(app: &mut App, fog_enabled: bool, order: isize) -> Handle<Image> {
    let target = create_camera_target(app);
    let mut camera = app.world_mut().spawn((
        Camera3d::default(),
        Camera {
            order,
            clear_color: Color::BLACK.into(),
            ..default()
        },
        RenderTarget::Image(target.clone().into()),
        Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        Msaa::Off,
        Tonemapping::None,
    ));
    if fog_enabled {
        camera.insert(camera_fog());
    }
    target
}

fn create_camera_target(app: &mut App) -> Handle<Image> {
    app.world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new_target_texture(
            32,
            32,
            TextureFormat::Rgba8UnormSrgb,
            None,
        ))
}

fn camera_fog() -> DistanceFog {
    DistanceFog {
        color: Color::srgb(0.0, 0.0, 1.0),
        falloff: FogFalloff::Linear {
            start: 0.0,
            end: 10.0,
        },
        ..default()
    }
}

fn spawn_effect_quad(app: &mut App) {
    let material = create_effect_material(app);
    let mesh = create_effect_mesh(app);
    app.world_mut()
        .spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform::default()));
}

fn create_effect_material(app: &mut App) -> Handle<M2EffectMaterial> {
    let texture = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(crate::rgba_image(vec![255, 0, 0, 255], 1, 1));
    app.world_mut()
        .resource_mut::<Assets<M2EffectMaterial>>()
        .add(M2EffectMaterial {
            settings: unlit_effect_settings(),
            base_texture: texture.clone(),
            second_texture: texture,
            scene_light: crate::retail_light::RETAIL_SCENE_LIGHT_BUFFER,
            blend_mode: 0,
            two_sided: false,
            global_sequences: Vec::new(),
            texture_anim_1: None,
            texture_anim_2: None,
        })
}

fn unlit_effect_settings() -> M2EffectSettings {
    M2EffectSettings {
        transparency: 1.0,
        alpha_test: 0.0,
        shader_id: 0x0010,
        blend_mode: 0,
        uv_mode_1: 0,
        uv_mode_2: 0,
        // Unlit output makes the pixel independent of lighting, while the real
        // fragment shader must still compile every referenced function.
        render_flags: 1,
        gx_blend: 0,
        uv_offset_1: Vec2::ZERO,
        uv_offset_2: Vec2::ZERO,
    }
}

fn create_effect_mesh(app: &mut App) -> Handle<Mesh> {
    let mut mesh = Mesh::from(Rectangle::new(4.0, 4.0));
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_1,
        vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    );
    app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh)
}

fn assert_no_pipeline_errors(app: &App) {
    let cache = app.sub_app(RenderApp).world().resource::<PipelineCache>();
    for pipeline in cache.pipelines() {
        if let CachedPipelineState::Err(error) = &pipeline.state {
            match error {
                // Bevy retries asynchronous asset imports; the bounded pixel deadline
                // still fails if either shader variant never becomes available.
                bevy::shader::ShaderCacheError::ShaderNotLoaded(_)
                | bevy::shader::ShaderCacheError::ShaderImportNotYetAvailable => {}
                _ => panic!("actual M2 fog specialization failed: {error}\n{error:#?}"),
            }
        }
    }
}

#[test]
#[ignore = "requires GPU; run explicitly with --ignored --test-threads=1"]
fn m2_effect_shader_renders_with_distance_fog_disabled_and_enabled() {
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
    app.add_plugins((
        M2EffectMaterialPlugin,
        crate::retail_light::RetailLightingPlugin,
    ));
    app.insert_resource(crate::retail_light::RetailSceneLight::m2_scene(Vec3::ONE));
    app.finish();
    app.cleanup();
    let targets = [
        spawn_camera(&mut app, false, 0),
        spawn_camera(&mut app, true, 1),
    ];
    spawn_effect_quad(&mut app);
    let (sender, receiver) = mpsc::channel();
    let mut pending = [false; 2];
    let mut ready = [false; 2];
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        app.update();
        assert_no_pipeline_errors(&app);
        for (index, target) in targets.iter().enumerate() {
            if pending[index] || ready[index] {
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
            let pixel = rgba.get_pixel(16, 16).0;
            // Unlit batches still fog (only render flag 0x2 skips it): halfway
            // through the 0–10 yard fog, red and fog blue mix in authored space.
            let expected = if index == 1 {
                [128, 0, 128]
            } else {
                [255, 0, 0]
            };
            ready[index] = (0..3).all(|channel| pixel[channel].abs_diff(expected[channel]) <= 3);
            if ready[index] {
                println!("DISTANCE_FOG={}: rendered {pixel:?}", index == 1);
            }
        }
        if ready.iter().all(|ready| *ready) {
            return;
        }
    }
    panic!("M2 fog-off/fog-on pixels did not render within deadline: {ready:?}");
}

fn spawn_lit_comparison_camera(app: &mut App, layer: usize) -> Handle<Image> {
    let target = create_camera_target(app);
    app.world_mut().spawn((
        Camera3d::default(),
        Camera {
            order: layer as isize,
            clear_color: Color::BLACK.into(),
            ..default()
        },
        RenderTarget::Image(target.clone().into()),
        Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        Msaa::Off,
        Tonemapping::None,
        RenderLayers::layer(layer),
    ));
    target
}

fn lit_comparison_settings() -> M2EffectSettings {
    M2EffectSettings {
        transparency: 1.0,
        alpha_test: 0.0,
        shader_id: 0x0010,
        blend_mode: 2,
        uv_mode_1: 0,
        uv_mode_2: 0,
        // Bypass fog without enabling the unlit bit: exercise Retail lighting.
        render_flags: 2,
        gx_blend: 0,
        uv_offset_1: Vec2::ZERO,
        uv_offset_2: Vec2::ZERO,
    }
}

#[test]
#[ignore = "requires GPU; run explicitly with --ignored --test-threads=1"]
fn lit_m2_effect_matches_retail_m2_material_without_fog() {
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
    app.add_plugins((
        M2EffectMaterialPlugin,
        crate::retail_light::RetailLightingPlugin,
    ));
    // Grey ambient, sun from behind the camera: a lit, unsaturated white texel.
    app.insert_resource(crate::retail_light::RetailSceneLight {
        direct: Vec3::splat(0.3),
        sun_direction: Vec3::NEG_Z,
        ..crate::retail_light::RetailSceneLight::m2_scene(Vec3::splat(0.4))
    });
    app.finish();
    app.cleanup();
    let targets = [
        spawn_lit_comparison_camera(&mut app, 0),
        spawn_lit_comparison_camera(&mut app, 1),
    ];
    // Both paths share the same rectangle (+Z normals), texture and transform.
    let mesh = create_effect_mesh(&mut app);
    let texture = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(crate::rgba_image(vec![255, 255, 255, 255], 1, 1));
    let effect = app
        .world_mut()
        .resource_mut::<Assets<M2EffectMaterial>>()
        .add(M2EffectMaterial {
            settings: lit_comparison_settings(),
            base_texture: texture.clone(),
            second_texture: texture.clone(),
            scene_light: crate::retail_light::RETAIL_SCENE_LIGHT_BUFFER,
            blend_mode: 2,
            two_sided: false,
            global_sequences: Vec::new(),
            texture_anim_1: None,
            texture_anim_2: None,
        });
    let retail = app
        .world_mut()
        .resource_mut::<Assets<crate::retail_m2_material::M2Material>>()
        .add(crate::retail_m2_material::retail_m2_material(
            StandardMaterial {
                base_color_texture: Some(texture),
                alpha_mode: AlphaMode::Blend,
                ..default()
            },
            0x02,
            2,
        ));
    app.world_mut().spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(effect),
        Transform::IDENTITY,
        RenderLayers::layer(0),
    ));
    app.world_mut().spawn((
        Mesh3d(mesh),
        MeshMaterial3d(retail),
        Transform::IDENTITY,
        RenderLayers::layer(1),
    ));

    let capture = capture_lit_comparison(&mut app, &targets);
    assert_lit_comparison(&capture);
    // calcLight: white * (0.4 * 1.1 + 0.3) = 0.74 -> 189.
    for pixel in capture.pixels {
        assert!(
            pixel[0].abs_diff(189) <= 2,
            "Retail-lit white texel: {pixel:?}"
        );
    }
}

#[derive(Default)]
struct LitComparisonCapture {
    pending: [bool; 2],
    ready: [bool; 2],
    stable_frames: [u8; 2],
    pixels: [[u8; 4]; 2],
}

impl LitComparisonCapture {
    fn request_screenshots(
        &mut self,
        app: &mut App,
        targets: &[Handle<Image>; 2],
        sender: &mpsc::Sender<(usize, Image)>,
    ) {
        for (index, target) in targets.iter().enumerate() {
            if self.pending[index] || self.ready[index] {
                continue;
            }
            let sender = sender.clone();
            app.world_mut()
                .spawn(Screenshot::image(target.clone()))
                .observe(move |capture: On<ScreenshotCaptured>| {
                    sender
                        .send((index, capture.image.clone()))
                        .expect("lit comparison receiver exists");
                });
            self.pending[index] = true;
        }
    }

    fn receive_screenshots(&mut self, receiver: &mpsc::Receiver<(usize, Image)>) {
        for (index, image) in receiver.try_iter() {
            self.pending[index] = false;
            let rgba = image
                .try_into_dynamic()
                .expect("readable lit comparison image")
                .to_rgba8();
            let pixel = rgba.get_pixel(16, 16).0;
            let lit = is_lit_pixel(pixel);
            self.stable_frames[index] = if lit && pixel == self.pixels[index] {
                self.stable_frames[index] + 1
            } else {
                u8::from(lit)
            };
            self.pixels[index] = pixel;
            self.ready[index] = self.stable_frames[index] >= 2;
        }
    }
}

fn is_lit_pixel(pixel: [u8; 4]) -> bool {
    const MIN_LIT_CHANNEL: u8 = 8;
    const MAX_LIT_CHANNEL: u8 = 247;
    pixel[..3]
        .iter()
        .all(|channel| (MIN_LIT_CHANNEL..=MAX_LIT_CHANNEL).contains(channel))
}

fn capture_lit_comparison(app: &mut App, targets: &[Handle<Image>; 2]) -> LitComparisonCapture {
    let (sender, receiver) = mpsc::channel();
    let mut capture = LitComparisonCapture::default();
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        app.update();
        assert_no_pipeline_errors(app);
        capture.request_screenshots(app, targets, &sender);
        capture.receive_screenshots(&receiver);
        if capture.ready.iter().all(|ready| *ready) {
            break;
        }
    }
    capture
}

fn assert_lit_pixel(label: &str, pixel: [u8; 4]) {
    assert!(
        is_lit_pixel(pixel),
        "{label} must render nonblack, nonsaturated RGB: {pixel:?}"
    );
    assert_eq!(pixel[3], 255, "{label} must retain alpha 1: {pixel:?}");
}

fn assert_lit_comparison(capture: &LitComparisonCapture) {
    const RGB_TOLERANCE: u8 = 4;
    let pixels = capture.pixels;
    println!(
        "Lit comparison: M2EffectMaterial={:?}, M2Material={:?}",
        pixels[0], pixels[1]
    );
    for (label, pixel) in ["M2EffectMaterial", "M2Material"].into_iter().zip(pixels) {
        assert_lit_pixel(label, pixel);
    }
    assert!(
        capture.ready.iter().all(|ready| *ready),
        "lit comparison did not settle within ten seconds: {pixels:?}"
    );
    for channel in 0..3 {
        let delta = pixels[0][channel].abs_diff(pixels[1][channel]);
        assert!(
            delta <= RGB_TOLERANCE,
            "lit RGB channel {channel} differs by {delta}: effect={:?}, M2Material={:?}",
            pixels[0],
            pixels[1]
        );
    }
}
