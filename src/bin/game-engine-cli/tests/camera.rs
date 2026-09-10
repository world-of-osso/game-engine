use super::*;

#[test]
fn camera_set_accepts_each_axis_and_negative_values() {
    for (args, expected_yaw, expected_pitch) in [
        (vec!["--yaw-degrees", "-90"], Some(-90.0), None),
        (vec!["--pitch-degrees", "-45"], None, Some(-45.0)),
        (
            vec!["--yaw-degrees", "120", "--pitch-degrees", "-30"],
            Some(120.0),
            Some(-30.0),
        ),
    ] {
        let mut command = vec!["game-engine-cli", "camera", "set"];
        command.extend(args);
        let parsed = Cli::try_parse_from(command).unwrap();
        let Cmd::Camera { command: camera } = parsed.command else {
            panic!("expected camera command");
        };
        let request = camera_request(camera);
        assert_eq!(
            request,
            Request::SetCameraDirection {
                yaw_degrees: expected_yaw,
                pitch_degrees: expected_pitch,
            }
        );
    }
}

#[test]
fn camera_direction_request_preserves_optional_axes_on_wire() {
    for payload in [
        serde_json::json!({"yaw_degrees": -90.0, "pitch_degrees": null}),
        serde_json::json!({"yaw_degrees": null, "pitch_degrees": -45.0}),
        serde_json::json!({"yaw_degrees": 120.0, "pitch_degrees": -30.0}),
    ] {
        let wire = serde_json::json!({"SetCameraDirection": payload});
        let request: Request = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(request).unwrap(), wire);
    }
}

#[test]
fn camera_set_requires_an_axis() {
    assert!(Cli::try_parse_from(["game-engine-cli", "camera", "set"]).is_err());
}
