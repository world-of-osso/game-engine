use super::*;

#[test]
fn party_start_ready_check() {
    let mut state = PartyState {
        members: vec![named_unit("Alice"), named_unit("Bob")],
        ..Default::default()
    };
    state.start_ready_check();
    assert!(state.ready_check_active);
    assert_eq!(state.members[0].ready_check, ReadyCheck::Pending);
    assert_eq!(state.members[1].ready_check, ReadyCheck::Pending);
}

#[test]
fn party_respond_ready_check() {
    let mut state = PartyState {
        members: vec![named_unit("Alice"), named_unit("Bob")],
        ..Default::default()
    };
    state.start_ready_check();
    state.respond_ready_check("Alice", ReadyCheck::Accepted);
    assert_eq!(state.members[0].ready_check, ReadyCheck::Accepted);
    assert_eq!(state.members[1].ready_check, ReadyCheck::Pending);
    assert!(!state.all_responded());

    state.respond_ready_check("Bob", ReadyCheck::Declined);
    assert!(state.all_responded());
}

#[test]
fn party_finish_ready_check() {
    let mut state = PartyState {
        members: vec![named_unit("Alice")],
        ..Default::default()
    };
    state.start_ready_check();
    state.respond_ready_check("Alice", ReadyCheck::Accepted);
    state.finish_ready_check();
    assert!(!state.ready_check_active);
    assert_eq!(state.members[0].ready_check, ReadyCheck::None);
}

#[test]
fn party_all_responded_not_active() {
    let state = PartyState::default();
    assert!(!state.all_responded());
}

#[test]
fn ready_check_state_start() {
    let mut state = ReadyCheckState::default();
    state.start("Leader".into());
    assert!(state.active);
    assert_eq!(state.initiator, "Leader");
    assert!(state.awaiting_response());
    assert!((state.remaining_secs - READY_CHECK_TIMEOUT_SECS).abs() < 0.01);
}

#[test]
fn ready_check_state_respond() {
    let mut state = ReadyCheckState::default();
    state.start("Leader".into());
    state.respond(ReadyCheck::Accepted);
    assert!(!state.awaiting_response());
    assert_eq!(state.local_response, ReadyCheck::Accepted);
}

#[test]
fn ready_check_state_finish() {
    let mut state = ReadyCheckState::default();
    state.start("Leader".into());
    state.respond(ReadyCheck::Accepted);
    state.finish();
    assert!(!state.active);
    assert_eq!(state.remaining_secs, 0.0);
}

#[test]
fn ready_check_state_awaiting_only_when_pending() {
    let mut state = ReadyCheckState::default();
    assert!(!state.awaiting_response());
    state.start("X".into());
    assert!(state.awaiting_response());
    state.respond(ReadyCheck::Declined);
    assert!(!state.awaiting_response());
}
