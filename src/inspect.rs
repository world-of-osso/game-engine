use std::collections::VecDeque;
use std::sync::mpsc;

use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::Player as NetPlayer;
use shared::protocol::{InspectChannel, InspectStateUpdate, QueryInspectTarget};

use crate::ipc::{Request, Response};
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
        app.add_systems(Update, send_pending_queries);
        app.add_systems(Update, receive_inspect_updates);
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
    mut senders: Query<&mut MessageSender<QueryInspectTarget>>,
) {
    if !runtime.pending_query {
        return;
    }

    runtime.pending_query = false;
    let request = QueryInspectTarget {
        target_entity: runtime.current_target.map(|entity| entity.to_bits()),
    };
    if send_all(&mut senders, request) {
        return;
    }

    while let Some(reply) = runtime.pending_replies.pop_front() {
        let _ = reply.send(Response::Error(
            "inspect is unavailable: not connected".into(),
        ));
    }
}

fn send_all<T: Clone + lightyear::prelude::Message>(
    senders: &mut Query<&mut MessageSender<T>>,
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
    mut receivers: Query<&mut MessageReceiver<InspectStateUpdate>>,
) {
    for mut receiver in receivers.iter_mut() {
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
