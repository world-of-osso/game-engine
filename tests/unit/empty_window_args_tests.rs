use super::{ServiceWindowMode, requests_empty_window, requests_service_window};

#[test]
fn service_window_mode_is_explicit_and_exclusive() {
    for (name, mode) in [
        ("core", ServiceWindowMode::Core),
        ("render", ServiceWindowMode::Render),
        ("continuous", ServiceWindowMode::Continuous),
    ] {
        assert_eq!(
            requests_service_window(&["--service-window".into(), name.into()]),
            Ok(Some(mode))
        );
    }
    assert!(requests_service_window(&["--service-window".into()]).is_err());
    assert!(requests_service_window(&["--service-window".into(), "unknown".into()]).is_err());
    assert!(
        requests_service_window(&["--service-window".into(), "--empty-window".into()]).is_err()
    );
    assert!(
        requests_service_window(&["--service-window".into(), "--server".into(), "dev".into()])
            .is_err()
    );
    assert_eq!(requests_service_window(&[]), Ok(None));
    assert_eq!(
        requests_service_window(&["--screen".into(), "inworld".into()]),
        Ok(None)
    );
    assert_eq!(
        requests_service_window(&["--empty-window".into()]),
        Ok(None)
    );
}

#[test]
fn service_window_continuous_accepts_one_generic_removal() {
    for (location, system, seconds) in [
        (
            "main:PostUpdate",
            "bevy_transform::systems::mark_dirty_trees",
            "10",
        ),
        ("render:Render", "example::another_callback", "0"),
        (
            "main:FixedUpdate:Combat",
            "example::third_callback",
            "18446744073709551615",
        ),
    ] {
        let args = strings(&[
            "--service-window",
            "continuous",
            "--remove-system-after",
            location,
            system,
            seconds,
        ]);
        assert_eq!(
            requests_service_window(&args),
            Ok(Some(ServiceWindowMode::Continuous))
        );
    }
}

#[test]
fn service_window_rejects_removal_tails_outside_the_single_continuous_form() {
    for values in [
        vec![
            "--service-window",
            "core",
            "--remove-system-after",
            "main:PostUpdate",
            "callback",
            "10",
        ],
        vec![
            "--service-window",
            "render",
            "--remove-system-after",
            "main:PostUpdate",
            "callback",
            "10",
        ],
        vec!["--service-window", "continuous", "--remove-system-after"],
        vec![
            "--service-window",
            "continuous",
            "--remove-system-after",
            "main:PostUpdate",
        ],
        vec![
            "--service-window",
            "continuous",
            "--remove-system-after",
            "main:PostUpdate",
            "callback",
        ],
        vec![
            "--service-window",
            "continuous",
            "--unknown",
            "main:PostUpdate",
            "callback",
            "10",
        ],
        vec![
            "--service-window",
            "continuous",
            "--remove-system-after",
            "main:PostUpdate",
            "callback",
            "10",
            "extra",
        ],
        vec![
            "--service-window",
            "continuous",
            "--remove-system-after",
            "main:PostUpdate",
            "callback",
            "10",
            "--remove-system-after",
            "main:Update",
            "another",
            "20",
        ],
        vec!["extra", "--service-window", "continuous"],
    ] {
        assert!(
            requests_service_window(&strings(&values)).is_err(),
            "{values:?}"
        );
    }
}

#[test]
fn service_window_preserves_removal_parser_validation_errors() {
    for (location, system, seconds, expected) in [
        ("other:PostUpdate", "callback", "10", "unknown world"),
        ("PostUpdate", "callback", "10", "schedule must use"),
        ("main:", "callback", "10", "must be nonempty"),
        ("main:   ", "callback", "10", "must be nonempty"),
        ("main:PostUpdate", "", "10", "must be nonempty"),
        ("main:PostUpdate", "   ", "10", "must be nonempty"),
        ("main:PostUpdate", "callback", "-1", "unsigned integer"),
        ("main:PostUpdate", "callback", "1.5", "unsigned integer"),
        ("main:PostUpdate", "callback", "ten", "unsigned integer"),
        (
            "main:PostUpdate",
            "callback",
            "18446744073709551616",
            "unsigned integer",
        ),
    ] {
        let args = strings(&[
            "--service-window",
            "continuous",
            "--remove-system-after",
            location,
            system,
            seconds,
        ]);
        let error = requests_service_window(&args).unwrap_err();
        assert!(error.contains(expected), "{args:?}: {error}");
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn empty_window_mode_is_explicit_and_exclusive() {
    assert_eq!(requests_empty_window(&["--empty-window".into()]), Ok(true));
    assert!(
        requests_empty_window(&["--empty-window".into(), "--server".into(), "dev".into()]).is_err()
    );
    assert!(requests_empty_window(&["--empty-window".into(), "model.m2".into()]).is_err());
}

#[test]
fn normal_arguments_keep_normal_startup() {
    assert_eq!(requests_empty_window(&[]), Ok(false));
    assert_eq!(
        requests_empty_window(&["--screen".into(), "inworld".into()]),
        Ok(false)
    );
}
