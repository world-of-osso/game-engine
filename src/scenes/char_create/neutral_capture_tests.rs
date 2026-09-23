//! Explicit GPU fixture for the catalog-supported neutral backdrop, not a selectable actor.
use super::*;
use bevy::camera::RenderTarget;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use bevy::window::PrimaryWindow;
use std::sync::mpsc;
use std::time::{Duration, Instant};

const OUTPUT: &str = "data/diagnostics/charcreate-authored-scenes-20260923/neutral-backdrop.webp";
const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

fn spawn_neutral_backdrop(app: &mut App, target: Handle<Image>) {
    app.world_mut()
        .run_system_once(
            move |mut spawn: CharCreateSpawnParams, mut displayed: ResMut<DisplayedModels>| {
                let fdid = spawn
                    .scene_catalog
                    .lookup(24)
                    .expect("neutral scene mapping");
                assert_eq!(fdid, 623716, "capture must use authored neutral scene");
                spawn_lighting(&mut spawn.commands);
                ensure_sky_env_map(&mut spawn.commands, &mut spawn.images);
                let backdrop =
                    background::spawn(&mut CharCreateSpawnContext::from_params(&mut spawn), fdid)
                        .expect("neutral backdrop loader");
                let camera = spawn_camera(&mut spawn.commands, backdrop.framing);
                spawn
                    .commands
                    .entity(camera)
                    .insert((RenderTarget::Image(target.into()), Msaa::Off));
                displayed.background = Some(backdrop);
            },
        )
        .expect("spawn backdrop through production system parameters");
    app.update();
    app.world_mut()
        .run_system_once(sync_scene_projection)
        .expect("apply production window-aspect projection");
}

fn assert_backdrop_only(app: &mut App) {
    let world = app.world_mut();
    let backdrop = world
        .resource::<DisplayedModels>()
        .background
        .as_ref()
        .expect("neutral backdrop");
    assert_eq!(backdrop.fdid, 623716);
    assert!(world.get_entity(backdrop.root).is_ok());
    assert_eq!(world.query::<&CharCreateModelRoot>().iter(world).count(), 0);
    let renderable = world
        .query::<(Entity, &Mesh3d)>()
        .iter(world)
        .filter(|(entity, mesh)| {
            world.resource::<Assets<Mesh>>().get(&mesh.0).is_some()
                && (world
                    .get::<MeshMaterial3d<StandardMaterial>>(*entity)
                    .is_some()
                    || world
                        .get::<MeshMaterial3d<M2EffectMaterial>>(*entity)
                        .is_some())
        })
        .count();
    assert!(
        renderable > 20,
        "neutral backdrop needs actual materialized M2 batches"
    );
    assert!(world.query::<&PointLight>().iter(world).count() > 0);
}

fn save_rendered_backdrop(image: &Image) -> bool {
    assert_eq!(image.size(), UVec2::new(WIDTH, HEIGHT));
    let pixels = game_engine::screenshot::rgba_bytes(image).expect("GPU screenshot pixels");
    let distinct = pixels
        .chunks_exact(4)
        .step_by(64)
        .map(|pixel| [pixel[0], pixel[1], pixel[2]])
        .collect::<std::collections::HashSet<_>>()
        .len();
    if distinct < 64 {
        return false;
    }
    let webp =
        game_engine::screenshot::encode_webp(image, game_engine::screenshot::DEFAULT_WEBP_QUALITY)
            .expect("encode captured GPU image");
    let output = std::path::Path::new(OUTPUT);
    std::fs::create_dir_all(output.parent().expect("diagnostic directory"))
        .expect("create diagnostic directory");
    std::fs::write(output, webp).expect("write neutral GPU capture");
    true
}

#[test]
#[ignore = "requires GPU and local CASC assets; run explicitly with --ignored --test-threads=1"]
fn capture_neutral_loader_backdrop_without_character_actor() {
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
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.insert_state(crate::game_state::GameState::CharCreate);
    app.add_plugins((
        crate::m2_effect_material::M2EffectMaterialPlugin,
        crate::animation::AnimationPlugin,
        crate::particle::ParticlePlugin,
    ));
    app.insert_resource(CreationSceneCatalog::load(Path::new("data/ChrRaces.csv")).unwrap());
    app.insert_resource(creature_display::CreatureDisplayMap);
    app.init_resource::<Assets<SkinnedMeshInverseBindposes>>();
    app.init_resource::<DisplayedModels>();
    app.finish();
    app.cleanup();

    app.world_mut().spawn((
        Window {
            resolution: (WIDTH, HEIGHT).into(),
            ..default()
        },
        PrimaryWindow,
    ));
    let target = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new_target_texture(
            WIDTH,
            HEIGHT,
            TextureFormat::Rgba8UnormSrgb,
            None,
        ));
    spawn_neutral_backdrop(&mut app, target.clone());
    assert_backdrop_only(&mut app);

    let (sender, receiver) = mpsc::channel();
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut pending = false;
    let mut frames = 0;
    while Instant::now() < deadline && frames < 300 {
        app.update();
        frames += 1;
        if let Ok(image) = receiver.try_recv() {
            pending = false;
            if save_rendered_backdrop(&image) {
                return;
            }
        }
        if frames >= 8 && !pending {
            let sender = sender.clone();
            app.world_mut()
                .spawn(Screenshot::image(target.clone()))
                .observe(move |capture: On<ScreenshotCaptured>| {
                    sender
                        .send(capture.image.clone())
                        .expect("capture receiver");
                });
            pending = true;
        }
    }
    panic!(
        "neutral authored backdrop never produced a non-flat GPU screenshot within {frames} frames"
    );
}
