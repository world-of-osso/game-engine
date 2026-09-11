use bevy::input::keyboard::KeyCode;
use bevy::prelude::NextState;

use game_engine::ui::automation::UiAutomationAction;

use super::super::{LoginFocus, LoginStatus};
use super::support::{login_fixture, make_ui_state, run_automation_action};
use crate::game_state::GameState;

#[test]
fn login_workflow_type_credentials_and_connect() {
    let (reg, login) = login_fixture();
    let mut ui = make_ui_state(reg);
    let mut focus = LoginFocus::default();
    let mut status = LoginStatus::default();
    let mut next_state = NextState::<GameState>::default();

    run_automation_action(
        &mut ui,
        &login,
        &mut focus,
        &mut status,
        &mut next_state,
        &UiAutomationAction::ClickFrame("UsernameInput".to_string()),
    );
    assert_eq!(focus.0, Some(login.username_input));

    run_automation_action(
        &mut ui,
        &login,
        &mut focus,
        &mut status,
        &mut next_state,
        &UiAutomationAction::TypeText("testuser".to_string()),
    );
    assert_eq!(
        super::get_editbox_text(&ui.registry, login.username_input),
        "testuser"
    );

    run_automation_action(
        &mut ui,
        &login,
        &mut focus,
        &mut status,
        &mut next_state,
        &UiAutomationAction::PressKey(KeyCode::Tab),
    );
    assert_eq!(focus.0, Some(login.password_input));

    run_automation_action(
        &mut ui,
        &login,
        &mut focus,
        &mut status,
        &mut next_state,
        &UiAutomationAction::TypeText("secret123".to_string()),
    );
    assert_eq!(
        super::get_editbox_text(&ui.registry, login.password_input),
        "secret123"
    );

    run_automation_action(
        &mut ui,
        &login,
        &mut focus,
        &mut status,
        &mut next_state,
        &UiAutomationAction::PressKey(KeyCode::Enter),
    );

    assert!(
        !status.0.is_empty(),
        "status should have feedback after pressing Enter with credentials",
    );
}

#[test]
fn login_workflow_empty_fields_shows_error() {
    let (reg, login) = login_fixture();
    let mut ui = make_ui_state(reg);
    let mut focus = LoginFocus::default();
    let mut status = LoginStatus::default();
    let mut next_state = NextState::<GameState>::default();

    run_automation_action(
        &mut ui,
        &login,
        &mut focus,
        &mut status,
        &mut next_state,
        &UiAutomationAction::ClickFrame("ConnectButton".to_string()),
    );

    assert_eq!(status.0, "Please fill in all fields");
    assert!(matches!(next_state, NextState::Unchanged));
}

#[test]
fn login_workflow_tab_cycles_through_fields() {
    let (reg, login) = login_fixture();
    let mut ui = make_ui_state(reg);
    let mut focus = LoginFocus::default();
    let mut status = LoginStatus::default();
    let mut next_state = NextState::<GameState>::default();

    run_automation_action(
        &mut ui,
        &login,
        &mut focus,
        &mut status,
        &mut next_state,
        &UiAutomationAction::ClickFrame("UsernameInput".to_string()),
    );
    assert_eq!(
        focus.0,
        Some(login.username_input),
        "focus should be on username (id={})",
        login.username_input
    );

    run_automation_action(
        &mut ui,
        &login,
        &mut focus,
        &mut status,
        &mut next_state,
        &UiAutomationAction::PressKey(KeyCode::Tab),
    );
    assert_eq!(
        focus.0,
        Some(login.password_input),
        "Tab should move focus to password (id={}), got {:?}. username_id={}",
        login.password_input,
        focus.0,
        login.username_input
    );

    run_automation_action(
        &mut ui,
        &login,
        &mut focus,
        &mut status,
        &mut next_state,
        &UiAutomationAction::PressKey(KeyCode::Tab),
    );
    assert_eq!(
        focus.0,
        Some(login.username_input),
        "Tab again should cycle back to username"
    );
}
