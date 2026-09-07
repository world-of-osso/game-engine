use super::*;
use bevy::ecs::schedule::Schedules;
use bevy::ecs::system::IntoSystem;
use bevy::render::{Extract, MainWorld};

#[derive(Resource, Default)]
struct Counts {
    target: usize,
    unrelated: usize,
}

#[derive(Resource, Default)]
struct Payloads(Vec<u32>);

#[derive(SystemSet, Debug, Clone, Eq, PartialEq, Hash)]
struct SharedGroup;

fn target(mut counts: ResMut<Counts>) {
    counts.target += 1;
}

fn unrelated(mut counts: ResMut<Counts>) {
    counts.unrelated += 1;
}

fn move_camera(mut cameras: Query<&mut Transform>) {
    for mut camera in &mut cameras {
        camera.translation.x += 1.0;
    }
}

fn send_payloads(mut payloads: ResMut<Payloads>, mut counts: ResMut<Counts>) {
    counts.target += payloads.0.drain(..).count();
}

fn extract_first(main: Extract<Res<Counts>>, mut counts: ResMut<Counts>) {
    counts.target += main.target + 1;
}

fn extract_second(main: Extract<Res<Counts>>, mut counts: ResMut<Counts>) {
    counts.target += main.target + 2;
}

fn request(world: SystemWorld, schedule: &str, system: &str, seconds: u64) -> RemovalRequest {
    RemovalRequest {
        world,
        schedule: schedule.into(),
        system: system.into(),
        deadline: Duration::from_secs(seconds),
    }
}

fn render_world(seconds: u64, requests: Vec<RemovalRequest>) -> World {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    let mut world = World::new();
    let mut main = MainWorld::default();
    let mut time = Time::<Real>::default();
    time.advance_by(Duration::from_secs(seconds));
    main.insert_resource(time);
    main.init_resource::<Counts>();
    main.init_resource::<Payloads>();
    main.init_resource::<Schedules>();
    main.resource_mut::<Schedules>()
        .insert(Schedule::new(Update));
    main.resource_mut::<Schedules>()
        .insert(Schedule::new(PostUpdate));
    world.insert_resource(main);
    world.init_resource::<Counts>();
    world.init_resource::<Schedules>();
    world
        .resource_mut::<Schedules>()
        .insert(Schedule::new(Render));
    world.insert_resource(PendingRemovals::new(requests));
    world
}

fn run_main_and_render(world: &mut World) {
    world.resource_mut::<MainWorld>().run_schedule(Update);
    world.run_schedule(Render);
}

#[test]
fn separate_deadlines_remove_one_main_then_one_render_callback() {
    let mut world = render_world(
        19,
        vec![
            request(SystemWorld::Main, "Update", "main-target", 20),
            request(SystemWorld::Render, "Render", "render-target", 22),
        ],
    );
    world
        .resource_mut::<MainWorld>()
        .schedule_scope(Update, |_world, schedule| {
            schedule.add_systems(
                (
                    IntoSystem::into_system(target).with_name("main-target"),
                    unrelated,
                )
                    .chain(),
            );
        });
    world.schedule_scope(Render, |_world, schedule| {
        schedule.add_systems(
            (
                IntoSystem::into_system(target).with_name("render-target"),
                unrelated,
            )
                .chain(),
        );
    });
    run_main_and_render(&mut world);
    apply_main_and_render_removals(&mut world);
    run_main_and_render(&mut world);
    assert_eq!(world.resource::<MainWorld>().resource::<Counts>().target, 2);
    assert_eq!(world.resource::<Counts>().target, 2);

    world
        .resource_mut::<MainWorld>()
        .resource_mut::<Time<Real>>()
        .advance_by(Duration::from_secs(1));
    apply_main_and_render_removals(&mut world);
    run_main_and_render(&mut world);
    assert_eq!(world.resource::<MainWorld>().resource::<Counts>().target, 2);
    assert_eq!(world.resource::<Counts>().target, 3);

    world
        .resource_mut::<MainWorld>()
        .resource_mut::<Time<Real>>()
        .advance_by(Duration::from_secs(2));
    apply_main_and_render_removals(&mut world);
    run_main_and_render(&mut world);
    apply_main_and_render_removals(&mut world);
    run_main_and_render(&mut world);
    assert_eq!(world.resource::<MainWorld>().resource::<Counts>().target, 2);
    assert_eq!(world.resource::<Counts>().target, 3);
    assert_eq!(
        world.resource::<MainWorld>().resource::<Counts>().unrelated,
        5
    );
    assert_eq!(world.resource::<Counts>().unrelated, 5);
}

#[test]
fn removal_retains_camera_pose_and_queued_payloads_without_removing_shared_group() {
    let mut world = render_world(
        19,
        vec![
            request(SystemWorld::Main, "Update", "camera", 20),
            request(SystemWorld::Main, "PostUpdate", "sender", 20),
        ],
    );
    let camera = world
        .resource_mut::<MainWorld>()
        .spawn(Transform::from_xyz(1.0, 2.0, 3.0))
        .id();
    world
        .resource_mut::<MainWorld>()
        .schedule_scope(Update, |_world, schedule| {
            schedule.add_systems(IntoSystem::into_system(move_camera).with_name("camera"));
        });
    world
        .resource_mut::<MainWorld>()
        .schedule_scope(PostUpdate, |_world, schedule| {
            schedule.add_systems(
                (
                    IntoSystem::into_system(send_payloads)
                        .with_name("sender")
                        .in_set(SharedGroup),
                    unrelated.in_set(SharedGroup),
                )
                    .chain(),
            );
        });
    world
        .resource_mut::<MainWorld>()
        .resource_mut::<Payloads>()
        .0
        .push(7);
    world.resource_mut::<MainWorld>().run_schedule(Update);
    world.resource_mut::<MainWorld>().run_schedule(PostUpdate);
    let pose = *world
        .resource::<MainWorld>()
        .get::<Transform>(camera)
        .unwrap();
    world
        .resource_mut::<MainWorld>()
        .resource_mut::<Payloads>()
        .0
        .push(42);
    world
        .resource_mut::<MainWorld>()
        .resource_mut::<Time<Real>>()
        .advance_by(Duration::from_secs(1));
    apply_main_and_render_removals(&mut world);
    world.resource_mut::<MainWorld>().run_schedule(Update);
    world.resource_mut::<MainWorld>().run_schedule(PostUpdate);
    let main = world.resource::<MainWorld>();
    assert_eq!(*main.get::<Transform>(camera).unwrap(), pose);
    assert_eq!(main.resource::<Payloads>().0, vec![42]);
    assert_eq!(main.resource::<Counts>().target, 1);
    assert_eq!(main.resource::<Counts>().unrelated, 2);
}

#[test]
fn extract_callbacks_are_removed_after_extraction_with_main_world_absent() {
    let mut world = render_world(
        20,
        vec![
            request(SystemWorld::Render, "ExtractSchedule", "extract-first", 20),
            request(SystemWorld::Render, "ExtractSchedule", "extract-second", 20),
        ],
    );
    let mut extract = Schedule::new(ExtractSchedule);
    extract.add_systems(
        (
            IntoSystem::into_system(extract_first).with_name("extract-first"),
            IntoSystem::into_system(extract_second).with_name("extract-second"),
            unrelated,
            apply_main_and_render_removals,
        )
            .chain(),
    );
    world.resource_mut::<Schedules>().insert(extract);
    world.run_schedule(ExtractSchedule);
    assert_eq!(world.resource::<Counts>().target, 3);
    let main = world.remove_resource::<MainWorld>().unwrap();
    apply_extract_removals(&mut world);
    world.insert_resource(main);
    world.run_schedule(ExtractSchedule);
    assert_eq!(world.resource::<Counts>().target, 3);
    assert_eq!(world.resource::<Counts>().unrelated, 2);
}

#[test]
fn ambiguous_names_and_shared_implicit_types_are_rejected_without_removal() {
    for same_type in [false, true] {
        let mut world = render_world(20, vec![]);
        world.schedule_scope(Render, |_world, schedule| {
            schedule.add_systems(IntoSystem::into_system(target).with_name("selected"));
            if same_type {
                schedule.add_systems(IntoSystem::into_system(target).with_name("other-alias"));
            } else {
                schedule.add_systems(IntoSystem::into_system(unrelated).with_name("selected"));
            }
        });
        assert!(remove_named_system(&mut world, "Render", "selected").is_err());
        world.run_schedule(Render);
        let counts = world.resource::<Counts>();
        assert_eq!(counts.target, if same_type { 2 } else { 1 });
        assert_eq!(counts.unrelated, if same_type { 0 } else { 1 });
    }
}

#[test]
fn missing_schedule_or_callback_is_an_explicit_error() {
    let mut world = render_world(20, vec![]);
    assert!(remove_named_system(&mut world, "NotASchedule", "missing").is_err());
    assert!(remove_named_system(&mut world, "Render", "missing").is_err());
}

#[test]
fn no_requests_do_not_require_a_render_app() {
    configure(&mut App::new(), &[]);
}

#[test]
fn selector_values_are_not_asset_paths() {
    let args = [
        "--remove-system-after",
        "main:Update",
        "system with spaces",
        "20",
        "model.m2",
    ]
    .map(str::to_owned);
    assert_eq!(
        crate::cli_args::parse_asset_path_from_args(&args),
        Some("model.m2".into())
    );
}
