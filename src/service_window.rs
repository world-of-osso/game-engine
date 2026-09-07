use bevy::{
    app::{App, Last, PluginGroup, ScheduleRunnerPlugin},
    ecs::{resource::Resource, system::ResMut},
    prelude::MinimalPlugins,
};

const UPDATE_LOG_INTERVAL: u64 = 64;

/// Builds the first additive Bevy layer for the native empty-window runner.
///
/// `MinimalPlugins` installs `TaskPoolPlugin`, `FrameCountPlugin`, and `TimePlugin`.
/// `ScheduleRunnerPlugin` is removed: the parent-owned native winit event loop calls
/// [`App::update`] only after actual events, while its `ControlFlow::Wait` blocks idle.
pub(crate) fn build_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.build().disable::<ScheduleRunnerPlugin>())
        .init_resource::<EventDrivenUpdateCount>()
        .add_systems(Last, log_update_count);
    app
}

#[derive(Resource, Default)]
struct EventDrivenUpdateCount(u64);

fn log_update_count(mut count: ResMut<EventDrivenUpdateCount>) {
    count.0 += 1;
    if count.0 == 1 || count.0.is_multiple_of(UPDATE_LOG_INTERVAL) {
        eprintln!("service window updates: {}", count.0);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{
        app::Update,
        ecs::{resource::Resource, system::ResMut},
        time::{Time, TimeUpdateStrategy, Virtual},
    };

    use super::build_app;

    #[derive(Resource, Default)]
    struct UpdateCount(u32);

    fn count_updates(mut count: ResMut<UpdateCount>) {
        count.0 += 1;
    }

    #[test]
    fn event_driven_app_updates_and_advances_time() {
        let mut app = build_app();
        app.init_resource::<UpdateCount>()
            .add_systems(Update, count_updates)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
                10,
            )));

        app.update();
        app.update();

        assert_eq!(app.world().resource::<UpdateCount>().0, 2);
        assert_eq!(
            app.world().resource::<Time<Virtual>>().elapsed(),
            Duration::from_millis(10)
        );
    }
}
