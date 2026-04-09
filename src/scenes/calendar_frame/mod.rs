use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::calendar::{
    CalendarRuntimeState, format_relative_start, player_signup_status, queue_query,
    queue_schedule_action, queue_signup_action,
};
use game_engine::status::{
    CalendarEventEntry, CalendarSignupStateEntry, CalendarStatusSnapshot, CharacterStatsSnapshot,
};
use game_engine::ui::input::find_frame_at;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::calendar_frame_component::{
    ACTION_CALENDAR_CLOSE, ACTION_CALENDAR_REFRESH, ACTION_CALENDAR_SCHEDULE_PARTY,
    ACTION_CALENDAR_SCHEDULE_RAID, ACTION_CALENDAR_SELECT_PREFIX, ACTION_CALENDAR_SIGNUP_PREFIX,
    ACTION_CALENDAR_TOGGLE, CalendarDetailState, CalendarEventRow, CalendarFrameState,
    CalendarSignupRow, calendar_frame_screen,
};
use shared::protocol::CalendarSignupStatusSnapshot;
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game_state::GameState;
use crate::ui_input::walk_up_for_onclick;

#[derive(Resource, Default)]
pub struct CalendarFrameOpen(pub bool);

#[derive(Resource, Default, Clone, PartialEq, Eq)]
struct CalendarFrameSelection {
    selected_event_id: Option<u64>,
}

struct CalendarFrameRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for CalendarFrameRes {}
unsafe impl Sync for CalendarFrameRes {}

#[derive(Resource)]
struct CalendarFrameWrap(CalendarFrameRes);

#[derive(Resource, Clone, PartialEq)]
struct CalendarFrameModel(CalendarFrameState);

pub struct CalendarFramePlugin;

impl Plugin for CalendarFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CalendarFrameOpen>();
        app.init_resource::<CalendarFrameSelection>();
        app.add_systems(OnEnter(GameState::InWorld), build_calendar_frame_ui);
        app.add_systems(OnExit(GameState::InWorld), teardown_calendar_frame_ui);
        app.add_systems(
            Update,
            (sync_calendar_frame_state, handle_calendar_frame_input)
                .run_if(in_state(GameState::InWorld)),
        );
    }
}

fn build_calendar_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    snapshot: Option<Res<CalendarStatusSnapshot>>,
    character_stats: Option<Res<CharacterStatsSnapshot>>,
    open: Res<CalendarFrameOpen>,
    selection: Res<CalendarFrameSelection>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = build_state(
        snapshot.as_deref(),
        character_stats.as_deref(),
        &open,
        &selection,
    );
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(calendar_frame_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(CalendarFrameWrap(CalendarFrameRes { screen, shared }));
    commands.insert_resource(CalendarFrameModel(state));
}

fn teardown_calendar_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<CalendarFrameWrap>>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<CalendarFrameWrap>();
    commands.remove_resource::<CalendarFrameModel>();
}

fn sync_calendar_frame_state(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<CalendarFrameWrap>>,
    mut last_model: Option<ResMut<CalendarFrameModel>>,
    snapshot: Option<Res<CalendarStatusSnapshot>>,
    character_stats: Option<Res<CharacterStatsSnapshot>>,
    open: Res<CalendarFrameOpen>,
    selection: Res<CalendarFrameSelection>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let state = build_state(
        snapshot.as_deref(),
        character_stats.as_deref(),
        &open,
        &selection,
    );
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

fn handle_calendar_frame_input(
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    ui: Res<UiState>,
    snapshot: Option<Res<CalendarStatusSnapshot>>,
    mut open_state: ResMut<CalendarFrameOpen>,
    mut selection: ResMut<CalendarFrameSelection>,
    mut runtime: ResMut<CalendarRuntimeState>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
        return;
    }
    let Some(mouse) = mouse else { return };
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Some(frame_id) = find_frame_at(&ui.registry, cursor.x, cursor.y) else {
        return;
    };
    let Some(action) = walk_up_for_onclick(&ui.registry, frame_id) else {
        return;
    };
    if !open_state.0 && action != ACTION_CALENDAR_TOGGLE {
        return;
    }
    dispatch_action(
        &action,
        snapshot.as_deref(),
        open_state.as_mut(),
        selection.as_mut(),
        runtime.as_mut(),
    );
}

fn build_state(
    snapshot: Option<&CalendarStatusSnapshot>,
    character_stats: Option<&CharacterStatsSnapshot>,
    open: &CalendarFrameOpen,
    selection: &CalendarFrameSelection,
) -> CalendarFrameState {
    let events = snapshot.map_or_else(Vec::new, |snapshot| snapshot.events.clone());
    let selected_event = resolve_selected_event(&events, selection.selected_event_id);
    CalendarFrameState {
        visible: open.0,
        events: events
            .iter()
            .map(|event| map_event_row(event, selected_event.map(|event| event.event_id)))
            .collect(),
        detail: selected_event.map(|event| {
            build_detail_state(
                event,
                character_stats.and_then(|stats| stats.name.as_deref()),
            )
        }),
        status_text: snapshot
            .and_then(|snapshot| {
                snapshot
                    .last_error
                    .clone()
                    .or_else(|| snapshot.last_server_message.clone())
            })
            .unwrap_or_default(),
        empty_text: if open.0 && events.is_empty() {
            Some("No scheduled events yet.".into())
        } else {
            None
        },
    }
}

fn resolve_selected_event<'a>(
    events: &'a [CalendarEventEntry],
    selected_event_id: Option<u64>,
) -> Option<&'a CalendarEventEntry> {
    selected_event_id
        .and_then(|event_id| events.iter().find(|event| event.event_id == event_id))
        .or_else(|| events.first())
}

fn map_event_row(event: &CalendarEventEntry, selected_event_id: Option<u64>) -> CalendarEventRow {
    let confirmed = count_signups(event, CalendarSignupStateEntry::Confirmed);
    CalendarEventRow {
        title: event.title.clone(),
        schedule_text: format_relative_start(event.starts_at_unix_secs),
        counts_text: format!("{confirmed}/{} confirmed", event.max_signups),
        active: Some(event.event_id) == selected_event_id,
        action: format!("{ACTION_CALENDAR_SELECT_PREFIX}{}", event.event_id),
    }
}

fn build_detail_state(
    event: &CalendarEventEntry,
    local_player_name: Option<&str>,
) -> CalendarDetailState {
    let confirmed = count_signups(event, CalendarSignupStateEntry::Confirmed);
    let tentative = count_signups(event, CalendarSignupStateEntry::Tentative);
    let declined = count_signups(event, CalendarSignupStateEntry::Declined);
    let player_status = player_signup_status(event, local_player_name)
        .map(signup_status_label)
        .unwrap_or("Not signed up");
    CalendarDetailState {
        title: event.title.clone(),
        organizer: format!("Organizer: {}", event.organizer_name),
        schedule_text: format!(
            "Starts: {}",
            format_relative_start(event.starts_at_unix_secs)
        ),
        type_text: format!("Type: {}", if event.is_raid { "Raid" } else { "Party" }),
        signup_text: format!(
            "Signups: {confirmed}/{} confirmed, {tentative} tentative, {declined} declined",
            event.max_signups
        ),
        player_status_text: format!("Your signup: {player_status}"),
        signups: event
            .signups
            .iter()
            .map(|signup| CalendarSignupRow {
                name: signup.character_name.clone(),
                status_text: signup_status_label(signup.status).into(),
            })
            .collect(),
    }
}

fn count_signups(event: &CalendarEventEntry, status: CalendarSignupStateEntry) -> usize {
    event
        .signups
        .iter()
        .filter(|signup| signup.status == status)
        .count()
}

fn signup_status_label(status: CalendarSignupStateEntry) -> &'static str {
    match status {
        CalendarSignupStateEntry::Confirmed => "Confirmed",
        CalendarSignupStateEntry::Tentative => "Tentative",
        CalendarSignupStateEntry::Declined => "Declined",
    }
}

fn dispatch_action(
    action: &str,
    snapshot: Option<&CalendarStatusSnapshot>,
    open: &mut CalendarFrameOpen,
    selection: &mut CalendarFrameSelection,
    runtime: &mut CalendarRuntimeState,
) {
    match action {
        ACTION_CALENDAR_TOGGLE => {
            open.0 = !open.0;
            if open.0 {
                queue_query(runtime);
                selection.selected_event_id = snapshot
                    .and_then(|snapshot| snapshot.events.first().map(|event| event.event_id));
            }
        }
        ACTION_CALENDAR_CLOSE => open.0 = false,
        ACTION_CALENDAR_REFRESH => queue_query(runtime),
        ACTION_CALENDAR_SCHEDULE_RAID => {
            queue_schedule_action(runtime, "Raid Group", 60, 20, true);
        }
        ACTION_CALENDAR_SCHEDULE_PARTY => {
            queue_schedule_action(runtime, "Party Group", 30, 5, false);
        }
        _ => {
            if let Some(event_id) = parse_event_id(action, ACTION_CALENDAR_SELECT_PREFIX) {
                selection.selected_event_id = Some(event_id);
                return;
            }
            if let Some(status) = parse_signup_status(action) {
                if let Some(event_id) = selected_event_id(snapshot, selection.selected_event_id) {
                    queue_signup_action(runtime, event_id, status);
                }
            }
        }
    }
}

fn selected_event_id(
    snapshot: Option<&CalendarStatusSnapshot>,
    selected_event_id: Option<u64>,
) -> Option<u64> {
    selected_event_id.or_else(|| {
        snapshot.and_then(|snapshot| snapshot.events.first().map(|event| event.event_id))
    })
}

fn parse_event_id(action: &str, prefix: &str) -> Option<u64> {
    action.strip_prefix(prefix)?.parse().ok()
}

fn parse_signup_status(action: &str) -> Option<CalendarSignupStatusSnapshot> {
    match action.strip_prefix(ACTION_CALENDAR_SIGNUP_PREFIX)? {
        "confirmed" => Some(CalendarSignupStatusSnapshot::Confirmed),
        "tentative" => Some(CalendarSignupStatusSnapshot::Tentative),
        "declined" => Some(CalendarSignupStatusSnapshot::Declined),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_snapshot() -> CalendarStatusSnapshot {
        CalendarStatusSnapshot {
            events: vec![CalendarEventEntry {
                event_id: 7,
                title: "Karazhan".into(),
                organizer_name: "Theron".into(),
                starts_at_unix_secs: u64::MAX,
                max_signups: 10,
                is_raid: true,
                signups: vec![game_engine::status::CalendarSignupEntry {
                    character_name: "Theron".into(),
                    status: CalendarSignupStateEntry::Confirmed,
                }],
            }],
            last_server_message: Some("calendar refreshed".into()),
            last_error: None,
        }
    }

    #[test]
    fn build_state_maps_calendar_events() {
        let snapshot = sample_snapshot();
        let character_stats = CharacterStatsSnapshot {
            name: Some("Theron".into()),
            ..Default::default()
        };

        let state = build_state(
            Some(&snapshot),
            Some(&character_stats),
            &CalendarFrameOpen(true),
            &CalendarFrameSelection {
                selected_event_id: Some(7),
            },
        );

        assert_eq!(state.events.len(), 1);
        assert!(state.detail.is_some());
        assert_eq!(
            state
                .detail
                .as_ref()
                .map(|detail| detail.player_status_text.as_str()),
            Some("Your signup: Confirmed")
        );
    }

    #[test]
    fn toggle_action_opens_and_queues_query() {
        let mut open = CalendarFrameOpen(false);
        let mut selection = CalendarFrameSelection::default();
        let mut runtime = CalendarRuntimeState::default();
        let snapshot = sample_snapshot();

        dispatch_action(
            ACTION_CALENDAR_TOGGLE,
            Some(&snapshot),
            &mut open,
            &mut selection,
            &mut runtime,
        );

        assert!(open.0);
        assert_eq!(selection.selected_event_id, Some(7));
        assert_eq!(game_engine::calendar::pending_action_count(&runtime), 1);
    }
}
