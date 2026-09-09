//! Pixel proof for the real replicated-NPC observer and projected UI text.
use super::*;
use bevy::camera::RenderTarget;
use bevy::ecs::system::RunSystemOnce;
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
    app.init_state::<GameState>();
    app.insert_resource(State::new(GameState::InWorld));
    app.init_resource::<Assets<M2EffectMaterial>>();
    app.add_plugins(NameplatePlugin);
    app.finish();
    app.cleanup();
    app
}

fn cameras_and_wolf(app: &mut App) -> Handle<Image> {
    let target = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new_target_texture(
            256,
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
        Transform::from_xyz(-9000.0, NPC_NAMEPLATE_Y, 10.0)
            .looking_at(Vec3::new(-9000.0, NPC_NAMEPLATE_Y, 0.0), Vec3::Y),
        Msaa::Off,
    ));
    app.world_mut()
        .run_system_once(ui_toolkit::render::setup_ui_camera)
        .unwrap();
    let overlay = app
        .world_mut()
        .query_filtered::<Entity, With<ui_toolkit::render::UiCamera>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .entity_mut(overlay)
        .insert((RenderTarget::Image(target.clone().into()), Msaa::Off));
    app.world_mut().spawn((
        Npc {
            template_id: 299,
            name: "Diseased Young Wolf".into(),
        },
        Transform::from_xyz(-9000.0, 0.0, 0.0),
        Visibility::Visible,
    ));
    target
}

fn yellow_glyph_pixels(image: &Image) -> usize {
    let pixels = image.data.as_ref().expect("captured pixels");
    let width = image.width() as usize;
    pixels
        .chunks_exact(4)
        .enumerate()
        .filter(|(index, rgba)| {
            let x = index % width;
            let y = index / width;
            (12..244).contains(&x)
                && (44..84).contains(&y)
                && rgba[0] > 100
                && rgba[1] > 80
                && rgba[2] < 80
        })
        .count()
}

#[test]
#[ignore = "requires GPU; run explicitly with --ignored --test-threads=1"]
fn nameplate_gpu_world_observer_renders_glyphs_at_projected_anchor() {
    let mut app = render_app();
    let target = cameras_and_wolf(&mut app);
    let (sender, receiver) = mpsc::channel();
    let mut pending = false;
    let mut maximum_glyph_pixels = 0;
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        app.update();
        if !pending {
            let sender = sender.clone();
            app.world_mut()
                .spawn(Screenshot::image(target.clone()))
                .observe(move |capture: On<ScreenshotCaptured>| {
                    sender.send(capture.image.clone()).unwrap();
                });
            pending = true;
        }
        if let Ok(image) = receiver.try_recv() {
            pending = false;
            maximum_glyph_pixels = maximum_glyph_pixels.max(yellow_glyph_pixels(&image));
            if maximum_glyph_pixels > 100 {
                return;
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!(
        "world NPC nameplate produced only {maximum_glyph_pixels} yellow glyph pixels at its projected anchor"
    );
}
