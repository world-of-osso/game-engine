//! Destructive, timed render-set removal for settled static-scene diagnostics.

use std::time::Duration;

use bevy::ecs::schedule::ScheduleCleanupPolicy;
use bevy::prelude::*;
use bevy::render::{ExtractSchedule, MainWorld, Render, RenderApp, RenderSystems};
use bevy::time::{Real, Time};

const FLAG: &str = "--remove-render-set-after";

#[derive(Resource, Clone)]
struct Cutoff {
    set: RenderSystems,
    deadline: Duration,
}

#[derive(Resource, Default)]
struct UpdateReport {
    updates: u64,
    last_report: Duration,
}

pub(crate) fn configure(app: &mut App, arguments: &[String]) {
    let Some(cutoff) = parse_cutoff(arguments) else {
        return;
    };
    app.get_sub_app_mut(RenderApp)
        .expect("render-set isolation requires RenderApp")
        .insert_resource(cutoff)
        .add_systems(ExtractSchedule, remove_set_at_cutoff);
    app.init_resource::<UpdateReport>()
        .add_systems(Last, report_updates);
}

fn parse_cutoff(arguments: &[String]) -> Option<Cutoff> {
    let mut matches = arguments.iter().enumerate().filter(|(_, arg)| *arg == FLAG);
    let (index, _) = matches.next()?;
    assert!(matches.next().is_none(), "{FLAG}: duplicate option");
    let name = arguments
        .get(index + 1)
        .expect("render-set cutoff: missing NAME");
    let set = match name.as_str() {
        "PrepareNonUi" => RenderSystems::Prepare,
        "PrepareAssets" => RenderSystems::PrepareAssets,
        "Specialize" => RenderSystems::Specialize,
        "Queue" => RenderSystems::Queue,
        _ => panic!(
            "{FLAG}: invalid set {name:?}; expected PrepareNonUi, PrepareAssets, Specialize, or Queue"
        ),
    };
    let value = arguments
        .get(index + 2)
        .expect("render-set cutoff: missing SECONDS");
    assert!(
        !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()),
        "{FLAG}: invalid unsigned SECONDS {value:?}"
    );
    let seconds = value
        .parse::<u64>()
        .unwrap_or_else(|error| panic!("{FLAG}: invalid SECONDS {value:?}: {error}"));
    Some(Cutoff {
        set,
        deadline: Duration::from_secs(seconds),
    })
}

fn report_updates(time: Res<Time<Real>>, mut report: ResMut<UpdateReport>) {
    report.updates += 1;
    let elapsed = time.elapsed();
    if elapsed.saturating_sub(report.last_report) >= Duration::from_secs(1) {
        eprintln!(
            "render set isolation elapsed_s={:.3} updates={}",
            elapsed.as_secs_f64(),
            report.updates
        );
        report.last_report = elapsed;
    }
}

fn remove_set_at_cutoff(world: &mut World) {
    let Some(cutoff) = world.get_resource::<Cutoff>() else {
        return;
    };
    let elapsed = world
        .resource::<MainWorld>()
        .resource::<Time<Real>>()
        .elapsed();
    if elapsed < cutoff.deadline {
        return;
    }
    let cutoff = cutoff.clone();
    let removed = remove_set(world, cutoff.set.clone());
    world.remove_resource::<Cutoff>();
    eprintln!(
        "render set isolation cutoff elapsed_s={:.3} set={:?} removed={} deadline_s={}",
        elapsed.as_secs_f64(),
        cutoff.set,
        removed,
        cutoff.deadline.as_secs()
    );
}

fn remove_set(world: &mut World, set: RenderSystems) -> usize {
    world.schedule_scope(Render, |world, schedule| {
        schedule
            .initialize(world)
            .unwrap_or_else(|error| panic!("render-set schedule initialization failed: {error:?}"));
        if set == RenderSystems::Prepare {
            return remove_non_ui_preparation(schedule, world);
        }
        let removed = schedule
            .remove_systems_in_set(set.clone(), world, ScheduleCleanupPolicy::RemoveSystemsOnly)
            .unwrap_or_else(|error| panic!("render-set removal {set:?} failed: {error:?}"));
        assert!(
            removed > 0,
            "render-set removal {set:?} matched zero systems"
        );
        removed
    })
}

fn collect_non_ui_preparation(
    schedule: &Schedule,
) -> Vec<(String, bevy::ecs::schedule::InternedSystemSet)> {
    let members = schedule
        .graph()
        .systems_in_set(RenderSystems::Prepare.intern())
        .expect("Prepare set must exist")
        .clone();
    schedule
        .systems()
        .expect("initialized schedule")
        .filter_map(|(key, system)| {
            let name = system.name().to_string();
            if !members.contains(&key) {
                return None;
            }
            // UI and sprite preparation also drain extracted per-frame data.
            // Removing that cleanup creates growing queues rather than isolating work.
            if name.contains("bevy_ui_render::") || name.contains("bevy_sprite_render::") {
                eprintln!("render set isolation preserved cleanup: {name}");
                return None;
            }
            let sets = system.default_system_sets();
            assert_eq!(sets.len(), 1, "expected one implicit set for {name}");
            Some((name, sets[0]))
        })
        .collect()
}

fn remove_non_ui_preparation(schedule: &mut Schedule, world: &mut World) -> usize {
    let targets = collect_non_ui_preparation(schedule);
    assert!(!targets.is_empty(), "PrepareNonUi matched zero systems");
    for (name, target) in &targets {
        let removed = schedule
            .remove_systems_in_set(*target, world, ScheduleCleanupPolicy::RemoveSystemsOnly)
            .unwrap_or_else(|error| panic!("preparation removal {name} failed: {error:?}"));
        assert_eq!(removed, 1, "ambiguous preparation callback {name}");
    }
    targets.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Resource, Default)]
    struct Counts {
        inside: usize,
        outside: usize,
    }

    #[test]
    fn removed_set_stops_its_systems_while_other_systems_continue() {
        let mut world = World::new();
        world.init_resource::<Counts>();
        let mut schedule = Schedule::new(Render);
        schedule.add_systems(
            (
                |mut counts: ResMut<Counts>| counts.inside += 1,
                |mut counts: ResMut<Counts>| counts.inside += 1,
            )
                .in_set(RenderSystems::Prepare),
        );
        schedule.add_systems(|mut counts: ResMut<Counts>| counts.outside += 1);
        world.add_schedule(schedule);
        world.run_schedule(Render);
        let counts = world.resource::<Counts>();
        assert_eq!((counts.inside, counts.outside), (2, 1));
        assert_eq!(remove_set(&mut world, RenderSystems::Prepare), 2);
        world.run_schedule(Render);
        world.run_schedule(Render);
        let counts = world.resource::<Counts>();
        assert_eq!((counts.inside, counts.outside), (2, 3));
    }

    #[test]
    fn parser_rejects_invalid_missing_and_duplicate_requests() {
        for values in [
            vec![FLAG],
            vec![FLAG, "Prepare"],
            vec![FLAG, "Cleanup", "1"],
            vec![FLAG, "PrepareNonUi", "-1"],
            vec![FLAG, "Prepare", "0"],
            vec![FLAG, "Queue", "1.5"],
            vec![FLAG, "Queue", "18446744073709551616"],
            vec![FLAG, "Queue", "0", FLAG, "Prepare", "1"],
        ] {
            let args = values.into_iter().map(str::to_owned).collect::<Vec<_>>();
            assert!(std::panic::catch_unwind(|| parse_cutoff(&args)).is_err());
        }
        assert!(parse_cutoff(&[]).is_none());
        for name in ["PrepareNonUi", "PrepareAssets", "Specialize", "Queue"] {
            let args = [FLAG, name, "0"].map(str::to_owned);
            assert_eq!(parse_cutoff(&args).unwrap().deadline, Duration::ZERO);
        }
    }
}
