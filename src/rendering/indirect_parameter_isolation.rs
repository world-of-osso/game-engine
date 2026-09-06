use std::time::Duration;

use bevy::ecs::schedule::{IntoSystemSet, ScheduleCleanupPolicy, Schedules};
use bevy::prelude::*;
use bevy::render::batching::gpu_preprocessing::write_indirect_parameters_buffers;
use bevy::render::{ExtractSchedule, MainWorld, Render, RenderApp};

#[derive(Resource, Clone, Copy)]
struct FreezeDeadline(Duration);

pub(crate) fn configure(app: &mut App, args: &[String]) {
    let Some(deadline) = parse_deadline(args) else {
        return;
    };
    let render_app = app
        .get_sub_app_mut(RenderApp)
        .expect("--freeze-indirect-parameters-after requires the render app");
    render_app.insert_resource(FreezeDeadline(deadline));
    render_app.add_systems(ExtractSchedule, freeze_indirect_parameters);
}

fn parse_deadline(args: &[String]) -> Option<Duration> {
    let index = args
        .iter()
        .position(|arg| arg == "--freeze-indirect-parameters-after")?;
    let value = args
        .get(index + 1)
        .expect("--freeze-indirect-parameters-after requires seconds");
    let seconds = value
        .parse::<u64>()
        .expect("--freeze-indirect-parameters-after requires unsigned integer seconds");
    Some(Duration::from_secs(seconds))
}

fn freeze_indirect_parameters(world: &mut World) {
    if remove_target_once(world, write_indirect_parameters_buffers) {
        let elapsed = world
            .resource::<MainWorld>()
            .resource::<Time<Real>>()
            .elapsed_secs_f64();
        info!(
            "Indirect parameter uploads frozen at {elapsed:.3}s: removed write_indirect_parameters_buffers"
        );
    }
}

fn remove_target_once<M>(world: &mut World, target: impl IntoSystemSet<M>) -> bool {
    let Some(deadline) = world.get_resource::<FreezeDeadline>().copied() else {
        return false;
    };
    let elapsed = world
        .resource::<MainWorld>()
        .resource::<Time<Real>>()
        .elapsed();
    if elapsed < deadline.0 {
        return false;
    }
    remove_render_system(world, target);
    world.remove_resource::<FreezeDeadline>();
    true
}

fn remove_render_system<M>(world: &mut World, target: impl IntoSystemSet<M>) {
    let removed = world.resource_scope(|world, mut schedules: Mut<Schedules>| {
        schedules
            .get_mut(Render)
            .expect("indirect-parameter isolation requires the Render schedule")
            .remove_systems_in_set(target, world, ScheduleCleanupPolicy::RemoveSystemsOnly)
            .expect("failed to remove the indirect-parameter upload system")
    });
    assert_eq!(
        removed, 1,
        "expected exactly one indirect-parameter upload system"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::schedule::Schedules;

    #[derive(Resource, Default)]
    struct WorkCounts {
        target: usize,
        unrelated: usize,
    }

    fn target_work(mut counts: ResMut<WorkCounts>) {
        counts.target += 1;
    }

    fn unrelated_work(mut counts: ResMut<WorkCounts>) {
        counts.unrelated += 1;
    }

    #[test]
    fn removes_only_target_after_deadline_and_only_once() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let mut world = World::new();
        let mut main_world = MainWorld::default();
        let mut time = Time::<Real>::default();
        time.advance_by(Duration::from_secs(19));
        main_world.insert_resource(time);
        world.insert_resource(main_world);
        world.insert_resource(FreezeDeadline(Duration::from_secs(20)));
        world.init_resource::<WorkCounts>();
        let mut schedules = Schedules::default();
        let mut render = Schedule::new(Render);
        render.add_systems((target_work, unrelated_work));
        schedules.insert(render);
        world.insert_resource(schedules);

        world.run_schedule(Render);
        remove_target_once(&mut world, target_work);
        world.run_schedule(Render);
        assert_eq!(world.resource::<WorkCounts>().target, 2);
        assert_eq!(world.resource::<WorkCounts>().unrelated, 2);

        world
            .resource_mut::<MainWorld>()
            .resource_mut::<Time<Real>>()
            .advance_by(Duration::from_secs(1));
        remove_target_once(&mut world, target_work);
        world.run_schedule(Render);
        assert_eq!(world.resource::<WorkCounts>().target, 2);
        assert_eq!(world.resource::<WorkCounts>().unrelated, 3);

        remove_target_once(&mut world, target_work);
        world.run_schedule(Render);
        assert_eq!(world.resource::<WorkCounts>().target, 2);
        assert_eq!(world.resource::<WorkCounts>().unrelated, 4);
    }

    #[test]
    fn absent_flag_does_not_require_or_change_a_render_app() {
        configure(&mut App::new(), &[]);
        assert_eq!(parse_deadline(&[]), None);
        assert_eq!(
            parse_deadline(&["--freeze-indirect-parameters-after".into(), "20".into()]),
            Some(Duration::from_secs(20))
        );
        let args = [
            "--freeze-indirect-parameters-after".into(),
            "20".into(),
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
        parse_deadline(&["--freeze-indirect-parameters-after".into()]);
    }

    #[test]
    #[should_panic(expected = "requires unsigned integer seconds")]
    fn rejects_invalid_deadline() {
        parse_deadline(&["--freeze-indirect-parameters-after".into(), "-1".into()]);
    }
}
