use super::*;
use bevy::{ecs::schedule::Schedules, ecs::system::IntoSystem};
use lightyear::prelude::MessageSystems;

#[derive(Resource, Default)]
struct MessageProbe {
    pending_messages: usize,
    delivered_messages: usize,
    transport_runs: usize,
}

#[derive(Resource, Default)]
struct AmbiguousProbe {
    first_runs: usize,
    second_runs: usize,
}

fn sender_probe(mut probe: ResMut<MessageProbe>) {
    if probe.pending_messages > 0 {
        probe.pending_messages -= 1;
        probe.delivered_messages += 1;
    }
}

fn transport_probe(mut probe: ResMut<MessageProbe>) {
    probe.transport_runs += 1;
}

fn first_ambiguous_sender_probe(mut probe: ResMut<AmbiguousProbe>) {
    probe.first_runs += 1;
}

fn second_ambiguous_sender_probe(mut probe: ResMut<AmbiguousProbe>) {
    probe.second_runs += 1;
}

fn message_send_test_world(elapsed: u64) -> World {
    let mut world = World::new();
    let mut time = Time::<Real>::default();
    time.advance_by(Duration::from_secs(elapsed));
    world.insert_resource(time);
    world.insert_resource(SendFreezeDeadline(Duration::from_secs(20)));
    world.insert_resource(MessageProbe {
        pending_messages: 1,
        ..default()
    });

    let mut schedules = Schedules::default();
    let mut post_update = Schedule::new(PostUpdate);
    post_update.add_systems(
        (
            IntoSystem::into_system(sender_probe)
                .with_name("MessagePlugin::send")
                .in_set(MessageSystems::Send),
            transport_probe.in_set(MessageSystems::Send),
        )
            .chain(),
    );
    schedules.insert(post_update);
    world.insert_resource(schedules);
    world
}

#[test]
fn deadline_removes_only_message_sender_and_preserves_other_send_work() {
    let mut world = message_send_test_world(19);

    remove_due_message_send(&mut world);
    world.run_schedule(PostUpdate);
    assert_eq!(world.resource::<MessageProbe>().pending_messages, 0);
    assert_eq!(world.resource::<MessageProbe>().delivered_messages, 1);
    assert_eq!(world.resource::<MessageProbe>().transport_runs, 1);

    world
        .resource_mut::<Time<Real>>()
        .advance_by(Duration::from_secs(1));
    world.resource_mut::<MessageProbe>().pending_messages = 1;
    remove_due_message_send(&mut world);
    world.run_schedule(PostUpdate);
    assert_eq!(world.resource::<MessageProbe>().pending_messages, 1);
    assert_eq!(world.resource::<MessageProbe>().delivered_messages, 1);
    assert_eq!(world.resource::<MessageProbe>().transport_runs, 2);

    remove_due_message_send(&mut world);
    world.run_schedule(PostUpdate);
    assert_eq!(world.resource::<MessageProbe>().pending_messages, 1);
    assert_eq!(world.resource::<MessageProbe>().delivered_messages, 1);
    assert_eq!(world.resource::<MessageProbe>().transport_runs, 3);
}

#[test]
fn absent_flag_leaves_message_send_configuration_unchanged() {
    let mut app = App::new();
    configure(&mut app, &[]);
    assert!(!app.world().contains_resource::<SendFreezeDeadline>());
}

#[test]
fn missing_or_invalid_message_send_seconds_panics() {
    for args in [
        vec!["--freeze-message-send-after".to_owned()],
        vec!["--freeze-message-send-after".to_owned(), "bad".to_owned()],
    ] {
        assert!(std::panic::catch_unwind(|| configure(&mut App::new(), &args)).is_err());
    }
}

#[test]
fn message_send_deadline_flag_does_not_become_an_asset_path() {
    let args = ["--freeze-message-send-after".to_owned(), "20".to_owned()];
    let mut app = App::new();
    configure(&mut app, &args);

    assert_eq!(
        app.world().resource::<SendFreezeDeadline>().0,
        Duration::from_secs(20)
    );
    assert_eq!(crate::cli_args::parse_asset_path_from_args(&args), None);
}

#[test]
fn ambiguous_message_sender_name_panics_before_removing_either_system() {
    let mut world = World::new();
    let mut time = Time::<Real>::default();
    time.advance_by(Duration::from_secs(20));
    world.insert_resource(time);
    world.insert_resource(SendFreezeDeadline(Duration::from_secs(20)));
    world.init_resource::<AmbiguousProbe>();

    let mut schedules = Schedules::default();
    let mut post_update = Schedule::new(PostUpdate);
    post_update.add_systems((
        IntoSystem::into_system(first_ambiguous_sender_probe)
            .with_name("MessagePlugin::send")
            .in_set(MessageSystems::Send),
        IntoSystem::into_system(second_ambiguous_sender_probe)
            .with_name("MessagePlugin::send")
            .in_set(MessageSystems::Send),
    ));
    schedules.insert(post_update);
    world.insert_resource(schedules);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        remove_due_message_send(&mut world);
    }));
    assert!(
        result.is_err(),
        "ambiguous MessagePlugin::send systems must panic"
    );

    world.run_schedule(PostUpdate);
    assert_eq!(world.resource::<AmbiguousProbe>().first_runs, 1);
    assert_eq!(world.resource::<AmbiguousProbe>().second_runs, 1);
}
