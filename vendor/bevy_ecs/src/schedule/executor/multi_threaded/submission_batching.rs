use arrayvec::ArrayVec;
use std::env::{self, VarError};

use super::{Context, ExecutorState, MultiThreadedExecutor};

// Keep pending indices on the stack and bound each active-task-lock hold.
pub(super) type PendingSubmissions = ArrayVec<usize, 32>;
const SETTING: &str = "BEVY_ECS_BATCH_TASK_SUBMISSIONS";

impl MultiThreadedExecutor {
    /// Enables bulk submission of independently runnable Send systems.
    ///
    /// System bodies, access checks, completion notifications, and worker pools
    /// remain unchanged. This overrides `BEVY_ECS_BATCH_TASK_SUBMISSIONS` for
    /// this executor.
    pub fn with_task_submission_batching(mut self, enabled: bool) -> Self {
        self.state.get_mut().unwrap().batch_task_submissions = enabled;
        self
    }
}

pub(super) fn read_enabled() -> bool {
    match env::var(SETTING) {
        Ok(value) => parse_enabled(Some(&value)),
        Err(VarError::NotPresent) => parse_enabled(None),
        Err(error) => panic!("Cannot read {SETTING}: {error}"),
    }
}

fn parse_enabled(value: Option<&str>) -> bool {
    match value {
        None | Some("0") => false,
        Some("1") => true,
        Some(value) => panic!("{SETTING} must be 0 or 1, got {value:?}"),
    }
}

/// # Safety
/// The system must be marked running after its access and condition checks.
/// It must be Send and non-exclusive, with no outstanding system reference.
pub(super) unsafe fn enqueue(
    context: &Context,
    pending: &mut PendingSubmissions,
    system_index: usize,
) {
    pending.push(system_index);
    if pending.is_full() {
        // SAFETY: Every index was added under this function's safety contract.
        unsafe { submit(context, pending) };
    }
}

/// # Safety
/// All pending systems must satisfy `enqueue`'s reservation requirements.
pub(super) unsafe fn submit(context: &Context, pending: &mut PendingSubmissions) {
    if pending.is_empty() {
        return;
    }
    let context = *context;
    let indices = core::mem::take(pending);
    let tasks = indices.into_iter().map(move |system_index| {
        // SAFETY: The caller reserved each system through the existing scheduler.
        unsafe { ExecutorState::run_system_task(context, system_index) }
    });
    context.scope.spawn_many(tasks);
}

#[cfg(test)]
mod tests {
    use super::parse_enabled;

    #[test]
    fn submission_batch_setting_accepts_only_baseline_or_enabled() {
        assert!(!parse_enabled(None));
        assert!(!parse_enabled(Some("0")));
        assert!(parse_enabled(Some("1")));
        assert!(std::panic::catch_unwind(|| parse_enabled(Some("true"))).is_err());
    }
}
