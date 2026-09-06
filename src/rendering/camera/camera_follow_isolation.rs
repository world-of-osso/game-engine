//! Stationary-scene diagnostic: retain the camera pose and remove only follow work.

use std::time::Duration;

use bevy::ecs::schedule::{IntoSystemSet, ScheduleCleanupPolicy};
use bevy::prelude::*;

use super::camera_follow::camera_follow;

const FREEZE_FLAG: &str = "--freeze-camera-follow-after";

#[derive(Resource)]
struct FollowFreezeDeadline(Duration);

pub(crate) fn configure(app: &mut App, args: &[String]) {
    let Some(index) = args.iter().position(|arg| arg == FREEZE_FLAG) else {
        return;
    };
    let seconds = args
        .get(index + 1)
        .unwrap_or_else(|| panic!("{FREEZE_FLAG} requires seconds"))
        .parse::<u64>()
        .unwrap_or_else(|_| panic!("{FREEZE_FLAG} requires unsigned integer seconds"));
    app.insert_resource(FollowFreezeDeadline(Duration::from_secs(seconds)));
    app.add_systems(Last, freeze_camera_follow);
}

fn freeze_camera_follow(world: &mut World) {
    remove_due_follow(world, camera_follow);
}

fn remove_due_follow<M>(world: &mut World, target: impl IntoSystemSet<M>) {
    let Some(deadline) = world
        .get_resource::<FollowFreezeDeadline>()
        .map(|value| value.0)
    else {
        return;
    };
    let elapsed = world.resource::<Time<Real>>().elapsed();
    if elapsed < deadline {
        return;
    }
    let removed = world.schedule_scope(Update, |world, schedule| {
        schedule
            .remove_systems_in_set(target, world, ScheduleCleanupPolicy::RemoveSystemsOnly)
            .expect("failed to remove camera-follow system")
    });
    assert_eq!(removed, 1, "expected exactly one camera-follow system");
    world.remove_resource::<FollowFreezeDeadline>();
    info!(
        "Camera-follow system removed at {:.3}s; last camera transform retained",
        elapsed.as_secs_f64()
    );
}

#[cfg(test)]
#[path = "../../../tests/unit/camera_follow_isolation_tests.rs"]
mod tests;
