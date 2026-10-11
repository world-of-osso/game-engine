use game_engine_ui_model::{
    chat_frame::{ChatCommand, parse_chat_input},
    group_state::{GroupCommand, GroupState},
};
use shared::protocol::GroupCountdownUpdate;

fn update(seconds: u32) -> GroupCountdownUpdate {
    GroupCountdownUpdate {
        deadline_unix_ms: u64::from(seconds) * 1000,
        initiator_name: "Alice".into(),
        duration_secs: seconds,
        time_remaining_secs: seconds as f32,
    }
}

#[test]
fn countdown_slash_accepts_duration_and_zero_cancel_without_inventing_defaults() {
    for seconds in [10, 30, 0] {
        assert_eq!(
            parse_chat_input(&format!("/countdown {seconds}"), None),
            ChatCommand::Group(GroupCommand::Countdown(seconds))
        );
    }
    for text in ["/countdown", "/countdown nope", "/countdown -1"] {
        assert_eq!(parse_chat_input(text, None), ChatCommand::None);
    }
}

#[test]
fn countdown_ticks_once_per_number_finishes_once_and_cancel_is_silent() {
    let mut group = GroupState::default();
    group.apply_countdown_at(update(10), 0);
    assert_eq!(group.countdown.as_ref().unwrap().seconds(), 10);
    assert_eq!(group.drain_countdown_sounds(), vec![158960, 158565]);
    group.tick_countdown(0.25);
    assert!(group.drain_countdown_sounds().is_empty());
    group.tick_countdown(0.75);
    assert_eq!(group.countdown.as_ref().unwrap().seconds(), 9);
    assert_eq!(group.drain_countdown_sounds(), vec![158565]);
    group.tick_countdown(9.0);
    assert!(group.countdown.is_none());
    assert_eq!(group.drain_countdown_sounds(), vec![158566]);
    group.apply_countdown_at(
        GroupCountdownUpdate {
            time_remaining_secs: 0.0,
            ..update(10)
        },
        10000,
    );
    assert!(group.drain_countdown_sounds().is_empty());
    group.apply_countdown_at(update(10), 0);
    group.drain_countdown_sounds();
    group.apply_countdown_at(update(0), 0);
    assert!(group.countdown.is_none());
    assert!(group.drain_countdown_sounds().is_empty());
}

#[test]
fn countdown_delayed_delivery_uses_server_deadline_and_never_revives_expired_timer() {
    let mut group = GroupState::default();
    group.apply_countdown_at(update(10), 12000);
    assert!(group.countdown.is_none());
    assert!(group.drain_countdown_sounds().is_empty());
    group.apply_countdown_at(update(10), 2500);
    assert_eq!(
        group.countdown.as_ref().unwrap().update.time_remaining_secs,
        7.5
    );
    assert_eq!(group.countdown.as_ref().unwrap().seconds(), 8);
}

#[test]
fn countdown_bar_switches_to_medium_then_large_at_retail_markers() {
    let mut group = GroupState::default();
    group.apply_countdown_at(update(40), 0);
    assert!(group.countdown.as_ref().unwrap().bar_visible());
    assert_eq!(group.drain_countdown_sounds(), vec![158958]);
    group.tick_countdown(10.0);
    assert!(!group.countdown.as_ref().unwrap().bar_visible());
    assert!(!group.countdown.as_ref().unwrap().large());
    assert_eq!(group.drain_countdown_sounds(), vec![158959, 158565]);
    group.tick_countdown(20.0);
    assert!(group.countdown.as_ref().unwrap().large());
    assert_eq!(group.drain_countdown_sounds(), vec![158960, 158565]);
}
