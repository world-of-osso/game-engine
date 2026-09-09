use super::super::m2_material;
use crate::asset::m2::M2RenderBatch;
use bevy::camera::RenderTarget;
use bevy::core_pipeline::prepass::{DepthPrepass, NormalPrepass};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Exercise the actual multisampled PBR depth/normal prepass, not material enums.
#[test]
#[ignore = "requires a GPU; run explicitly with --ignored --test-threads=1"]
fn foliage_cutouts_do_not_occlude_background_in_depth_prepass() {
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
    app.finish();
    app.cleanup();
    let target = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new_target_texture(
            128,
            128,
            TextureFormat::Rgba8UnormSrgb,
            None,
        ));
    app.world_mut().spawn((
        Camera3d::default(),
        Camera {
            clear_color: Color::BLACK.into(),
            ..default()
        },
        RenderTarget::Image(target.clone().into()),
        Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        Msaa::Sample4,
        DepthPrepass,
        NormalPrepass,
        Tonemapping::None,
    ));
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Rectangle::new(4.0, 4.0));
    let background = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color: Color::srgb(0.0, 1.0, 0.0),
            unlit: true,
            ..default()
        });
    app.world_mut().spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(background),
        Transform::default(),
    ));
    // Left half is fully transparent; right half is opaque red. Both must retain
    // the same geometry/depth ordering, exposing discarded-fragment depth writes.
    let texture = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(crate::rgba_image(vec![255, 0, 0, 0, 255, 0, 0, 255], 2, 1));
    let batch = foliage_batch();
    let foreground = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(m2_material(Some(texture), None, &batch));
    app.world_mut().spawn((
        Mesh3d(mesh),
        MeshMaterial3d(foreground),
        Transform::from_xyz(0.0, 0.0, 0.5),
    ));

    let (sender, receiver) = mpsc::channel();
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut frames = 0;
    let mut capture_pending = false;
    while Instant::now() < deadline {
        app.update();
        frames += 1;
        if frames >= 30 && !capture_pending {
            let sender = sender.clone();
            app.world_mut()
                .spawn(Screenshot::image(target.clone()))
                .observe(move |capture: On<ScreenshotCaptured>| {
                    sender
                        .send(capture.image.clone())
                        .expect("test receiver exists");
                });
            capture_pending = true;
        }
        if let Ok(image) = receiver.try_recv() {
            let pixels = image
                .try_into_dynamic()
                .expect("readable screenshot")
                .to_rgba8();
            let opaque = pixels.get_pixel(84, 64).0;
            // Wait for material pipelines before crediting either pixel result.
            if opaque[0] < 180 || opaque[1] > 40 {
                capture_pending = false;
                continue;
            }
            let transparent = pixels.get_pixel(44, 64).0;
            println!(
                "foliage GPU pixels: transparent={transparent:?}, opaque={opaque:?}, frames={frames}"
            );
            assert!(
                transparent[1] > 180 && transparent[0] < 40,
                "alpha-zero foliage must reveal green background, got {transparent:?}"
            );
            return;
        }
    }
    panic!("GPU fixture did not produce a ready screenshot within 10 seconds ({frames} frames)");
}

fn foliage_batch() -> M2RenderBatch {
    M2RenderBatch {
        mesh: Mesh::from(Rectangle::new(4.0, 4.0)),
        texture_fdid: None,
        texture_2_fdid: None,
        extra_texture_fdids: vec![],
        texture_type: None,
        overlays: vec![],
        render_flags: 0x05,
        blend_mode: 1,
        transparency: 1.0,
        transparency_track_index: None,
        color_opacity_track_index: None,
        transparency_anim: None,
        color_opacity_anim: None,
        texture_anim: None,
        texture_anim_2: None,
        use_uv_2_1: false,
        use_uv_2_2: false,
        use_env_map_2: false,
        shader_id: 0,
        texture_count: 1,
        uses_texture_combiner_combos: false,
        priority_plane: 0,
        material_layer: 0,
        mesh_part_id: 0,
    }
}
