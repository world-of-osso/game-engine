use game_engine_session::{PendingWorldPort, Session, SessionScreen};
use shared::protocol::{TransferAbortReason, TransferAborted};

#[test]
fn new_world_ack_waits_for_world_entry_and_is_consumed_once() {
    let mut session = Session::default();
    session.begin_world_port(530);
    assert_eq!(session.screen, SessionScreen::Loading);
    assert_eq!(session.pending_world_port(), PendingWorldPort::Loading(530));
    assert_eq!(session.loaded_world_port(), None);
    assert_eq!(session.take_world_port_ack(), None);

    session.finish_world_port();
    assert_eq!(session.pending_world_port(), PendingWorldPort::Loaded(530));
    assert_eq!(session.loaded_world_port(), Some(530));
    assert_eq!(session.take_world_port_ack(), Some(530));
    assert_eq!(session.take_world_port_ack(), None);
    assert_eq!(session.pending_world_port(), PendingWorldPort::None);
}

#[test]
fn newest_destination_replaces_earlier_pending_ack() {
    let mut session = Session::default();
    session.begin_world_port(0);
    session.finish_world_port();
    session.begin_world_port(1);
    assert_eq!(session.loaded_world_port(), None);
    session.finish_world_port();
    assert_eq!(session.take_world_port_ack(), Some(1));
}

#[test]
fn refused_transfer_exposes_retail_error_without_clearing_pending_world() {
    let mut session = Session::default();
    session.begin_world_port(530);
    let error = session.receive_transfer_aborted(TransferAborted {
        map_id: 1,
        reason: TransferAbortReason::MapNotAllowed,
    });
    assert_eq!(error, "Map cannot be entered at this time.");
    assert_eq!(session.pending_world_port(), PendingWorldPort::Loading(530));
    assert_eq!(session.screen, SessionScreen::Loading);
    assert_eq!(session.feedback, None);
}

#[test]
fn account_reset_discards_unsent_transfer_ack() {
    let mut session = Session::default();
    session.begin_world_port(530);
    session.finish_world_port();
    session.reset_world_port();
    assert_eq!(session.pending_world_port(), PendingWorldPort::None);
    assert_eq!(session.loaded_world_port(), None);
    assert_eq!(session.take_world_port_ack(), None);
}
