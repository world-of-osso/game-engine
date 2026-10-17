//! UIErrorsFrame.xml displayDuration2/fadeDuration.5; fadePower shape is unqualified.
use game_engine_ui_model::ui_errors_data::UiErrorsData;

#[test]
fn error_and_info_messages_fade_after_two_seconds_and_expire_at_two_and_half() {
    let mut messages = UiErrorsData::default();
    messages.add("Out of range.");
    messages.add_info("Blackrock Worg slain: 1/6");
    messages.tick(2.0);
    assert_eq!(messages.lines.len(), 2);
    assert!(messages.lines.iter().all(|line| line.alpha() == 1.0));
    messages.tick(0.25);
    assert!(
        messages
            .lines
            .iter()
            .all(|line| line.alpha() > 0.0 && line.alpha() < 1.0),
        "Source hold has ended at2.25s"
    );
    messages.tick(0.25);
    assert!(
        messages.lines.is_empty(),
        "Source hold2 + fade.5 has expired"
    );
}

#[test]
fn repeating_message_restarts_the_two_second_hold_without_extra_lines() {
    let mut messages = UiErrorsData::default();
    messages.add("Out of range.");
    messages.tick(2.25);
    assert!(messages.lines[0].alpha() < 1.0);
    messages.add("Out of range.");
    assert_eq!(messages.lines.len(), 1);
    assert_eq!(messages.lines[0].alpha(), 1.0);
    messages.tick(2.0);
    assert_eq!(messages.lines[0].alpha(), 1.0);
    messages.tick(0.5);
    assert!(messages.lines.is_empty());
}
