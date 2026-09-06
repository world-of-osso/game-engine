use std::{collections::HashMap, time::Duration};

use bevy::ecs::schedule::{IntoSystemSet, ScheduleCleanupPolicy};
use bevy::pbr::MeshPipeline;
use bevy::prelude::*;
use bevy::render::batching::gpu_preprocessing::{
    write_batched_instance_buffers, write_indirect_parameters_buffers,
};
use bevy::render::{ExtractSchedule, MainWorld, Render, RenderApp};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum UploadTarget {
    IndirectParameters,
    BatchedInstances,
}

#[derive(Resource)]
struct FreezeDeadlines(HashMap<UploadTarget, Duration>);

pub(crate) fn configure(app: &mut App, args: &[String]) {
    let deadlines: HashMap<_, _> = [
        (
            UploadTarget::IndirectParameters,
            "--freeze-indirect-parameters-after",
        ),
        (
            UploadTarget::BatchedInstances,
            "--freeze-batched-instances-after",
        ),
    ]
    .into_iter()
    .filter_map(|(target, flag)| parse_deadline(args, flag).map(|delay| (target, delay)))
    .collect();
    if deadlines.is_empty() {
        return;
    }
    let render_app = app
        .get_sub_app_mut(RenderApp)
        .expect("render upload isolation requires the render app");
    render_app.insert_resource(FreezeDeadlines(deadlines));
    render_app.add_systems(ExtractSchedule, freeze_render_uploads);
}

fn parse_deadline(args: &[String], flag: &str) -> Option<Duration> {
    let index = args.iter().position(|arg| arg == flag)?;
    let value = args
        .get(index + 1)
        .unwrap_or_else(|| panic!("{flag} requires seconds"));
    let seconds = value
        .parse::<u64>()
        .unwrap_or_else(|_| panic!("{flag} requires unsigned integer seconds"));
    Some(Duration::from_secs(seconds))
}

fn freeze_render_uploads(world: &mut World) {
    remove_target_once(
        world,
        UploadTarget::IndirectParameters,
        write_indirect_parameters_buffers,
    );
    remove_target_once(
        world,
        UploadTarget::BatchedInstances,
        write_batched_instance_buffers::<MeshPipeline>,
    );
}

fn remove_target_once<M>(world: &mut World, kind: UploadTarget, target: impl IntoSystemSet<M>) {
    let deadline = world
        .get_resource::<FreezeDeadlines>()
        .and_then(|deadlines| deadlines.0.get(&kind))
        .copied();
    let Some(deadline) = deadline else {
        return;
    };
    let elapsed = world
        .resource::<MainWorld>()
        .resource::<Time<Real>>()
        .elapsed();
    if elapsed < deadline {
        return;
    }
    remove_render_system(world, target);
    world.resource_mut::<FreezeDeadlines>().0.remove(&kind);
    info!(
        "Render upload system removed at {:.3}s: {kind:?}",
        elapsed.as_secs_f64()
    );
}

fn remove_render_system<M>(world: &mut World, target: impl IntoSystemSet<M>) {
    let removed = world.schedule_scope(Render, |world, schedule| {
        schedule
            .remove_systems_in_set(target, world, ScheduleCleanupPolicy::RemoveSystemsOnly)
            .expect("failed to remove the render upload system")
    });
    assert_eq!(removed, 1, "expected exactly one render upload system");
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::schedule::Schedules;

    #[derive(Resource, Default)]
    struct WorkCounts {
        first: usize,
        second: usize,
        unrelated: usize,
    }

    fn first_work(mut counts: ResMut<WorkCounts>) {
        counts.first += 1;
    }

    fn second_work(mut counts: ResMut<WorkCounts>) {
        counts.second += 1;
    }

    fn unrelated_work(mut counts: ResMut<WorkCounts>) {
        counts.unrelated += 1;
    }

    #[test]
    fn removes_targets_at_separate_deadlines_and_preserves_other_work() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let mut world = World::new();
        let mut main_world = MainWorld::default();
        let mut time = Time::<Real>::default();
        time.advance_by(Duration::from_secs(19));
        main_world.insert_resource(time);
        world.insert_resource(main_world);
        world.insert_resource(FreezeDeadlines(HashMap::from([
            (UploadTarget::IndirectParameters, Duration::from_secs(20)),
            (UploadTarget::BatchedInstances, Duration::from_secs(22)),
        ])));
        world.init_resource::<WorkCounts>();
        let mut schedules = Schedules::default();
        let mut render = Schedule::new(Render);
        render.add_systems((first_work, second_work, unrelated_work));
        schedules.insert(render);
        world.insert_resource(schedules);

        world.run_schedule(Render);
        remove_target_once(&mut world, UploadTarget::IndirectParameters, first_work);
        world.run_schedule(Render);
        assert_eq!(world.resource::<WorkCounts>().first, 2);
        assert_eq!(world.resource::<WorkCounts>().second, 2);

        world
            .resource_mut::<MainWorld>()
            .resource_mut::<Time<Real>>()
            .advance_by(Duration::from_secs(1));
        remove_target_once(&mut world, UploadTarget::IndirectParameters, first_work);
        world.run_schedule(Render);
        assert_eq!(world.resource::<WorkCounts>().first, 2);
        assert_eq!(world.resource::<WorkCounts>().second, 3);

        remove_target_once(&mut world, UploadTarget::IndirectParameters, first_work);
        remove_target_once(&mut world, UploadTarget::BatchedInstances, second_work);
        world.run_schedule(Render);
        assert_eq!(world.resource::<WorkCounts>().first, 2);
        assert_eq!(world.resource::<WorkCounts>().second, 4);

        world
            .resource_mut::<MainWorld>()
            .resource_mut::<Time<Real>>()
            .advance_by(Duration::from_secs(2));
        remove_target_once(&mut world, UploadTarget::BatchedInstances, second_work);
        world.run_schedule(Render);
        assert_eq!(world.resource::<WorkCounts>().first, 2);
        assert_eq!(world.resource::<WorkCounts>().second, 4);
        assert_eq!(world.resource::<WorkCounts>().unrelated, 5);

        remove_target_once(&mut world, UploadTarget::BatchedInstances, second_work);
        world.run_schedule(Render);
        assert_eq!(world.resource::<WorkCounts>().second, 4);
        assert_eq!(world.resource::<WorkCounts>().unrelated, 6);
    }

    #[test]
    fn removes_two_targets_in_one_extract_before_render_rebuilds() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let mut world = World::new();
        let mut main_world = MainWorld::default();
        let mut time = Time::<Real>::default();
        time.advance_by(Duration::from_secs(20));
        main_world.insert_resource(time);
        world.insert_resource(main_world);
        world.insert_resource(FreezeDeadlines(HashMap::from([
            (UploadTarget::IndirectParameters, Duration::from_secs(20)),
            (UploadTarget::BatchedInstances, Duration::from_secs(20)),
        ])));
        world.init_resource::<WorkCounts>();
        let mut schedules = Schedules::default();
        let mut render = Schedule::new(Render);
        render.add_systems((first_work, second_work, unrelated_work));
        schedules.insert(render);
        world.insert_resource(schedules);
        world.run_schedule(Render);

        remove_target_once(&mut world, UploadTarget::IndirectParameters, first_work);
        remove_target_once(&mut world, UploadTarget::BatchedInstances, second_work);
        world.run_schedule(Render);
        assert_eq!(world.resource::<WorkCounts>().first, 1);
        assert_eq!(world.resource::<WorkCounts>().second, 1);
        assert_eq!(world.resource::<WorkCounts>().unrelated, 2);
    }

    #[test]
    fn absent_flag_does_not_require_or_change_a_render_app() {
        configure(&mut App::new(), &[]);
        assert_eq!(
            parse_deadline(&[], "--freeze-indirect-parameters-after"),
            None
        );
        assert_eq!(
            parse_deadline(
                &["--freeze-indirect-parameters-after".into(), "20".into()],
                "--freeze-indirect-parameters-after"
            ),
            Some(Duration::from_secs(20))
        );
        assert_eq!(
            parse_deadline(
                &["--freeze-batched-instances-after".into(), "25".into()],
                "--freeze-batched-instances-after"
            ),
            Some(Duration::from_secs(25))
        );
        let args = [
            "--freeze-indirect-parameters-after".into(),
            "20".into(),
            "--freeze-batched-instances-after".into(),
            "25".into(),
            "model.m2".into(),
        ];
        assert_eq!(
            crate::cli_args::parse_asset_path_from_args(&args),
            Some(std::path::PathBuf::from("model.m2"))
        );
    }

    #[test]
    #[should_panic(expected = "requires seconds")]
    fn rejects_missing_deadline() {
        parse_deadline(
            &["--freeze-indirect-parameters-after".into()],
            "--freeze-indirect-parameters-after",
        );
    }

    #[test]
    #[should_panic(expected = "requires unsigned integer seconds")]
    fn rejects_invalid_deadline() {
        parse_deadline(
            &["--freeze-indirect-parameters-after".into(), "-1".into()],
            "--freeze-indirect-parameters-after",
        );
    }
}
