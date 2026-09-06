use super::*;
use bevy::ecs::schedule::Schedules;

#[derive(Resource, Default)]
struct UnrelatedWork(usize);

fn follow_probe(mut transforms: Query<&mut Transform>) {
    for mut transform in &mut transforms {
        transform.translation.x += 1.0;
    }
}

fn unrelated_probe(mut work: ResMut<UnrelatedWork>) {
    work.0 += 1;
}

fn follow_test_world(elapsed: u64) -> (World, Entity) {
    let mut world = World::new();
    let mut time = Time::<Real>::default();
    time.advance_by(Duration::from_secs(elapsed));
    world.insert_resource(time);
    world.insert_resource(FollowFreezeDeadline(Duration::from_secs(20)));
    let camera = world.spawn(Transform::from_xyz(0.0, 3.0, 4.0)).id();
    world.init_resource::<UnrelatedWork>();
    let mut schedules = Schedules::default();
    let mut update = Schedule::new(Update);
    update.add_systems((follow_probe, unrelated_probe).chain());
    schedules.insert(update);
    world.insert_resource(schedules);
    (world, camera)
}

#[test]
fn due_removal_retains_transform_and_preserves_unrelated_chained_work() {
    let (mut world, camera) = follow_test_world(19);
    world.run_schedule(Update);
    remove_due_follow(&mut world, follow_probe);
    world.run_schedule(Update);
    assert_eq!(world.get::<Transform>(camera).unwrap().translation.x, 2.0);
    assert_eq!(world.resource::<UnrelatedWork>().0, 2);
    let retained = *world.get::<Transform>(camera).unwrap();

    world
        .resource_mut::<Time<Real>>()
        .advance_by(Duration::from_secs(1));
    remove_due_follow(&mut world, follow_probe);
    world.run_schedule(Update);
    assert_eq!(*world.get::<Transform>(camera).unwrap(), retained);
    assert_eq!(world.resource::<UnrelatedWork>().0, 3);

    remove_due_follow(&mut world, follow_probe);
    world.run_schedule(Update);
    assert_eq!(*world.get::<Transform>(camera).unwrap(), retained);
    assert_eq!(world.resource::<UnrelatedWork>().0, 4);
}

#[test]
fn absent_flag_leaves_follow_configuration_unchanged() {
    let mut app = App::new();
    configure(&mut app, &[]);
    assert!(!app.world().contains_resource::<FollowFreezeDeadline>());
}

#[test]
fn missing_or_invalid_follow_seconds_panics() {
    for args in [
        vec![FREEZE_FLAG.to_owned()],
        vec![FREEZE_FLAG.to_owned(), "bad".into()],
    ] {
        assert!(std::panic::catch_unwind(|| configure(&mut App::new(), &args)).is_err());
    }
}

#[test]
fn follow_deadline_is_configured_without_becoming_an_asset_path() {
    let args = [
        FREEZE_FLAG.to_owned(),
        "20".to_owned(),
        "model.m2".to_owned(),
    ];
    let mut app = App::new();
    configure(&mut app, &args);
    assert_eq!(
        app.world().resource::<FollowFreezeDeadline>().0,
        Duration::from_secs(20)
    );
    assert_eq!(
        crate::cli_args::parse_asset_path_from_args(&args),
        Some("model.m2".into())
    );
}

#[test]
#[should_panic(expected = "failed to remove camera-follow system")]
fn absent_target_fails_explicitly() {
    let mut world = World::new();
    let mut time = Time::<Real>::default();
    time.advance_by(Duration::from_secs(20));
    world.insert_resource(time);
    world.insert_resource(FollowFreezeDeadline(Duration::from_secs(20)));
    let mut schedules = Schedules::default();
    schedules.insert(Schedule::new(Update));
    world.insert_resource(schedules);
    remove_due_follow(&mut world, follow_probe);
}
