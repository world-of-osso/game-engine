use super::*;
use game_engine_network::ProtocolMessage;
use shared::protocol::{CalendarEventSnapshot, CalendarSnapshot, CalendarStateUpdate};

#[test]
fn calendar_snapshot_update_reaches_the_account_event_queue() {
    let mut account = Account::new(PathBuf::new());
    let update = CalendarStateUpdate {
        snapshot: Some(CalendarSnapshot {
            events: vec![CalendarEventSnapshot {
                event_id: 7,
                title: "Server raid".into(),
                organizer_name: "Alice".into(),
                starts_at_unix_secs: 1_800_000_000,
                max_signups: 10,
                is_raid: true,
                signups: Vec::new(),
            }],
        }),
        message: Some("calendar loaded".into()),
        error: None,
    };
    let mut events = Vec::new();

    account
        .dispatch_message(ProtocolMessage::for_tests(update.clone()), &mut events)
        .unwrap();

    assert!(matches!(
        events.as_slice(),
        [AccountEvent::Calendar(received)] if received == &update
    ));
}
