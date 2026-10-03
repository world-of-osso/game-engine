use game_engine_core::client_options_data::ClientOptionsFile;
use game_engine_core::status_text_data::{
    StatusBarText, StatusTextDisplay, TextStatusBar, abbreviate_large_numbers,
};

fn center(text: &str) -> StatusBarText {
    StatusBarText {
        center: text.into(),
        ..Default::default()
    }
}

fn sides(left: &str, right: &str) -> StatusBarText {
    StatusBarText {
        left: left.into(),
        right: right.into(),
        ..Default::default()
    }
}

const HEALTH: TextStatusBar = TextStatusBar::HEALTH;

#[test]
fn numeric_shows_abbreviated_value_over_max() {
    let text = HEALTH.text(12_345, 15_000, StatusTextDisplay::Numeric, false);
    assert_eq!(text, center("12,345 / 15,000"));
    let text = HEALTH.text(1_234_567, 1_500_000, StatusTextDisplay::Numeric, false);
    assert_eq!(text, center("1234 K / 1500 K"));
}

#[test]
fn percentage_rounds_up_in_double_arithmetic() {
    let percent = |value, max| HEALTH.text(value, max, StatusTextDisplay::Percent, false);
    assert_eq!(percent(12_345, 15_000), center("83%"));
    assert_eq!(percent(15_000, 15_000), center("100%"));
    assert_eq!(percent(1, 15_000), center("1%"));
    // (7 / 100) * 100 is 7.000000000000001 in doubles: Retail's math.ceil shows 8%.
    assert_eq!(percent(7, 100), center("8%"));
}

#[test]
fn both_splits_percent_left_and_current_value_right() {
    let text = HEALTH.text(12_345, 15_000, StatusTextDisplay::Both, false);
    assert_eq!(text, sides("83%", "12,345"));
    let mana = TextStatusBar::power(true).text(0, 60_000, StatusTextDisplay::Both, false);
    assert_eq!(mana, sides("0%", "0"));
}

#[test]
fn both_shows_only_the_value_for_non_mana_power() {
    let rage = TextStatusBar::power(false).text(45, 100, StatusTextDisplay::Both, false);
    assert_eq!(rage, sides("", "45"));
}

#[test]
fn none_hides_text_until_the_bar_is_hovered_then_shows_numeric() {
    assert_eq!(
        HEALTH.text(12_345, 15_000, StatusTextDisplay::None, false),
        StatusBarText::default()
    );
    assert_eq!(
        HEALTH.text(12_345, 15_000, StatusTextDisplay::None, true),
        center("12,345 / 15,000")
    );
}

#[test]
fn zero_power_shows_zero_unless_the_bar_has_zero_text() {
    let mana = TextStatusBar::power(true);
    assert_eq!(
        mana.text(0, 60_000, StatusTextDisplay::Numeric, false),
        center("0 / 60,000")
    );
    assert_eq!(
        mana.text(0, 60_000, StatusTextDisplay::Percent, false),
        center("0%")
    );
    let target = mana.with_zero_text("");
    assert_eq!(
        target.text(0, 60_000, StatusTextDisplay::Numeric, false),
        StatusBarText::default()
    );
}

#[test]
fn no_max_hides_the_text() {
    assert_eq!(
        HEALTH.text(0, 0, StatusTextDisplay::Numeric, true),
        StatusBarText::default()
    );
}

#[test]
fn abbreviation_follows_digit_count() {
    let cases = [
        (0, "0"),
        (999, "999"),
        (1_000, "1,000"),
        (99_999, "99,999"),
        (123_456, "123 K"),
        (12_345_678, "12345 K"),
        (99_999_999, "99999 K"),
        (123_456_789, "123 M"),
        (1_234_567_890, "1234 M"),
    ];
    for (value, expected) in cases {
        assert_eq!(abbreviate_large_numbers(value), expected, "{value}");
    }
}

#[test]
fn status_text_display_saves_as_its_cvar_value_and_defaults_to_none() {
    let saved_before: ClientOptionsFile = ron::from_str("()").unwrap();
    assert_eq!(
        saved_before.hud.status_text_display,
        StatusTextDisplay::None
    );
    let mut file = ClientOptionsFile::default();
    file.hud.status_text_display = StatusTextDisplay::Both;
    let saved = ron::to_string(&file).unwrap();
    assert!(saved.contains("statusTextDisplay:BOTH"), "{saved}");
    let restored: ClientOptionsFile = ron::from_str(&saved).unwrap();
    assert_eq!(restored.hud.status_text_display, StatusTextDisplay::Both);
}
