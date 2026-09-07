use std::collections::VecDeque;
use std::sync::mpsc;

use bevy::prelude::*;
use game_engine::network_runtime::messages::{MessageReceivers, MessageSenders};
use game_engine::network_runtime::replication::ReplicationMirrorMap;
use shared::components::Player as NetPlayer;
use shared::protocol::{InspectChannel, InspectStateUpdate, QueryInspectTarget};

use crate::ipc::{Request, Response};
use crate::network_events::{register_message_handler, register_outgoing_handler};
use crate::status::{InspectStatusSnapshot, TalentNodeEntry, TalentSpecTabEntry};
use crate::targeting::CurrentTarget;

#[derive(Resource, Default)]
pub struct InspectRuntimeState {
    pending_query: bool,
    pending_replies: VecDeque<mpsc::Sender<Response>>,
    current_target: Option<Entity>,
}

pub struct InspectPlugin;

impl Plugin for InspectPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InspectRuntimeState>();
        app.add_systems(Update, sync_target_change);
        register_outgoing_handler(app, send_pending_queries, |world| {
            world.resource::<InspectRuntimeState>().pending_query
        });
        register_message_handler::<InspectStateUpdate, _>(app, receive_inspect_updates, |_| true);
    }
}

pub fn queue_ipc_request(
    runtime: &mut InspectRuntimeState,
    snapshot: &InspectStatusSnapshot,
    request: &Request,
    respond: mpsc::Sender<Response>,
) -> bool {
    match request {
        Request::InspectStatus => {
            let _ = respond.send(Response::Text(format_status(snapshot)));
            true
        }
        Request::InspectQuery => {
            runtime.pending_query = true;
            runtime.pending_replies.push_back(respond);
            true
        }
        _ => false,
    }
}

fn sync_target_change(
    current_target: Res<CurrentTarget>,
    inspectable_targets: Query<(), With<NetPlayer>>,
    mut runtime: ResMut<InspectRuntimeState>,
    mut snapshot: ResMut<InspectStatusSnapshot>,
) {
    if !current_target.is_changed() {
        return;
    }

    let inspectable_target = current_target
        .0
        .filter(|entity| inspectable_targets.contains(*entity));
    if runtime.current_target == current_target.0 {
        return;
    }

    runtime.current_target = current_target.0;
    clear_snapshot(&mut snapshot);
    runtime.pending_query = inspectable_target.is_some();
}

fn clear_snapshot(snapshot: &mut InspectStatusSnapshot) {
    *snapshot = InspectStatusSnapshot::default();
}

fn send_pending_queries(
    mut runtime: ResMut<InspectRuntimeState>,
    mut senders: MessageSenders<QueryInspectTarget>,
    map: Res<ReplicationMirrorMap>,
) {
    if !runtime.pending_query {
        return;
    }

    runtime.pending_query = false;
    let outcome = if senders.is_empty() {
        Err("inspect is unavailable: not connected")
    } else {
        build_inspect_query(runtime.current_target, &map).and_then(|request| {
            send_all(&mut senders, request)
                .then_some(())
                .ok_or("inspect is unavailable: not connected")
        })
    };
    if let Err(error) = outcome {
        while let Some(reply) = runtime.pending_replies.pop_front() {
            let _ = reply.send(Response::Error(error.into()));
        }
    }
}

fn build_inspect_query(
    target: Option<Entity>,
    map: &ReplicationMirrorMap,
) -> Result<QueryInspectTarget, &'static str> {
    let target_entity = target
        .map(|main| {
            map.main_to_server(main)
                .map(Entity::to_bits)
                .ok_or("inspect target is no longer replicated")
        })
        .transpose()?;
    Ok(QueryInspectTarget { target_entity })
}

fn send_all<T: Clone + lightyear::prelude::Message>(
    senders: &mut MessageSenders<T>,
    message: T,
) -> bool {
    let mut sent = false;
    for mut sender in senders.iter_mut() {
        sender.send::<InspectChannel>(message.clone());
        sent = true;
    }
    sent
}

fn receive_inspect_updates(
    mut runtime: ResMut<InspectRuntimeState>,
    mut snapshot: ResMut<InspectStatusSnapshot>,
    mut receivers: MessageReceivers<InspectStateUpdate>,
) {
    for receiver in receivers.iter_mut() {
        for update in receiver.receive() {
            apply_inspect_state_update(&mut snapshot, update);
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

pub fn reset_runtime(runtime: &mut InspectRuntimeState) {
    *runtime = InspectRuntimeState::default();
}

pub fn request_query_for_target(runtime: &mut InspectRuntimeState, target: Option<Entity>) {
    runtime.current_target = target;
    runtime.pending_query = target.is_some();
}

pub fn apply_inspect_state_update(
    snapshot: &mut InspectStatusSnapshot,
    update: InspectStateUpdate,
) {
    if let Some(inspect_snapshot) = update.snapshot {
        snapshot.target_name = Some(inspect_snapshot.target_name);
        snapshot.equipment_appearance = inspect_snapshot.equipment_appearance;
        snapshot.spec_tabs = inspect_snapshot
            .talents
            .spec_tabs
            .into_iter()
            .map(|tab| TalentSpecTabEntry {
                name: tab.name,
                active: tab.active,
            })
            .collect();
        snapshot.talents = inspect_snapshot
            .talents
            .talents
            .into_iter()
            .map(|talent| TalentNodeEntry {
                talent_id: talent.talent_id,
                name: talent.name,
                points_spent: talent.points_spent,
                max_points: talent.max_points,
                active: talent.active,
            })
            .collect();
        snapshot.points_remaining = inspect_snapshot.talents.points_remaining;
    } else {
        clear_snapshot(snapshot);
    }
    snapshot.last_server_message = update.message;
    snapshot.last_error = update.error;
}

fn format_status(snapshot: &InspectStatusSnapshot) -> String {
    let Some(target_name) = snapshot.target_name.as_deref() else {
        return "inspect: none\n-".into();
    };

    let mut lines = vec![format!(
        "inspect: {target_name}\nequipment={} talents={} points_remaining={}",
        snapshot.equipment_appearance.entries.len(),
        active_talent_count(snapshot),
        snapshot.points_remaining
    )];
    push_optional_line(
        &mut lines,
        "message",
        snapshot.last_server_message.as_deref(),
    );
    push_optional_line(&mut lines, "error", snapshot.last_error.as_deref());
    if !snapshot.equipment_appearance.entries.is_empty() {
        lines.push(format_equipment_entries(snapshot));
    }
    lines.join("\n")
}

fn active_talent_count(snapshot: &InspectStatusSnapshot) -> usize {
    snapshot
        .talents
        .iter()
        .filter(|talent| talent.active)
        .count()
}

fn format_equipment_entries(snapshot: &InspectStatusSnapshot) -> String {
    snapshot
        .equipment_appearance
        .entries
        .iter()
        .map(format_equipment_entry)
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_equipment_entry(entry: &shared::components::EquippedAppearanceEntry) -> String {
    format!(
        "{:?} item={} display={} hidden={}",
        entry.slot,
        format_optional_id(entry.item_id),
        format_optional_id(entry.display_info_id),
        entry.hidden
    )
}

fn format_optional_id(value: Option<u32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".into())
}

fn push_optional_line(lines: &mut Vec<String>, label: &str, value: Option<&str>) {
    if let Some(value) = value {
        lines.push(format!("{label}: {value}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removed_inspect_target_replies_without_sending_a_command() {
        use crate::network_runtime::messages::ConnectionSender;
        let mut app = App::new();
        app.init_resource::<ReplicationMirrorMap>();
        app.add_plugins(InspectPlugin);
        let main = app.world_mut().spawn_empty().id();
        let (commands, pending_commands) = mpsc::channel();
        app.insert_resource(ConnectionSender::new(Some(commands)));
        let (reply, responses) = mpsc::channel();
        {
            let mut runtime = app.world_mut().resource_mut::<InspectRuntimeState>();
            request_query_for_target(&mut runtime, Some(main));
            runtime.pending_replies.push_back(reply);
        }
        crate::network_events::dispatch_outgoing(app.world_mut());
        let Response::Error(error) = responses.try_recv().unwrap() else {
            panic!("expected unmapped target error");
        };
        assert_eq!(error, "inspect target is no longer replicated");
        assert!(pending_commands.try_recv().is_err());
        assert!(!app.world().resource::<InspectRuntimeState>().pending_query);
    }

    #[test]
    fn inspect_query_uses_server_identity_and_rejects_removed_mapping() {
        let mut world = World::new();
        let main = world.spawn_empty().id();
        let server = world.spawn_empty().id();
        let mut map = ReplicationMirrorMap::default();
        map.insert(server, main);
        assert_eq!(
            build_inspect_query(Some(main), &map).unwrap().target_entity,
            Some(server.to_bits())
        );
        map.clear();
        assert_eq!(
            build_inspect_query(Some(main), &map).unwrap_err(),
            "inspect target is no longer replicated"
        );
        assert_eq!(build_inspect_query(None, &map).unwrap().target_entity, None);
    }

    #[test]
    fn dispatcher_consumes_pending_query_and_replies_when_disconnected() {
        let mut app = App::new();
        app.init_resource::<game_engine::network_runtime::messages::ConnectionSender>();
        app.init_resource::<ReplicationMirrorMap>();
        app.add_plugins(InspectPlugin);
        let (reply, responses) = mpsc::channel();
        {
            let mut runtime = app.world_mut().resource_mut::<InspectRuntimeState>();
            runtime.pending_query = true;
            runtime.pending_replies.push_back(reply.clone());
            runtime.pending_replies.push_back(reply);
        }
        crate::network_events::dispatch_outgoing(app.world_mut());
        for _ in 0..2 {
            let Response::Error(error) = responses.try_recv().unwrap() else {
                panic!("expected disconnected inspect error");
            };
            assert_eq!(error, "inspect is unavailable: not connected");
        }
        assert!(!app.world().resource::<InspectRuntimeState>().pending_query);
        crate::network_events::dispatch_outgoing(app.world_mut());
        assert!(responses.try_recv().is_err());
    }
    use shared::components::{EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry};

    #[test]
    fn format_status_reports_target_and_equipment() {
        let snapshot = InspectStatusSnapshot {
            target_name: Some("Alice".into()),
            equipment_appearance: EquipmentAppearance {
                entries: vec![EquippedAppearanceEntry {
                    slot: EquipmentVisualSlot::Head,
                    item_id: Some(100),
                    display_info_id: Some(200),
                    inventory_type: 1,
                    hidden: false,
                }],
            },
            spec_tabs: vec![TalentSpecTabEntry {
                name: "Protection".into(),
                active: true,
            }],
            talents: vec![TalentNodeEntry {
                talent_id: 101,
                name: "Divine Strength".into(),
                points_spent: 1,
                max_points: 1,
                active: true,
            }],
            points_remaining: 50,
            last_server_message: Some("inspect ready".into()),
            last_error: None,
        };

        let text = format_status(&snapshot);

        assert!(text.contains("inspect: Alice"));
        assert!(text.contains("equipment=1"));
        assert!(text.contains("talents=1"));
        assert!(text.contains("Head"));
    }
}
