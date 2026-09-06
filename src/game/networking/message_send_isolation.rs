//! Timed, connected-idle diagnostic for Lightyear's application-message sender.

use std::time::Duration;

use bevy::ecs::schedule::{InternedSystemSet, ScheduleCleanupPolicy};
use bevy::prelude::*;

const FREEZE_FLAG: &str = "--freeze-message-send-after";
const SEND_SYSTEM_NAME: &str = "MessagePlugin::send";

#[derive(Resource)]
struct SendFreezeDeadline(Duration);

pub(crate) fn configure(app: &mut App, args: &[String]) {
    let Some(index) = args.iter().position(|arg| arg == FREEZE_FLAG) else {
        return;
    };
    let seconds = args
        .get(index + 1)
        .unwrap_or_else(|| panic!("{FREEZE_FLAG} requires seconds"))
        .parse::<u64>()
        .unwrap_or_else(|_| panic!("{FREEZE_FLAG} requires unsigned integer seconds"));
    app.insert_resource(SendFreezeDeadline(Duration::from_secs(seconds)));
    app.add_systems(Last, remove_due_message_send);
}

fn remove_due_message_send(world: &mut World) {
    let Some(deadline) = world
        .get_resource::<SendFreezeDeadline>()
        .map(|value| value.0)
    else {
        return;
    };
    let elapsed = world.resource::<Time<Real>>().elapsed();
    if elapsed < deadline {
        return;
    }
    let removed = world.schedule_scope(PostUpdate, |world, schedule| {
        schedule
            .initialize(world)
            .map_err(|error| format!("failed to initialize PostUpdate: {error:?}"))?;
        let target = find_sender_type_set(schedule)?;
        schedule
            .remove_systems_in_set(target, world, ScheduleCleanupPolicy::RemoveSystemsOnly)
            .map_err(|error| format!("failed to remove sender: {error:?}"))
    });
    let removed = removed.unwrap_or_else(|error| panic!("message-send isolation failed: {error}"));
    assert_eq!(
        removed, 1,
        "expected exactly one application-message sender"
    );
    world.remove_resource::<SendFreezeDeadline>();
    info!(
        "Application-message send system removed at {:.3}s; receive and transport retained",
        elapsed.as_secs_f64()
    );
}

fn find_sender_type_set(schedule: &Schedule) -> Result<InternedSystemSet, String> {
    let mut matches = schedule
        .systems()
        .map_err(|error| error.to_string())?
        .filter(|(_, system)| system.name().to_string() == SEND_SYSTEM_NAME);
    let (_, system) = matches.next().ok_or("MessagePlugin::send not found")?;
    if matches.next().is_some() {
        return Err("MessagePlugin::send is ambiguous".into());
    }
    let sets = system.default_system_sets();
    let [target] = sets.as_slice() else {
        return Err("expected one sender type set".into());
    };
    let members = schedule
        .systems()
        .map_err(|error| error.to_string())?
        .filter(|(_, system)| system.default_system_sets().contains(target))
        .count();
    if members != 1 {
        return Err("sender type set identifies multiple callbacks".into());
    }
    Ok(*target)
}

#[cfg(test)]
#[path = "../../../tests/unit/message_send_isolation_tests.rs"]
mod tests;
