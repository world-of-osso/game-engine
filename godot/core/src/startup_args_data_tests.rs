use crate::screen_arg_data::ScreenArg;
use crate::startup_args_data::{StartupArgs, StartupTarget};
use std::str::FromStr;

fn parse(args: &[&str]) -> Result<StartupArgs, String> {
    StartupArgs::parse(&args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>())
}

#[test]
fn all_eighteen_canonical_screens_round_trip() {
    for name in ScreenArg::CLI_VALUES {
        let screen = ScreenArg::from_str(name).unwrap();
        assert_eq!(screen.as_cli_str(), name);
        assert_eq!(
            parse(&["--screen", name]).unwrap().target,
            Some(StartupTarget::Screen(screen))
        );
    }
}

#[test]
fn original_screen_aliases_are_accepted() {
    for (alias, canonical) in [
        ("inworld-selectiondebug", "inworldselectiondebug"),
        ("m2-debug", "m2debug"),
        ("skybox-debug", "skyboxdebug"),
        ("menu", "gamemenu"),
        ("options", "optionsmenu"),
        ("nameplate-debug", "nameplatedebug"),
    ] {
        let screen = ScreenArg::from_str(canonical).unwrap();
        assert_eq!(
            parse(&["--screen", alias]).unwrap().target,
            Some(StartupTarget::Screen(screen))
        );
    }
}

#[test]
fn state_accepts_connection_states_and_normalizes_special_screens() {
    assert_eq!(
        parse(&["--state", "connecting"]).unwrap().target,
        Some(StartupTarget::Connecting)
    );
    assert_eq!(
        parse(&["--state", "reconnecting"]).unwrap().target,
        Some(StartupTarget::Reconnecting)
    );
    for (value, normalized) in [
        ("charcreate-customize", ScreenArg::CharCreate),
        ("optionsmenu", ScreenArg::GameMenu),
        ("options", ScreenArg::GameMenu),
    ] {
        assert_eq!(
            parse(&["--state", value]).unwrap().target,
            Some(StartupTarget::Screen(normalized))
        );
    }
    assert!(
        parse(&["--screen", "connecting"])
            .unwrap_err()
            .contains("invalid --screen")
    );
    assert!(
        parse(&["--screen", "reconnecting"])
            .unwrap_err()
            .contains("invalid --screen")
    );
}

#[test]
fn first_screen_or_state_wins_regardless_of_flag_kind() {
    assert_eq!(
        parse(&["--state", "login", "--screen", "inworld"])
            .unwrap()
            .target,
        Some(StartupTarget::Screen(ScreenArg::Login))
    );
    assert_eq!(
        parse(&["--screen", "charselect", "--state", "connecting"])
            .unwrap()
            .target,
        Some(StartupTarget::Screen(ScreenArg::CharSelect))
    );
}

#[test]
fn server_and_character_are_owned_optional_values() {
    let parsed = parse(&["--char", "Alice", "--server", "dev", "--screen", "inworld"]).unwrap();
    assert_eq!(parsed.server.as_deref(), Some("dev"));
    assert_eq!(parsed.character.as_deref(), Some("Alice"));
    assert_eq!(
        parsed.target,
        Some(StartupTarget::Screen(ScreenArg::InWorld))
    );
    assert_eq!(
        parse(&[]).unwrap(),
        StartupArgs {
            target: None,
            server: None,
            character: None,
            js_script: None,
        }
    );
}

#[test]
fn script_option_is_accepted_with_login_startup() {
    let parsed = parse(&["--screen", "login", "--run-js-ui-script", "debug/login.js"])
        .expect("the existing JS script option must reach native startup");
    assert_eq!(parsed.js_script.as_deref(), Some("debug/login.js"));
}

#[test]
fn skybox_original_options_accept_u32_values_and_valueless_verification() {
    for options in [
        vec!["--skybox-fdid", "120191"],
        vec!["--light-skybox-id", "42"],
        vec!["--skybox-time-ms", "43200000"],
        vec!["--skybox-verify"],
        vec![
            "--skybox-fdid",
            "0",
            "--skybox-time-ms",
            "4294967295",
            "--skybox-verify",
        ],
        vec![
            "--light-skybox-id",
            "4294967295",
            "--skybox-time-ms",
            "0",
            "--skybox-verify",
        ],
    ] {
        let args = [vec!["--screen", "skyboxdebug"], options].concat();
        let parsed = parse(&args).unwrap_or_else(|error| panic!("{args:?}: {error}"));
        assert_eq!(
            parsed.target,
            Some(StartupTarget::Screen(ScreenArg::SkyboxDebug))
        );
    }
}

#[test]
fn skybox_value_options_report_missing_values() {
    for flag in ["--skybox-fdid", "--light-skybox-id", "--skybox-time-ms"] {
        for suffix in [vec![], vec![""], vec!["--skybox-verify"]] {
            let args = [vec!["--screen", "skyboxdebug", flag], suffix].concat();
            assert_eq!(
                parse(&args).unwrap_err(),
                format!("missing value for {flag}"),
                "{args:?}"
            );
        }
    }
}

#[test]
fn skybox_value_options_report_invalid_u32_values() {
    for flag in ["--skybox-fdid", "--light-skybox-id", "--skybox-time-ms"] {
        for value in ["abc", "-1", "1.5", "4294967296"] {
            let error = parse(&["--screen", "skyboxdebug", flag, value]).unwrap_err();
            assert!(
                error.contains(&format!("invalid {flag} value '{value}'")),
                "{error}"
            );
            assert!(error.contains("u32"), "{error}");
        }
    }
}

#[test]
fn skybox_forced_fdid_and_light_id_are_mutually_exclusive_in_either_order() {
    for (first, second) in [
        ("--skybox-fdid", "--light-skybox-id"),
        ("--light-skybox-id", "--skybox-fdid"),
    ] {
        let error = parse(&["--screen", "skyboxdebug", first, "120191", second, "42"]).unwrap_err();
        assert!(error.contains("--skybox-fdid"), "{error}");
        assert!(error.contains("--light-skybox-id"), "{error}");
        assert!(error.contains("cannot be used together"), "{error}");
    }
}

#[test]
fn skybox_options_preserve_first_target_and_unknown_option_errors() {
    for (args, target) in [
        (
            vec![
                "--screen",
                "skyboxdebug",
                "--skybox-verify",
                "--state",
                "login",
            ],
            ScreenArg::SkyboxDebug,
        ),
        (
            vec![
                "--state",
                "login",
                "--skybox-verify",
                "--screen",
                "skyboxdebug",
            ],
            ScreenArg::Login,
        ),
    ] {
        assert_eq!(
            parse(&args).unwrap().target,
            Some(StartupTarget::Screen(target))
        );
    }
    assert_eq!(
        parse(&["--screen", "skyboxdebug", "--skybox-verify", "--unknown"]).unwrap_err(),
        "unknown client option '--unknown'"
    );
}

#[test]
fn invalid_missing_and_unknown_options_report_explicit_errors() {
    for args in [
        vec!["--screen"],
        vec!["--state", "--char"],
        vec!["--server"],
        vec!["--char", "--server"],
        vec!["--server", ""],
        vec!["--char", ""],
        vec!["--run-js-ui-script"],
        vec!["--run-js-ui-script", "--screen"],
        vec!["--run-js-ui-script", ""],
    ] {
        assert!(parse(&args).unwrap_err().contains(args[0]), "{args:?}");
    }
    assert!(
        parse(&["--state", "bogus"])
            .unwrap_err()
            .contains("invalid --state value 'bogus'")
    );
    assert!(
        parse(&["--screen", "bogus"])
            .unwrap_err()
            .contains("invalid --screen value 'bogus'")
    );
    assert!(
        parse(&["--unknown"])
            .unwrap_err()
            .contains("unknown client option '--unknown'")
    );
}
