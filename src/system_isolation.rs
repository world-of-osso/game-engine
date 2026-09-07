//! Exact, timed callback removal for stationary client diagnostics.

use std::collections::VecDeque;
use std::time::Duration;

use bevy::ecs::schedule::{
    InternedScheduleLabel, InternedSystemSet, ScheduleCleanupPolicy, Schedules,
};
use bevy::prelude::*;
use bevy::render::{ExtractSchedule, MainWorld, Render, RenderApp, RenderSystems};

mod args;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum SystemWorld {
    Main,
    Render,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RemovalRequest {
    world: SystemWorld,
    schedule: String,
    system: String,
    deadline: Duration,
}

#[derive(Resource, Default)]
struct PendingRemovals {
    before_render: VecDeque<RemovalRequest>,
    after_extract: VecDeque<RemovalRequest>,
    elapsed: Duration,
}

impl PendingRemovals {
    fn new(requests: Vec<RemovalRequest>) -> Self {
        let (after_extract, before_render) = requests.into_iter().partition(|request| {
            request.world == SystemWorld::Render && request.schedule == "ExtractSchedule"
        });
        Self {
            before_render,
            after_extract,
            elapsed: Duration::ZERO,
        }
    }
}

pub(crate) fn configure(app: &mut App, arguments: &[String]) {
    let requests = args::parse_requests(arguments)
        .unwrap_or_else(|error| panic!("invalid system isolation request: {error}"));
    if requests.is_empty() {
        return;
    }
    let pending = PendingRemovals::new(requests);
    let needs_after_extract = !pending.after_extract.is_empty();
    let render_app = app
        .get_sub_app_mut(RenderApp)
        .expect("system isolation requires the render app");
    render_app.insert_resource(pending);
    render_app.add_systems(ExtractSchedule, apply_main_and_render_removals);
    if needs_after_extract {
        render_app.add_systems(
            Render,
            apply_extract_removals.in_set(RenderSystems::Cleanup),
        );
    }
}

#[derive(Clone, Copy)]
enum RemovalPhase {
    BeforeRender,
    AfterExtract,
}

fn apply_main_and_render_removals(world: &mut World) {
    let Some(pending) = world.get_resource::<PendingRemovals>() else {
        return;
    };
    if pending.before_render.is_empty() && pending.after_extract.is_empty() {
        return;
    }
    let elapsed = world
        .resource::<MainWorld>()
        .resource::<Time<Real>>()
        .elapsed();
    world.resource_mut::<PendingRemovals>().elapsed = elapsed;
    apply_due_requests(world, RemovalPhase::BeforeRender, elapsed);
}

fn apply_extract_removals(world: &mut World) {
    let Some(pending) = world.get_resource::<PendingRemovals>() else {
        return;
    };
    let elapsed = pending.elapsed;
    apply_due_requests(world, RemovalPhase::AfterExtract, elapsed);
}

fn take_due_request(
    world: &mut World,
    phase: RemovalPhase,
    elapsed: Duration,
) -> Option<RemovalRequest> {
    let mut pending = world.get_resource_mut::<PendingRemovals>()?;
    let queue = match phase {
        RemovalPhase::BeforeRender => &mut pending.before_render,
        RemovalPhase::AfterExtract => &mut pending.after_extract,
    };
    if queue.front()?.deadline > elapsed {
        return None;
    }
    queue.pop_front()
}

fn apply_due_requests(world: &mut World, phase: RemovalPhase, elapsed: Duration) {
    while let Some(request) = take_due_request(world, phase, elapsed) {
        execute_removal(world, &request).unwrap_or_else(|error| {
            panic!(
                "system isolation failed for {:?}:{} {:?}: {error}",
                request.world, request.schedule, request.system
            )
        });
        info!(
            "System removed at {:.3}s: world={:?} schedule={} name={}",
            elapsed.as_secs_f64(),
            request.world,
            request.schedule,
            request.system
        );
    }
}

fn execute_removal(world: &mut World, request: &RemovalRequest) -> Result<(), String> {
    match request.world {
        SystemWorld::Render => remove_named_system(world, &request.schedule, &request.system),
        SystemWorld::Main => {
            if !world.contains_resource::<MainWorld>() {
                return Err("main world is unavailable outside extraction".into());
            }
            world.resource_scope(|_world, mut main: Mut<MainWorld>| {
                remove_named_system(&mut main, &request.schedule, &request.system)
            })
        }
    }
}

fn remove_named_system(
    world: &mut World,
    schedule_name: &str,
    system_name: &str,
) -> Result<(), String> {
    let label = find_schedule(world, schedule_name)?;
    world.schedule_scope(label, |world, schedule| {
        schedule
            .initialize(world)
            .map_err(|error| format!("schedule initialization failed: {error:?}"))?;
        let target = find_single_system_set(schedule, system_name)?;
        let removed = schedule
            .remove_systems_in_set(target, world, ScheduleCleanupPolicy::RemoveSystemsOnly)
            .map_err(|error| format!("system removal failed: {error:?}"))?;
        if removed != 1 {
            return Err(format!("expected one removed callback, got {removed}"));
        }
        Ok(())
    })
}

fn find_schedule(world: &World, name: &str) -> Result<InternedScheduleLabel, String> {
    let schedules = world
        .get_resource::<Schedules>()
        .ok_or("world has no schedule registry")?;
    let mut matches = schedules
        .iter()
        .filter(|(label, _)| format!("{label:?}") == name)
        .map(|(_, schedule)| schedule.label());
    let label = matches
        .next()
        .ok_or_else(|| format!("schedule {name:?} not found"))?;
    if matches.next().is_some() {
        return Err(format!("schedule name {name:?} is ambiguous"));
    }
    Ok(label)
}

fn find_single_system_set(schedule: &Schedule, name: &str) -> Result<InternedSystemSet, String> {
    let mut matches = schedule
        .systems()
        .map_err(|error| format!("schedule is not initialized: {error:?}"))?
        .filter(|(_, system)| system.name().to_string() == name);
    let (_, system) = matches
        .next()
        .ok_or_else(|| format!("system {name:?} not found"))?;
    if matches.next().is_some() {
        return Err(format!("system name {name:?} is ambiguous"));
    }
    let sets = system.default_system_sets();
    let [target] = sets.as_slice() else {
        return Err(format!("expected one implicit type set for {name:?}"));
    };
    let members = schedule
        .systems()
        .map_err(|error| format!("schedule is not initialized: {error:?}"))?
        .filter(|(_, system)| system.default_system_sets().contains(target))
        .count();
    if members != 1 {
        return Err(format!(
            "type set for {name:?} identifies {members} callbacks"
        ));
    }
    Ok(*target)
}

#[cfg(test)]
#[path = "../tests/unit/system_isolation_tests.rs"]
mod tests;
