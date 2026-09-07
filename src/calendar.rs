use std::collections::VecDeque;
use std::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use game_engine::network_runtime::messages::{MessageReceivers, MessageSenders};
use lightyear::prelude::Message as NetworkMessage;
use shared::protocol::{
    CalendarChannel, CalendarSignupStatusSnapshot, CalendarStateUpdate, QueryCalendar,
    RespondCalendarSignup, ScheduleCalendarEvent,
};

use crate::ipc::{Request, Response};
use crate::network_events::{register_message_handler, register_outgoing_handler};
use crate::status::{
    CalendarEventEntry, CalendarSignupEntry, CalendarSignupStateEntry, CalendarStatusSnapshot,
};

#[derive(Resource, Default)]
pub struct CalendarRuntimeState {
    pending_actions: VecDeque<Action>,
    pending_replies: VecDeque<mpsc::Sender<Response>>,
}

enum Action {
    Query,
    Schedule {
        title: String,
        starts_in_minutes: u32,
        max_signups: u8,
        is_raid: bool,
    },
    Signup {
        event_id: u64,
        status: CalendarSignupStatusSnapshot,
    },
}

pub struct CalendarPlugin;

impl Plugin for CalendarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CalendarRuntimeState>();
        register_outgoing_handler(app, send_pending_actions, |world| {
            !world
                .resource::<CalendarRuntimeState>()
                .pending_actions
                .is_empty()
        });
        register_message_handler::<CalendarStateUpdate, _>(app, receive_calendar_updates, |_| true);
    }
}

pub fn queue_ipc_request(
    runtime: &mut CalendarRuntimeState,
    snapshot: &CalendarStatusSnapshot,
    request: &Request,
    respond: mpsc::Sender<Response>,
) -> bool {
    let action = match request {
        Request::CalendarStatus => {
            let _ = respond.send(Response::Text(format_status(snapshot)));
            return true;
        }
        Request::CalendarQuery => Action::Query,
        Request::CalendarSchedule {
            title,
            starts_in_minutes,
            max_signups,
            is_raid,
        } => Action::Schedule {
            title: title.clone(),
            starts_in_minutes: *starts_in_minutes,
            max_signups: *max_signups,
            is_raid: *is_raid,
        },
        Request::CalendarSignup { event_id, status } => Action::Signup {
            event_id: *event_id,
            status: *status,
        },
        _ => return false,
    };
    runtime.pending_actions.push_back(action);
    runtime.pending_replies.push_back(respond);
    true
}

pub fn queue_query(runtime: &mut CalendarRuntimeState) {
    runtime.pending_actions.push_back(Action::Query);
}

pub fn queue_schedule_action(
    runtime: &mut CalendarRuntimeState,
    title: impl Into<String>,
    starts_in_minutes: u32,
    max_signups: u8,
    is_raid: bool,
) {
    runtime.pending_actions.push_back(Action::Schedule {
        title: title.into(),
        starts_in_minutes,
        max_signups,
        is_raid,
    });
}

pub fn queue_signup_action(
    runtime: &mut CalendarRuntimeState,
    event_id: u64,
    status: CalendarSignupStatusSnapshot,
) {
    runtime
        .pending_actions
        .push_back(Action::Signup { event_id, status });
}

fn send_pending_actions(
    mut runtime: ResMut<CalendarRuntimeState>,
    mut query_senders: MessageSenders<QueryCalendar>,
    mut schedule_senders: MessageSenders<ScheduleCalendarEvent>,
    mut signup_senders: MessageSenders<RespondCalendarSignup>,
) {
    while let Some(action) = runtime.pending_actions.pop_front() {
        let sent = match action {
            Action::Query => send_all(&mut query_senders, QueryCalendar),
            Action::Schedule {
                title,
                starts_in_minutes,
                max_signups,
                is_raid,
            } => send_all(
                &mut schedule_senders,
                ScheduleCalendarEvent {
                    title,
                    starts_at_unix_secs: unix_now_secs()
                        .saturating_add(starts_in_minutes as u64 * 60),
                    max_signups,
                    is_raid,
                },
            ),
            Action::Signup { event_id, status } => send_all(
                &mut signup_senders,
                RespondCalendarSignup { event_id, status },
            ),
        };
        if !sent && let Some(reply) = runtime.pending_replies.pop_front() {
            let _ = reply.send(Response::Error(
                "calendar is unavailable: not connected".into(),
            ));
        }
    }
}

fn send_all<T: Clone + NetworkMessage>(senders: &mut MessageSenders<T>, message: T) -> bool {
    let mut sent = false;
    for mut sender in senders.iter_mut() {
        sender.send::<CalendarChannel>(message.clone());
        sent = true;
    }
    sent
}

fn receive_calendar_updates(
    mut runtime: ResMut<CalendarRuntimeState>,
    mut snapshot: ResMut<CalendarStatusSnapshot>,
    mut receivers: MessageReceivers<CalendarStateUpdate>,
) {
    for receiver in receivers.iter_mut() {
        for update in receiver.receive() {
            apply_calendar_state_update(&mut snapshot, update);
            if let Some(reply) = runtime.pending_replies.pop_front() {
                let response = if let Some(error) = &snapshot.last_error {
                    Response::Error(error.clone())
                } else {
                    Response::Text(format_status(&snapshot))
                };
                let _ = reply.send(response);
            }
        }
    }
}

pub fn apply_calendar_state_update(
    snapshot: &mut CalendarStatusSnapshot,
    update: CalendarStateUpdate,
) {
    if let Some(calendar_snapshot) = update.snapshot {
        snapshot.events = calendar_snapshot
            .events
            .into_iter()
            .map(|event| CalendarEventEntry {
                event_id: event.event_id,
                title: event.title,
                organizer_name: event.organizer_name,
                starts_at_unix_secs: event.starts_at_unix_secs,
                max_signups: event.max_signups,
                is_raid: event.is_raid,
                signups: event
                    .signups
                    .into_iter()
                    .map(|signup| CalendarSignupEntry {
                        character_name: signup.character_name,
                        status: map_signup_status(signup.status),
                    })
                    .collect(),
            })
            .collect();
    }
    snapshot.last_server_message = update.message;
    snapshot.last_error = update.error;
}

pub fn reset_runtime(runtime: &mut CalendarRuntimeState) {
    *runtime = CalendarRuntimeState::default();
}

pub fn pending_action_count(runtime: &CalendarRuntimeState) -> usize {
    runtime.pending_actions.len()
}

pub fn format_relative_start(starts_at_unix_secs: u64) -> String {
    let now = unix_now_secs();
    if starts_at_unix_secs <= now {
        return "Started".into();
    }
    let remaining_secs = starts_at_unix_secs - now;
    let remaining_mins = remaining_secs.div_ceil(60);
    if remaining_mins < 60 {
        return format!("in {remaining_mins}m");
    }
    let hours = remaining_mins / 60;
    let mins = remaining_mins % 60;
    if hours < 24 {
        return format!("in {hours}h {mins}m");
    }
    let days = hours / 24;
    let rem_hours = hours % 24;
    format!("in {days}d {rem_hours}h")
}

pub fn player_signup_status(
    event: &CalendarEventEntry,
    player_name: Option<&str>,
) -> Option<CalendarSignupStateEntry> {
    let player_name = player_name?;
    event
        .signups
        .iter()
        .find(|signup| signup.character_name.eq_ignore_ascii_case(player_name))
        .map(|signup| signup.status)
}

fn map_signup_status(status: CalendarSignupStatusSnapshot) -> CalendarSignupStateEntry {
    match status {
        CalendarSignupStatusSnapshot::Confirmed => CalendarSignupStateEntry::Confirmed,
        CalendarSignupStatusSnapshot::Tentative => CalendarSignupStateEntry::Tentative,
        CalendarSignupStatusSnapshot::Declined => CalendarSignupStateEntry::Declined,
    }
}

fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn format_status(snapshot: &CalendarStatusSnapshot) -> String {
    crate::ipc::format::format_calendar_status(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queued_requests_wait_for_dispatch_then_report_disconnection() {
        let mut app = App::new();
        app.init_resource::<game_engine::network_runtime::messages::ConnectionSender>();
        app.add_plugins(CalendarPlugin)
            .init_resource::<CalendarStatusSnapshot>();
        let (reply, responses) = mpsc::channel();
        {
            let mut runtime = app.world_mut().resource_mut::<CalendarRuntimeState>();
            runtime.pending_actions.extend([
                Action::Query,
                Action::Signup {
                    event_id: 42,
                    status: CalendarSignupStatusSnapshot::Confirmed,
                },
            ]);
            runtime.pending_replies.extend([reply.clone(), reply]);
        }
        app.update();
        assert_eq!(
            app.world()
                .resource::<CalendarRuntimeState>()
                .pending_actions
                .len(),
            2
        );
        assert!(matches!(
            responses.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
        crate::network_events::dispatch_outgoing(app.world_mut());
        for _ in 0..2 {
            let Response::Error(error) = responses.try_recv().unwrap() else {
                panic!("expected disconnected error");
            };
            assert_eq!(error, "calendar is unavailable: not connected");
        }
        let runtime = app.world().resource::<CalendarRuntimeState>();
        assert!(runtime.pending_actions.is_empty());
        assert!(runtime.pending_replies.is_empty());
    }

    #[test]
    fn idle_dispatch_leaves_waiting_replies_untouched() {
        let mut app = App::new();
        app.init_resource::<game_engine::network_runtime::messages::ConnectionSender>();
        app.add_plugins(CalendarPlugin);
        let (reply, responses) = mpsc::channel();
        app.world_mut()
            .resource_mut::<CalendarRuntimeState>()
            .pending_replies
            .push_back(reply);
        // No inbox means no snapshot is needed by incoming handlers.
        crate::network_events::dispatch_incoming(app.world_mut());
        crate::network_events::dispatch_outgoing(app.world_mut());
        assert_eq!(
            app.world()
                .resource::<CalendarRuntimeState>()
                .pending_replies
                .len(),
            1
        );
        assert!(matches!(
            responses.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
    }

    #[test]
    fn calendar_state_update_populates_status_snapshot() {
        let mut snapshot = CalendarStatusSnapshot::default();

        apply_calendar_state_update(
            &mut snapshot,
            CalendarStateUpdate {
                snapshot: Some(shared::protocol::CalendarSnapshot {
                    events: vec![shared::protocol::CalendarEventSnapshot {
                        event_id: 7,
                        title: "Karazhan".into(),
                        organizer_name: "Theron".into(),
                        starts_at_unix_secs: 1_710_000_000,
                        max_signups: 10,
                        is_raid: true,
                        signups: vec![shared::protocol::CalendarSignupSnapshot {
                            character_name: "Alice".into(),
                            status: shared::protocol::CalendarSignupStatusSnapshot::Confirmed,
                        }],
                    }],
                }),
                message: Some("calendar updated".into()),
                error: None,
            },
        );

        assert_eq!(snapshot.events.len(), 1);
        assert_eq!(snapshot.events[0].title, "Karazhan");
        assert_eq!(
            snapshot.events[0].signups[0].status,
            CalendarSignupStateEntry::Confirmed
        );
    }
}
