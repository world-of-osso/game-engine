use shared::protocol::DurabilityStateUpdate;

use crate::status::{DurabilityEntry, DurabilityStatusSnapshot};

pub fn apply_durability_state_update(
    snapshot: &mut DurabilityStatusSnapshot,
    update: DurabilityStateUpdate,
) {
    if let Some(durability) = update.snapshot {
        snapshot.entries = durability
            .slots
            .into_iter()
            .map(|entry| DurabilityEntry {
                slot: entry.slot,
                current: entry.current,
                max: entry.max,
                repair_cost: entry.repair_cost,
            })
            .collect();
        snapshot.total_repair_cost = durability.total_repair_cost;
    }
    snapshot.last_server_message = update.message;
    snapshot.last_error = update.error;
}

pub fn reset_durability_status(snapshot: &mut DurabilityStatusSnapshot) {
    *snapshot = DurabilityStatusSnapshot::default();
}
