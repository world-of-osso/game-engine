use super::*;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn make_state() -> CalendarFrameState {
    CalendarFrameState {
        visible: true,
        events: vec![CalendarEventRow {
            title: "Karazhan".into(),
            schedule_text: "in 1h 0m".into(),
            counts_text: "1/10 confirmed".into(),
            active: true,
            action: format!("{ACTION_CALENDAR_SELECT_PREFIX}7"),
        }],
        detail: Some(CalendarDetailState {
            title: "Karazhan".into(),
            organizer: "Organizer: Theron".into(),
            schedule_text: "Starts: in 1h 0m".into(),
            type_text: "Type: Raid".into(),
            signup_text: "Signups: 1/10 confirmed, 0 tentative, 0 declined".into(),
            player_status_text: "Your signup: Confirmed".into(),
            signups: vec![CalendarSignupRow {
                name: "Theron".into(),
                status_text: "Confirmed".into(),
            }],
        }),
        status_text: "calendar refreshed".into(),
        empty_text: None,
    }
}

#[test]
fn calendar_frame_builds_core_frames() {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(make_state());
    Screen::new(calendar_frame_screen).sync(&shared, &mut registry);

    assert!(registry.get_by_name("CalendarFrame").is_some());
    assert!(registry.get_by_name("CalendarEventList").is_some());
    assert!(registry.get_by_name("CalendarDetailPanel").is_some());
}

#[test]
fn calendar_frame_renders_event_rows_and_signup_buttons() {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(make_state());
    Screen::new(calendar_frame_screen).sync(&shared, &mut registry);

    assert!(registry.get_by_name("CalendarEventRow0").is_some());
    assert!(registry.get_by_name("CalendarConfirmButton").is_some());
    assert!(registry.get_by_name("CalendarTentativeButton").is_some());
    assert!(registry.get_by_name("CalendarDeclineButton").is_some());
    let row = registry
        .get(
            registry
                .get_by_name("CalendarEventRow0")
                .expect("CalendarEventRow0"),
        )
        .expect("row");
    assert_eq!(row.onclick.as_deref(), Some("calendar_select:7"));
}
