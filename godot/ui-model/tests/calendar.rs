use chrono::{Datelike, Local, NaiveDate, TimeZone};
use game_engine_ui_model::calendar_data::{
    ACTION_CLOSE, ACTION_NEXT_MONTH, ACTION_PREVIOUS_MONTH, ACTION_TOGGLE, ACTION_TOGGLE_FILTER,
    ACTION_TOGGLE_SERVER_EVENTS, CalendarModel,
};
use shared::protocol::{CalendarEventSnapshot, CalendarSnapshot};

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("fixture date is valid")
}

fn date_parts(date: NaiveDate) -> (i32, u32, u32) {
    (date.year(), date.month(), date.day())
}

#[test]
fn calendar_toggle_opens_on_today_and_close_hides_the_panel() {
    let today = date(2026, 11, 11);
    let mut calendar = CalendarModel::default();

    calendar
        .apply_action(ACTION_TOGGLE, date_parts(today))
        .unwrap();
    assert!(calendar.is_open());
    let view = calendar.view().unwrap().unwrap();
    assert_eq!(view.month_name, "November");
    assert_eq!(view.year, 2026);
    assert!(
        view.days
            .iter()
            .any(|day| day.date == today && day.is_today)
    );

    calendar
        .apply_action(ACTION_CLOSE, date_parts(today))
        .unwrap();
    assert!(!calendar.is_open());
    assert!(calendar.view().unwrap().is_none());
}

#[test]
fn calendar_month_navigation_wraps_from_december_to_january_and_back() {
    let today = date(2026, 12, 12);
    let mut calendar = CalendarModel::default();
    calendar
        .apply_action(ACTION_TOGGLE, date_parts(today))
        .unwrap();

    calendar
        .apply_action(ACTION_NEXT_MONTH, date_parts(today))
        .unwrap();
    let january = calendar.view().unwrap().unwrap();
    assert_eq!(
        (january.month_name.as_str(), january.year),
        ("January", 2027)
    );

    calendar
        .apply_action(ACTION_PREVIOUS_MONTH, date_parts(today))
        .unwrap();
    let december = calendar.view().unwrap().unwrap();
    assert_eq!(
        (december.month_name.as_str(), december.year),
        ("December", 2026)
    );
}

#[test]
fn november_2026_grid_starts_on_sunday_and_contains_six_complete_weeks() {
    let today = date(2026, 11, 11);
    let mut calendar = CalendarModel::default();
    calendar
        .apply_action(ACTION_TOGGLE, date_parts(today))
        .unwrap();
    let view = calendar.view().unwrap().unwrap();

    assert_eq!(view.weekdays[0], "Sunday");
    assert_eq!(view.days.len(), 42);
    assert_eq!(view.days[0].date, date(2026, 11, 1));
    assert!(view.days[0].in_viewed_month);
    assert_eq!(view.days[10].date, today);
    assert!(view.days[10].is_today);
    assert_eq!(view.days[29].date, date(2026, 11, 30));
    assert_eq!(view.days[30].date, date(2026, 12, 1));
    assert!(!view.days[30].in_viewed_month);
    assert_eq!(view.days[41].date, date(2026, 12, 12));
}

#[test]
fn calendar_shows_only_server_events_and_the_filter_hides_them() {
    let today = date(2026, 11, 11);
    let event_date = date(2026, 11, 12);
    let local_start = Local
        .from_local_datetime(&event_date.and_hms_opt(12, 0, 0).unwrap())
        .single()
        .unwrap();
    let mut calendar = CalendarModel::default();
    calendar
        .apply_action(ACTION_TOGGLE, date_parts(today))
        .unwrap();
    calendar
        .replace_events(&CalendarSnapshot { events: Vec::new() })
        .unwrap();
    assert!(
        calendar
            .view()
            .unwrap()
            .unwrap()
            .days
            .iter()
            .all(|day| day.events.is_empty())
    );

    calendar
        .replace_events(&CalendarSnapshot {
            events: vec![CalendarEventSnapshot {
                event_id: 7,
                title: "Server raid".into(),
                organizer_name: "Alice".into(),
                starts_at_unix_secs: local_start.timestamp() as u64,
                max_signups: 10,
                is_raid: true,
                signups: Vec::new(),
            }],
        })
        .unwrap();
    let view = calendar.view().unwrap().unwrap();
    let event_day = view.days.iter().find(|day| day.date == event_date).unwrap();
    assert_eq!(event_day.events.len(), 1);
    assert_eq!(event_day.events[0].title, "Server raid");

    calendar
        .apply_action(ACTION_TOGGLE_FILTER, date_parts(today))
        .unwrap();
    assert!(calendar.view().unwrap().unwrap().filter_open);
    calendar
        .apply_action(ACTION_TOGGLE_SERVER_EVENTS, date_parts(today))
        .unwrap();
    let filtered = calendar.view().unwrap().unwrap();
    assert!(!filtered.filter_open);
    assert!(filtered.days.iter().all(|day| day.events.is_empty()));
}
