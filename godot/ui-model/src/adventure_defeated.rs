//! Saved instance state backing Retail boss defeated overlays.
use shared::protocol::InstanceLockInfo;

use crate::adventure_guide::View;

impl View {
    /// Replace the received raid-lock view, subtracting only local snapshot age.
    pub fn replace_saved_instances(&mut self, locks: &[InstanceLockInfo], elapsed_secs: u64) {
        self.saved_instances = locks
            .iter()
            .filter(|lock| lock.locked && u64::from(lock.time_remaining_secs) > elapsed_secs)
            .map(|lock| {
                let remaining = u64::from(lock.time_remaining_secs) - elapsed_secs;
                InstanceLockInfo {
                    time_remaining_secs: remaining as u32,
                    ..*lock
                }
            })
            .collect();
        self.lockouts = self
            .saved_instances
            .iter()
            .map(|lock| (lock.map_id, saved_summary(lock)))
            .collect();
    }
}

fn saved_summary(lock: &InstanceLockInfo) -> String {
    format!(
        "Saved · difficulty {} · {} bosses defeated · resets in {} minutes",
        lock.difficulty_id,
        lock.completed_mask.count_ones(),
        u64::from(lock.time_remaining_secs).div_ceil(60),
    )
}
