use super::*;

/// Death and Decay's kits start at the cast event (3) and end at the one-shot sentinel
/// (13), so an area trigger re-issues both of them on every update.
#[test]
fn cast_to_one_shot_kits_are_replayed() {
    assert!(replays_kit(VisualEvent::Cast, VisualEvent::OneShot));
    assert!(replays_kit(VisualEvent::Impact, VisualEvent::OneShot));
    assert!(replays_kit(VisualEvent::AuraStart, VisualEvent::OneShot));
}

/// The channel and gate start events are excluded even though they end at the sentinel.
#[test]
fn channel_and_gate_start_events_are_not_replayed() {
    for start in [
        VisualEvent::Other(9),
        VisualEvent::Other(10),
        VisualEvent::Other(14),
        VisualEvent::Other(15),
        VisualEvent::Other(16),
        VisualEvent::Other(17),
        VisualEvent::ChannelStart,
        VisualEvent::ChannelEnd,
    ] {
        assert!(!replays_kit(start, VisualEvent::OneShot), "{start:?}");
    }
}

/// A kit with its own end event is not a one-shot replay, and display-state visuals are
/// client lifecycle, never area-trigger kit sources.
#[test]
fn kits_that_end_on_their_own_event_or_display_state_are_not_replayed() {
    assert!(!replays_kit(VisualEvent::AuraStart, VisualEvent::AuraEnd));
    assert!(!replays_kit(
        VisualEvent::ChannelStart,
        VisualEvent::ChannelEnd
    ));
    assert!(!replays_kit(
        VisualEvent::DisplayStart,
        VisualEvent::OneShot
    ));
    assert!(!replays_kit(VisualEvent::DisplayEnd, VisualEvent::OneShot));
}
