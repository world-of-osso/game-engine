use super::*;
use bevy::ecs::system::SystemId;
use ui_toolkit::{event::EventBus, registry::FrameRegistry};

fn test_world() -> (World, SystemId) {
    let mut world = World::new();
    world.insert_resource(UiState {
        registry: FrameRegistry::new(1920.0, 1080.0),
        event_bus: EventBus::new(),
        focused_frame: None,
    });
    world.init_resource::<AchievementCompletionState>();
    world.init_resource::<AchievementFrameOpen>();
    world.init_resource::<ButtonInput<KeyCode>>();
    let build = world.register_system(build_achievement_frame_ui);
    world.run_system(build).unwrap();
    let sync = world.register_system(sync_achievement_frame_state);
    world.run_system(sync).unwrap();
    (world, sync)
}

fn published_state(world: &World) -> &AchievementFrameState {
    world
        .resource::<AchievementFrameWrap>()
        .0
        .shared
        .get::<AchievementFrameState>()
        .unwrap()
}

fn assert_visible(world: &World, visible: bool) {
    let registry = &world.resource::<UiState>().registry;
    let id = registry.get_by_name("AchievementFrame").unwrap();
    assert_eq!(registry.get(id).unwrap().hidden, !visible);
    assert_eq!(published_state(world).visible, visible);
}

#[test]
fn registered_sync_preserves_initial_and_idle_state() {
    let (mut world, sync) = test_world();
    let initial = published_state(&world).clone();
    assert_visible(&world, false);
    assert_eq!(initial.total_points, 0);
    assert_eq!(initial.categories[0].name, "General");
    assert!(initial.categories[0].selected);
    assert_eq!(initial.achievements[0].name, "Level 10");
    assert_eq!(initial.achievements[0].progress_text, "0 / 10");
    assert!(!initial.achievements[0].completed);

    for _ in 0..8 {
        world.run_system(sync).unwrap();
    }
    assert_eq!(published_state(&world), &initial);
    assert_eq!(world.resource::<AchievementFrameModel>().0, initial);
    assert_visible(&world, false);
}

#[test]
fn keyboard_toggle_changes_open_only_on_a_key_event() {
    let (mut world, sync) = test_world();
    let toggle = world.register_system(toggle_achievement_frame);
    world.clear_trackers();
    world.run_system(toggle).unwrap();
    assert!(!world.resource_ref::<AchievementFrameOpen>().is_changed());

    world
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyY);
    world.run_system(toggle).unwrap();
    assert!(world.resource_ref::<AchievementFrameOpen>().is_changed());
    world.run_system(sync).unwrap();
    assert_visible(&world, true);

    world.resource_mut::<ButtonInput<KeyCode>>().clear();
    world.clear_trackers();
    world.run_system(toggle).unwrap();
    assert!(!world.resource_ref::<AchievementFrameOpen>().is_changed());
    world.run_system(sync).unwrap();
    assert_visible(&world, true);

    let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
    keys.release(KeyCode::KeyY);
    keys.press(KeyCode::KeyY);
    world.run_system(toggle).unwrap();
    world.run_system(sync).unwrap();
    assert_visible(&world, false);
}

#[test]
fn closed_progress_and_completion_publish_before_opening() {
    let (mut world, sync) = test_world();
    world
        .resource_mut::<AchievementCompletionState>()
        .progress
        .insert(1, (2, 5));
    world.run_system(sync).unwrap();
    assert_visible(&world, false);
    let state = published_state(&world);
    assert_eq!(state.achievements[0].progress, 0.4);
    assert_eq!(state.achievements[0].progress_text, "2 / 5");
    assert!(!state.achievements[0].completed);

    {
        let mut completion = world.resource_mut::<AchievementCompletionState>();
        completion.earned.insert(1);
        completion.progress.insert(1, (5, 5));
    }
    world.run_system(sync).unwrap();
    assert_visible(&world, false);
    assert_eq!(published_state(&world).total_points, 10);
    assert!(published_state(&world).achievements[0].completed);
    assert_eq!(published_state(&world).achievements[0].progress, 1.0);

    world.resource_mut::<AchievementFrameOpen>().0 = true;
    world.run_system(sync).unwrap();
    assert_visible(&world, true);
    assert_eq!(published_state(&world).total_points, 10);
    assert_eq!(
        published_state(&world).achievements[0].progress_text,
        "5 / 5"
    );
}

#[test]
fn reset_and_reentry_initialize_even_after_sync_without_a_view() {
    let (mut world, sync) = test_world();
    world
        .resource_mut::<AchievementCompletionState>()
        .earned
        .insert(1);
    world.resource_mut::<AchievementFrameOpen>().0 = true;
    world.run_system(sync).unwrap();
    assert_eq!(published_state(&world).total_points, 10);

    let teardown = world.register_system(teardown_achievement_frame_ui);
    let build = world.register_system(build_achievement_frame_ui);
    world.run_system(teardown).unwrap();
    assert!(!world.contains_resource::<AchievementFrameModel>());
    assert!(!world.contains_resource::<AchievementFrameWrap>());
    assert!(
        world
            .resource::<UiState>()
            .registry
            .get_by_name("AchievementFrame")
            .is_none()
    );
    world.insert_resource(AchievementCompletionState::default());
    world.insert_resource(AchievementFrameOpen::default());
    world.run_system(sync).unwrap();
    world.run_system(build).unwrap();
    world.run_system(sync).unwrap();
    assert_visible(&world, false);
    assert_eq!(published_state(&world).total_points, 0);
    assert!(!published_state(&world).achievements[0].completed);

    // Recreate the screen again without changing either input resource.
    world.run_system(teardown).unwrap();
    world.run_system(sync).unwrap();
    world.run_system(build).unwrap();
    world.run_system(sync).unwrap();
    assert_visible(&world, false);
    assert_eq!(
        published_state(&world).achievements[0].progress_text,
        "0 / 10"
    );
}

#[test]
#[ignore = "manual settled-system timing characterization; no performance threshold"]
fn characterize_cached_idle_sync() {
    use std::time::Instant;

    const ITERATIONS: u32 = 250_000;
    let (mut world, sync) = test_world();
    let initial = published_state(&world).clone();
    for _ in 0..1_000 {
        world.run_system(sync).unwrap();
    }
    let mut samples = Vec::new();
    for _ in 0..7 {
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            world.run_system(sync).unwrap();
        }
        samples.push(start.elapsed().as_nanos() as f64 / f64::from(ITERATIONS));
        assert_eq!(published_state(&world), &initial);
    }
    assert_visible(&world, false);
    println!("ACHIEVEMENT_IDLE_NS_PER_SYNC iterations={ITERATIONS} samples={samples:?}");
}
