use game_engine_session::logout::{LogoutRequestOutcome, LogoutState};
use std::time::Duration;

#[test]
fn open_world_starts_twenty_second_countdown() {
    let mut logout = LogoutState::default();

    assert_eq!(
        logout.request(false, false),
        LogoutRequestOutcome::StartedCountdown
    );
    assert_eq!(
        logout.remaining_text(),
        "Logging out in 20s\nMove to cancel"
    );
    assert!(!logout.tick(Duration::from_secs(19)));
    assert_eq!(logout.remaining_text(), "Logging out in 1s\nMove to cancel");
}

#[test]
fn fractional_elapsed_time_rounds_remaining_seconds_up() {
    let mut logout = LogoutState::default();
    logout.request(false, false);

    assert!(!logout.tick(Duration::from_secs_f32(2.3)));
    assert_eq!(
        logout.remaining_text(),
        "Logging out in 18s\nMove to cancel"
    );
}

#[test]
fn repeated_request_does_not_restart_pending_countdown() {
    let mut logout = LogoutState::default();
    logout.request(false, false);
    logout.tick(Duration::from_secs(7));

    assert_eq!(
        logout.request(false, false),
        LogoutRequestOutcome::AlreadyPending
    );
    assert_eq!(
        logout.remaining_text(),
        "Logging out in 13s\nMove to cancel"
    );
    assert!(!logout.tick(Duration::from_secs(12)));
    assert!(logout.tick(Duration::from_secs(1)));
}

#[test]
fn combat_blocks_even_when_resting_and_pending() {
    let mut logout = LogoutState::default();
    logout.request(false, false);
    logout.tick(Duration::from_secs(3));

    assert_eq!(
        logout.request(true, true),
        LogoutRequestOutcome::BlockedInCombat
    );
    assert_eq!(
        logout.remaining_text(),
        "Logging out in 17s\nMove to cancel"
    );
    assert_eq!(
        logout.request(true, false),
        LogoutRequestOutcome::BlockedInCombat
    );
    assert_eq!(
        logout.remaining_text(),
        "Logging out in 17s\nMove to cancel"
    );
}

#[test]
fn rest_area_immediately_logs_out_and_clears_pending() {
    let mut logout = LogoutState::default();
    assert_eq!(logout.request(false, true), LogoutRequestOutcome::Immediate);
    assert_eq!(logout.remaining_text(), "");

    logout.request(false, false);
    logout.tick(Duration::from_secs(5));
    assert_eq!(logout.request(false, true), LogoutRequestOutcome::Immediate);
    assert_eq!(logout.remaining_text(), "");
    assert!(!logout.tick(Duration::from_secs(30)));
}

#[test]
fn countdown_expires_only_once() {
    let mut logout = LogoutState::default();
    assert!(!logout.tick(Duration::from_secs(20)));
    logout.request(false, false);

    assert!(logout.tick(Duration::from_secs(20)));
    assert_eq!(logout.remaining_text(), "");
    assert!(!logout.tick(Duration::from_secs(20)));
}

#[test]
fn cancel_and_clear_remove_pending_countdown() {
    let mut logout = LogoutState::default();
    logout.request(false, false);
    logout.cancel();
    assert_eq!(logout.remaining_text(), "");
    assert!(!logout.tick(Duration::from_secs(20)));

    logout.request(false, false);
    logout.clear();
    assert_eq!(logout.remaining_text(), "");
    assert!(!logout.tick(Duration::from_secs(20)));
    assert_eq!(
        logout.request(false, false),
        LogoutRequestOutcome::StartedCountdown
    );
}
