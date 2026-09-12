//! Compile and render the actual M2 shader with both camera fog specializations.
use super::{M2EffectMaterial, M2EffectMaterialPlugin, M2EffectSettings};
use bevy::camera::RenderTarget;
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
        // Unlit output makes pixel readiness independent of lighting, while the
        // real fragment shader must still compile every referenced function.
        render_flags: 1,
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
    app.add_plugins(M2EffectMaterialPlugin);
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
            ready[index] = pixel[0] > 180 && pixel[1] < 40 && pixel[2] < 40;
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
