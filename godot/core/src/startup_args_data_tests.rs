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
            character: None
        }
    );
}

#[test]
fn script_option_is_accepted_with_login_startup() {
    parse(&["--screen", "login", "--run-js-ui-script", "debug/login.js"])
        .expect("the existing JS script option must reach native startup");
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
