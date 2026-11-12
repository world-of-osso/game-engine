use std::collections::HashSet;

use chrono::{Datelike, Local, NaiveDate, TimeZone};
use game_engine_ui_model::{
    calendar_data::{
        ACTION_CLOSE, ACTION_NEXT_MONTH, ACTION_PREVIOUS_MONTH, ACTION_TOGGLE,
        ACTION_TOGGLE_FILTER, ACTION_TOGGLE_SERVER_EVENTS, CalendarModel,
    },
    calendar_frame_component::calendar_frame_screen,
};
use shared::protocol::{CalendarEventSnapshot, CalendarSnapshot};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    layout_values::Val,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
    strata::DrawLayer,
    widgets::texture::{BlendMode, TextureSource},
};

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("fixture date is valid")
}

fn date_parts(date: NaiveDate) -> (i32, u32, u32) {
    (date.year(), date.month(), date.day())
}

fn assert_calendar_weekday_tiles_match_retail_grid(registry: &FrameRegistry) {
    const DAY_SIZE: f32 = 91.0;
    const WEEKDAY_HEIGHT: f32 = 28.0;
    const DAY_COLUMNS: usize = 7;
    const GRID_LEFT: f32 = 12.0;
    const WEEKDAY_TOP: f32 = 46.0;
    const TILE_SIZE: f32 = 90.0 / 256.0 - 0.001;
    const TEX_TOP: f32 = 180.0 / 256.0;
    const TEX_HEIGHT: f32 = 28.0 / 256.0 - 0.001;
    const TEXTURE_EPSILON: f32 = 0.000001;

    for index in 0..DAY_COLUMNS {
        let name = format!("CalendarWeekday{}Background", index + 1);
        let frame = registry.get(registry.get_by_name(&name).unwrap()).unwrap();
        let expected_left = GRID_LEFT + index as f32 * DAY_SIZE;
        assert_eq!(frame.width.value(), DAY_SIZE, "{name} width");
        assert_eq!(frame.height.value(), WEEKDAY_HEIGHT, "{name} height");
        assert_eq!(frame.position.left, Val::Px(expected_left));
        assert_eq!(frame.position.top, Val::Px(WEEKDAY_TOP));

        let Some(WidgetData::Texture(texture)) = frame.widget_data.as_ref() else {
            panic!("{name} is not a texture");
        };
        assert_eq!(texture.source, TextureSource::FileDataId(235_428));
        let expected_tex_left = if index % 2 == 0 { TILE_SIZE } else { 0.0 };
        let [left, right, top, bottom] = texture.tex_coords;
        assert!((left - expected_tex_left).abs() < TEXTURE_EPSILON);
        assert!((right - left - TILE_SIZE).abs() < TEXTURE_EPSILON);
        assert!((top - TEX_TOP).abs() < TEXTURE_EPSILON);
        assert!((bottom - top - TEX_HEIGHT).abs() < TEXTURE_EPSILON);
    }
}

fn assert_calendar_day_buttons_match_retail_grid(registry: &FrameRegistry) {
    const DAY_SIZE: f32 = 91.0;
    const DAY_COLUMNS: usize = 7;
    const GRID_LEFT: f32 = 12.0;
    const GRID_TOP: f32 = 74.0;
    const TILE_SIZE: f32 = 90.0 / 256.0 - 0.001;
    const TEXTURE_EPSILON: f32 = 0.000001;

    let mut quadrants = HashSet::new();
    for index in 0..42 {
        let name = format!("CalendarDayButton{}", index + 1);
        let frame = registry.get(registry.get_by_name(&name).unwrap()).unwrap();
        let column = index % DAY_COLUMNS;
        let row = index / DAY_COLUMNS;
        assert_eq!(frame.width.value(), DAY_SIZE, "{name} width");
        assert_eq!(frame.height.value(), DAY_SIZE, "{name} height");
        assert_eq!(
            frame.position.left,
            Val::Px(GRID_LEFT + column as f32 * DAY_SIZE)
        );
        assert_eq!(
            frame.position.top,
            Val::Px(GRID_TOP + row as f32 * DAY_SIZE)
        );

        let Some(WidgetData::Button(button)) = frame.widget_data.as_ref() else {
            panic!("{name} is not a button");
        };
        let texture = button
            .normal_texture
            .as_ref()
            .expect("day button normal texture");
        assert_eq!(texture.source, TextureSource::FileDataId(235_428));
        let [left, right, top, bottom] = texture.tex_coords;
        let quadrant_x = if left > 0.0 { 1 } else { 0 };
        let quadrant_y = if top > 0.0 { 1 } else { 0 };
        assert!((left - quadrant_x as f32 * TILE_SIZE).abs() < TEXTURE_EPSILON);
        assert!((right - left - TILE_SIZE).abs() < TEXTURE_EPSILON);
        assert!((top - quadrant_y as f32 * TILE_SIZE).abs() < TEXTURE_EPSILON);
        assert!((bottom - top - TILE_SIZE).abs() < TEXTURE_EPSILON);
        quadrants.insert((quadrant_x, quadrant_y));

        let event_background = format!("{name}EventBackground");
        assert!(registry.get_by_name(&event_background).is_none());
    }
    assert_eq!(quadrants.len(), 4, "Retail uses all four atlas quadrants");
}

#[test]
fn calendar_frame_emits_retail_grid_and_anchor_art() {
    let today = date(2026, 11, 12);
    let mut calendar = CalendarModel::default();
    calendar
        .apply_action(ACTION_TOGGLE, date_parts(today))
        .unwrap();
    let mut shared = SharedContext::new();
    shared.insert(calendar.view().unwrap().unwrap());
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_thread_skin(ActiveSkin::Forever);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(calendar_frame_screen).sync(&shared, &mut registry);

    let root = registry
        .get(registry.get_by_name("CalendarFrame").unwrap())
        .unwrap();
    assert_eq!(root.width.value(), 659.0);
    assert_eq!(root.height.value(), 624.0);

    let year = registry
        .get(registry.get_by_name("CalendarYearName").unwrap())
        .unwrap();
    let year_background = registry
        .get(registry.get_by_name("CalendarYearBackground").unwrap())
        .unwrap();
    let weekday = registry
        .get(registry.get_by_name("CalendarWeekday4Name").unwrap())
        .unwrap();
    assert_eq!(year.draw_layer, DrawLayer::Artwork);
    assert_eq!(year.position.top, Val::Px(25.0));
    assert_eq!(year_background.position.top, Val::Px(24.0));
    assert_eq!(weekday.draw_layer, DrawLayer::Background);
    assert_calendar_weekday_tiles_match_retail_grid(&registry);
    assert_calendar_day_buttons_match_retail_grid(&registry);

    let today_texture = registry
        .get(registry.get_by_name("CalendarTodayTexture").unwrap())
        .unwrap();
    assert_eq!(today_texture.position.left, Val::Px(349.0));
    assert_eq!(today_texture.position.top, Val::Px(140.5));

    let glow = registry
        .get(registry.get_by_name("CalendarTodayTextureGlow").unwrap())
        .unwrap();
    assert_eq!(glow.position.left, Val::Px(367.5));
    assert_eq!(glow.position.top, Val::Px(130.5));
    match glow.widget_data.as_ref() {
        Some(WidgetData::Texture(texture)) => {
            assert_eq!(texture.blend_mode, BlendMode::Additive);
        }
        other => panic!("today glow is {other:?}"),
    }
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
