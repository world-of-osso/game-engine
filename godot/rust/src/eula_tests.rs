use game_engine_core::{screen_arg_data::ScreenArg, startup_args_data::StartupTarget};

use super::eula_precedes_login;

const LOGIN: StartupTarget = StartupTarget::Screen(ScreenArg::Login);
const CHARSELECT: StartupTarget = StartupTarget::Screen(ScreenArg::CharSelect);
const EULA: StartupTarget = StartupTarget::Screen(ScreenArg::Eula);

#[test]
fn enabled_gate_precedes_plain_and_login_startup_until_accepted() {
    assert!(eula_precedes_login(None, false, true));
    assert!(eula_precedes_login(Some(&LOGIN), false, true));
    assert!(!eula_precedes_login(None, true, true));
    assert!(!eula_precedes_login(Some(&LOGIN), true, true));
}

#[test]
fn disabled_gate_and_direct_screens_skip_it() {
    assert!(!eula_precedes_login(None, false, false));
    assert!(!eula_precedes_login(Some(&LOGIN), false, false));
    assert!(!eula_precedes_login(Some(&CHARSELECT), false, true));
}

#[test]
fn explicit_screen_shows_it_even_after_acceptance() {
    assert!(eula_precedes_login(Some(&EULA), true, false));
}
