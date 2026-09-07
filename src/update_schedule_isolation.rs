//! Timed whole-Update removal; the first update still runs at a zero deadline.

use std::time::Duration;

use bevy::app::{Last, Main, MainScheduleOrder, Update};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::{App, IntoScheduleConfigs, Res, ResMut, Resource};
use bevy::time::{Real, Time};

const FLAG: &str = "--skip-update-after";

#[derive(Resource)]
struct UpdateCutoff {
    deadline: Duration,
    elapsed: Duration,
    last_report: Duration,
    updates: u64,
    removed: bool,
}

pub(crate) fn configure(app: &mut App, arguments: &[String]) {
    let Some(deadline) = parse_deadline(arguments) else {
        return;
    };
    app.insert_resource(UpdateCutoff {
        deadline,
        elapsed: Duration::ZERO,
        last_report: Duration::ZERO,
        updates: 0,
        removed: false,
    })
    .add_systems(Last, track_updates)
    // Main temporarily takes MainScheduleOrder while its child schedules run.
    .add_systems(Main, remove_update_at_cutoff.after(Main::run_main));
}

fn parse_deadline(arguments: &[String]) -> Option<Duration> {
    let mut deadline = None;
    for (index, argument) in arguments.iter().enumerate() {
        if argument != FLAG {
            continue;
        }
        assert!(deadline.is_none(), "{FLAG}: duplicate option");
        let value = arguments
            .get(index + 1)
            .unwrap_or_else(|| panic!("{FLAG}: missing unsigned SECONDS"));
        assert!(
            !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()),
            "{FLAG}: invalid unsigned SECONDS {value:?}"
        );
        let seconds = value
            .parse::<u64>()
            .unwrap_or_else(|error| panic!("{FLAG}: invalid unsigned SECONDS {value:?}: {error}"));
        deadline = Some(Duration::from_secs(seconds));
    }
    deadline
}

fn track_updates(time: Res<Time<Real>>, mut cutoff: ResMut<UpdateCutoff>) {
    cutoff.elapsed = time.elapsed();
    cutoff.updates += 1;
    if cutoff.elapsed.saturating_sub(cutoff.last_report) >= Duration::from_secs(1) {
        eprintln!(
            "update schedule isolation elapsed_s={:.3} updates={} update_removed={}",
            cutoff.elapsed.as_secs_f64(),
            cutoff.updates,
            cutoff.removed,
        );
        cutoff.last_report = cutoff.elapsed;
    }
}

fn remove_update_at_cutoff(mut order: ResMut<MainScheduleOrder>, mut cutoff: ResMut<UpdateCutoff>) {
    if cutoff.removed || cutoff.elapsed < cutoff.deadline {
        return;
    }
    let index = order
        .labels
        .iter()
        .position(|label| *label == Update.intern())
        .unwrap_or_else(|| panic!("{FLAG}: Update absent from MainScheduleOrder"));
    order.labels.remove(index);
    cutoff.removed = true;
    eprintln!(
        "update schedule isolation cutoff elapsed_s={:.3} updates={} removed=Update deadline_s={}",
        cutoff.elapsed.as_secs_f64(),
        cutoff.updates,
        cutoff.deadline.as_secs(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::app::First;

    #[derive(Resource, Default)]
    struct Counts {
        first: usize,
        update: usize,
        last: usize,
    }

    fn test_app(arguments: &[&str]) -> App {
        let mut app = App::new();
        app.init_resource::<Time<Real>>()
            .init_resource::<Counts>()
            .add_systems(First, |mut counts: ResMut<Counts>| counts.first += 1)
            .add_systems(Update, |mut counts: ResMut<Counts>| counts.update += 1)
            .add_systems(Last, |mut counts: ResMut<Counts>| counts.last += 1);
        let arguments = arguments
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>();
        configure(&mut app, &arguments);
        app
    }

    #[test]
    fn update_stops_after_deadline_while_first_and_last_continue() {
        let mut app = test_app(&["--skip-update-after", "2"]);
        let original = app.world().resource::<MainScheduleOrder>().labels.clone();
        let startup = app
            .world()
            .resource::<MainScheduleOrder>()
            .startup_labels
            .clone();
        app.update();
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(Duration::from_secs(1));
        app.update();
        assert_eq!(app.world().resource::<MainScheduleOrder>().labels, original);
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(Duration::from_secs(1));
        app.update();
        assert_eq!(app.world().resource::<Counts>().update, 3);
        for _ in 0..3 {
            app.world_mut()
                .resource_mut::<Time<Real>>()
                .advance_by(Duration::from_secs(1));
            app.update();
        }
        let counts = app.world().resource::<Counts>();
        assert_eq!((counts.first, counts.update, counts.last), (6, 3, 6));
        let expected = original
            .into_iter()
            .filter(|label| *label != Update.intern())
            .collect::<Vec<_>>();
        let order = app.world().resource::<MainScheduleOrder>();
        assert_eq!(order.labels, expected);
        assert_eq!(order.startup_labels, startup);
    }

    #[test]
    fn zero_deadline_allows_first_update_then_removes_only_update() {
        let mut app = test_app(&["--skip-update-after", "0"]);
        app.update();
        app.update();
        let counts = app.world().resource::<Counts>();
        assert_eq!((counts.first, counts.update, counts.last), (2, 1, 2));
    }

    #[test]
    fn absent_flag_leaves_schedule_and_tracking_unchanged() {
        let mut app = test_app(&["--service-window", "continuous"]);
        let original = app.world().resource::<MainScheduleOrder>().labels.clone();
        app.update();
        app.update();
        assert_eq!(app.world().resource::<Counts>().update, 2);
        assert_eq!(app.world().resource::<MainScheduleOrder>().labels, original);
        assert!(!app.world().contains_resource::<UpdateCutoff>());
    }

    #[test]
    fn invalid_cutoff_arguments_fail_explicitly() {
        for arguments in [
            vec!["--skip-update-after"],
            vec!["--skip-update-after", "-1"],
            vec!["--skip-update-after", "1.5"],
            vec!["--skip-update-after", "abc"],
            vec!["--skip-update-after", "18446744073709551616"],
            vec!["--skip-update-after", "0", "--skip-update-after", "1"],
        ] {
            assert!(std::panic::catch_unwind(|| test_app(&arguments)).is_err());
        }
    }

    #[test]
    #[should_panic(expected = "--skip-update-after: Update absent from MainScheduleOrder")]
    fn missing_update_label_fails_at_cutoff() {
        let mut app = test_app(&["--skip-update-after", "0"]);
        app.world_mut()
            .resource_mut::<MainScheduleOrder>()
            .labels
            .retain(|label| *label != Update.intern());
        app.update();
    }
}
