//! Timed whole-Update removal; the first update still runs at a zero deadline.

use std::time::Duration;

use bevy::app::{Last, Main, MainScheduleOrder, PostUpdate, PreUpdate, Update};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::{App, IntoScheduleConfigs, Res, ResMut, Resource};
use bevy::time::{Real, Time};

const FLAG: &str = "--skip-update-after";
const MAIN_WORK_FLAG: &str = "--skip-main-work-after";
const PRE_FLAG: &str = "--skip-pre-update-after";
const POST_FLAG: &str = "--skip-post-update-after";

#[derive(Resource)]
struct UpdateCutoff {
    deadline: Duration,
    elapsed: Duration,
    last_report: Duration,
    updates: u64,
    removed: bool,
    flag: &'static str,
}

pub(crate) fn configure(app: &mut App, arguments: &[String]) {
    let selected: Vec<_> = [FLAG, MAIN_WORK_FLAG, PRE_FLAG, POST_FLAG]
        .into_iter()
        .filter(|flag| arguments.iter().any(|arg| arg == flag))
        .collect();
    assert!(
        selected.len() <= 1,
        "choose only one main-schedule cutoff diagnostic"
    );
    let flag = selected.first().copied().unwrap_or(FLAG);
    let Some(deadline) = parse_deadline(arguments, flag) else {
        return;
    };
    app.insert_resource(UpdateCutoff {
        deadline,
        elapsed: Duration::ZERO,
        last_report: Duration::ZERO,
        updates: 0,
        removed: false,
        flag,
    })
    .add_systems(Last, track_updates)
    // Main temporarily takes MainScheduleOrder while its child schedules run.
    .add_systems(Main, remove_update_at_cutoff.after(Main::run_main));
}

fn parse_deadline(arguments: &[String], flag: &str) -> Option<Duration> {
    let mut deadline = None;
    for (index, argument) in arguments.iter().enumerate() {
        if argument != flag {
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
    let labels = match cutoff.flag {
        MAIN_WORK_FLAG => vec![PreUpdate.intern(), Update.intern(), PostUpdate.intern()],
        PRE_FLAG => vec![PreUpdate.intern()],
        POST_FLAG => vec![PostUpdate.intern()],
        _ => vec![Update.intern()],
    };
    for label in labels {
        let index = order
            .labels
            .iter()
            .position(|candidate| *candidate == label)
            .unwrap_or_else(|| panic!("{FLAG}: {label:?} absent from MainScheduleOrder"));
        order.labels.remove(index);
    }
    let removed = match cutoff.flag {
        MAIN_WORK_FLAG => "PreUpdate,Update,PostUpdate",
        PRE_FLAG => "PreUpdate",
        POST_FLAG => "PostUpdate",
        _ => "Update",
    };
    cutoff.removed = true;
    eprintln!(
        "update schedule isolation cutoff elapsed_s={:.3} updates={} removed={removed} deadline_s={}",
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
        pre: usize,
        update: usize,
        post: usize,
        last: usize,
    }

    fn test_app(arguments: &[&str]) -> App {
        let mut app = App::new();
        app.init_resource::<Time<Real>>()
            .init_resource::<Counts>()
            .add_systems(First, |mut counts: ResMut<Counts>| counts.first += 1)
            .add_systems(PreUpdate, |mut counts: ResMut<Counts>| counts.pre += 1)
            .add_systems(Update, |mut counts: ResMut<Counts>| counts.update += 1)
            .add_systems(PostUpdate, |mut counts: ResMut<Counts>| counts.post += 1)
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
    fn whole_main_work_stops_while_first_and_last_continue() {
        let mut app = test_app(&["--skip-main-work-after", "0"]);
        for _ in 0..3 {
            app.update();
        }
        let counts = app.world().resource::<Counts>();
        assert_eq!(
            (
                counts.first,
                counts.pre,
                counts.update,
                counts.post,
                counts.last
            ),
            (3, 1, 1, 1, 3)
        );
    }

    #[test]
    fn individual_pre_and_post_cutoffs_preserve_other_work() {
        for (flag, expected) in [(PRE_FLAG, (1, 3, 3)), (POST_FLAG, (3, 3, 1))] {
            let mut app = test_app(&[flag, "0"]);
            for _ in 0..3 {
                app.update();
            }
            let counts = app.world().resource::<Counts>();
            assert_eq!((counts.pre, counts.update, counts.post), expected);
            assert_eq!((counts.first, counts.last), (3, 3));
        }
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
