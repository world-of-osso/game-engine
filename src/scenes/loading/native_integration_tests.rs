use super::*;
use bevy::asset::{AssetApp, AssetPlugin};
use bevy::camera::{CameraPlugin, ComputedCameraValues, RenderTargetInfo};
use bevy::ecs::system::RunSystemOnce;
use bevy::image::ImagePlugin;
use bevy::math::Affine2;
use bevy::text::TextPlugin;
use game_engine::ui::native::NativeUiElement;

fn fixture_ui() -> UiState {
    UiState {
        registry: game_engine::ui::registry::FrameRegistry::new(1280.0, 720.0),
        event_bus: game_engine::ui::event::EventBus::new(),
        focused_frame: None,
    }
}

fn loading_app(width: u32, height: u32) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        ImagePlugin::default(),
        bevy::mesh::MeshPlugin,
        bevy::window::WindowPlugin {
            primary_window: None,
            exit_condition: bevy::window::ExitCondition::DontExit,
            ..default()
        },
        bevy::input::InputPlugin,
        bevy::transform::TransformPlugin,
        CameraPlugin,
        TextPlugin,
        bevy::picking::DefaultPickingPlugins,
        bevy::ui::UiPlugin,
        bevy::state::app::StatesPlugin,
    ));
    app.init_asset::<TextureAtlasLayout>()
        .init_resource::<ui_toolkit::font_registry::FontRegistry>()
        .init_resource::<CurrentZone>()
        .init_resource::<AdtManager>()
        .insert_resource(fixture_ui())
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::ZERO,
        ))
        .insert_state(GameState::Loading)
        .add_plugins(LoadingScreenPlugin)
        .add_systems(
            PreStartup,
            |view: Res<LoadingView>, cameras: Query<(), With<UiCamera>>| {
                assert_ne!(view.root, view.camera);
                assert!(
                    cameras.is_empty(),
                    "initial loading enters before toolkit camera startup"
                );
            },
        )
        .add_systems(Startup, ui_toolkit::render::setup_ui_camera);
    app.finish();
    app.cleanup();
    app.update();
    let camera = app.world().resource::<LoadingView>().camera;
    app.world_mut().get_mut::<Camera>(camera).unwrap().computed = ComputedCameraValues {
        target_info: Some(RenderTargetInfo {
            physical_size: UVec2::new(width, height),
            scale_factor: 1.0,
        }),
        ..default()
    };
    app.update();
    app.update();
    app
}

fn named(world: &mut World, name: &str) -> Entity {
    let matches: Vec<_> = world
        .query_filtered::<(Entity, &Name), With<NativeUiElement>>()
        .iter(world)
        .filter_map(|(entity, candidate)| (candidate.as_str() == name).then_some(entity))
        .collect();
    assert_eq!(matches.len(), 1, "expected exactly one native {name}");
    matches[0]
}

fn bounds(world: &mut World, name: &str) -> Rect {
    let entity = named(world, name);
    Rect::from_center_size(
        Affine2::from(world.get::<UiGlobalTransform>(entity).unwrap()).translation,
        world.get::<ComputedNode>(entity).unwrap().size,
    )
}

fn near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 1.0,
        "expected {expected}, got {actual}"
    );
}

fn progress_state(percent: u8) -> LoadingScreenState {
    LoadingScreenState {
        status_text: "Loading terrain...".into(),
        zone_text: "Entering Elwynn Forest".into(),
        tip_text: DEFAULT_TIP_TEXT.into(),
        progress_percent: percent,
    }
}

fn apply_view(app: &mut App, state: &LoadingScreenState, layout: &LoadingScreenLayout) {
    let view = app.world().resource::<LoadingView>().clone();
    sync_loading_view(app.world_mut(), &view, state, layout);
    app.update();
}

#[test]
fn loading_native_layout_and_progress_match_at_two_viewports() {
    for (width, height) in [(1280, 720), (1920, 1080)] {
        let mut app = loading_app(width, height);
        let layout = LoadingScreenLayout::default();
        apply_view(&mut app, &progress_state(50), &layout);
        let world = app.world_mut();
        let root = bounds(world, "LoadingRoot");
        near(root.min.x, 0.0);
        near(root.min.y, 0.0);
        near(root.width(), width as f32);
        near(root.height(), height as f32);
        let art = bounds(world, "LoadingArtwork");
        near(art.center().x, width as f32 / 2.0);
        near(art.center().y, height as f32 / 2.0 + 50.0);
        near(art.width(), layout.art_width);
        near(art.height(), layout.art_height);
        let bar = bounds(world, "LoadingBarBackground");
        near(bar.width(), layout.bar_width);
        near(bar.max.y, art.max.y - layout.bar_y);
        let clip = bounds(world, "LoadingBarFillClip");
        let fill = bounds(world, "LoadingBarFill");
        near(clip.min.x, bar.min.x + layout.bar_fill_start_x);
        near(clip.width(), layout.bar_fill_max_width);
        near(fill.min.x, clip.min.x);
        near(fill.width(), layout.bar_fill_max_width / 2.0);
        near(fill.height(), layout.bar_fill_height);
        let progress = named(world, "LoadingProgressText");
        assert_eq!(world.get::<Text>(progress).unwrap().0, "50%");
        let status = named(world, "LoadingStatusText");
        assert_eq!(world.get::<Text>(status).unwrap().0, "Loading terrain...");
    }
}

#[test]
fn loading_layout_changes_update_existing_entities() {
    let mut app = loading_app(1280, 720);
    let fill = named(app.world_mut(), "LoadingBarFill");
    let root = named(app.world_mut(), "LoadingRoot");
    let layout = LoadingScreenLayout {
        bar_width: 700.0,
        bar_fill_start_x: 10.0,
        bar_fill_max_width: 680.0,
        bar_y: -30.0,
        ..default()
    };
    apply_view(&mut app, &progress_state(75), &layout);
    assert_eq!(named(app.world_mut(), "LoadingBarFill"), fill);
    assert_eq!(named(app.world_mut(), "LoadingRoot"), root);
    let bar = bounds(app.world_mut(), "LoadingBarBackground");
    let fill_rect = bounds(app.world_mut(), "LoadingBarFill");
    near(bar.width(), 700.0);
    near(fill_rect.min.x, bar.min.x + 10.0);
    near(fill_rect.width(), 510.0);
}

#[test]
fn loading_plugin_startup_and_exit_preserve_other_entities_and_camera() {
    let mut app = loading_app(1280, 720);
    let world = app.world_mut();
    let view = world.resource::<LoadingView>().clone();
    assert_eq!(world.get::<Camera>(view.camera).unwrap().order, 1);
    let (toolkit, camera) = world
        .query_filtered::<(Entity, &Camera), With<UiCamera>>()
        .single(world)
        .unwrap();
    assert_eq!(camera.order, 2);
    assert!(
        world
            .resource::<UiState>()
            .registry
            .get_by_name("LoadingRoot")
            .is_none()
    );
    let unrelated = world.spawn(Name::new("OtherScreen")).id();
    world
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Login);
    app.update();
    let world = app.world_mut();
    assert!(world.get_entity(view.root).is_err());
    assert!(world.get_entity(view.camera).is_err());
    assert!(world.get_entity(unrelated).is_ok());
    assert_eq!(world.get::<Camera>(toolkit).unwrap().order, 1);
    assert!(!world.contains_resource::<LoadingView>());
    assert!(!world.contains_resource::<LoadingSnapshot>());
    assert!(!world.contains_resource::<LoadingProgressAnimation>());
    assert!(!world.contains_resource::<LoadingCameraOrders>());
    assert_eq!(
        world
            .query_filtered::<Entity, With<NativeUiElement>>()
            .iter(world)
            .count(),
        0
    );
}

#[test]
fn loading_asset_failure_leaves_no_partial_view_or_camera_mutation() {
    let mut world = World::new();
    world.init_resource::<Assets<Font>>();
    world.init_resource::<ui_toolkit::font_registry::FontRegistry>();
    world.init_resource::<CurrentZone>();
    world.init_resource::<AdtManager>();
    let camera = world
        .spawn((
            UiCamera,
            Camera {
                order: 7,
                ..default()
            },
        ))
        .id();
    world
        .run_system_once(build_loading_ui)
        .expect("loading setup parameters available");
    assert_eq!(world.get::<Camera>(camera).unwrap().order, 7);
    assert!(!world.contains_resource::<LoadingView>());
    assert!(!world.contains_resource::<LoadingSnapshot>());
    assert!(!world.contains_resource::<LoadingProgressAnimation>());
    assert!(!world.contains_resource::<LoadingCameraOrders>());
    assert_eq!(world.query::<&Name>().iter(&world).count(), 0);
}
