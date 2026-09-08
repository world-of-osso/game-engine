# Local Bevy transform patch

`bevy_transform/` is the unmodified crates.io `bevy_transform` 0.19.0 source at import, retaining its licenses. `Cargo.toml` patches that exact package locally and includes it as a workspace member so its real contention/transform tests use the root build profile and target cache. Other Bevy crates remain registry dependencies.

Purpose: stop the actual empty-queue/active-worker spin. The current full-client sample attributes 19.46% of samples to transform-worker polling; 90.05% of that symbol lands on its lock-retry branch. Evidence: `data/diagnostics/cpu-goal-resumed/current-baseline/transform-hot-instructions-correct-symbol.log`.

Workers now block for a queue batch. The final busy worker sends one empty wake batch per worker, allowing every waiter to observe completion. This preserves worker count, queue/batch ownership, hierarchy traversal, and completion ordering; it adds no frame cap, sleep, or worker limit. `empty_work_queue_waits_for_busy_worker_without_spinning` reproduces the former 29 CPU ticks over 300 ms and requires at most two ticks while preserving wake/exit behavior.

Native CPU comparison is required; this patch is not performance proof. Retire the override when an upstream Bevy release removes the measured empty-queue spin and passes the queue wait, transform-lifecycle, and native CPU checks for this workload.
