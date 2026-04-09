use super::*;

#[test]
fn accept_advances_to_post_eula_state_when_save_succeeds() {
    let mut status = "old error".to_string();

    let resolution = resolve_eula_action(
        EulaAction::Accept,
        |_| Ok(()),
        Some(GameState::Connecting),
        &mut status,
    );

    assert_eq!(
        resolution,
        EulaResolution::Transition(GameState::Connecting)
    );
    assert!(status.is_empty());
}

#[test]
fn accept_stays_on_eula_when_save_fails() {
    let mut status = String::new();

    let resolution = resolve_eula_action(
        EulaAction::Accept,
        |_| Err("disk full".into()),
        Some(GameState::Login),
        &mut status,
    );

    assert_eq!(resolution, EulaResolution::Stay);
    assert_eq!(status, "Failed to save acceptance: disk full");
}

#[test]
fn decline_requests_exit() {
    let mut status = String::new();

    let resolution = resolve_eula_action(
        EulaAction::Decline,
        |_| Ok(()),
        Some(GameState::Login),
        &mut status,
    );

    assert_eq!(resolution, EulaResolution::Exit);
    assert!(status.is_empty());
}

#[test]
fn build_state_uses_status_text() {
    let state = build_state(&EulaStatus("save failed".into()));

    assert_eq!(
        state,
        EulaScreenState {
            status_text: "save failed".into(),
        }
    );
}
