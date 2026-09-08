# Local Bevy transform patch

`bevy_transform/` is the unmodified crates.io `bevy_transform` 0.19.0 source at import, retaining its licenses. `Cargo.toml` patches that exact package locally and includes it as a workspace member so its real contention/transform tests use the root build profile and target cache. Other Bevy crates remain registry dependencies.

Purpose: stop the actual empty-queue/active-worker spin. The current full-client sample attributes 19.46% of samples to transform-worker polling; 90.05% of that symbol lands on its lock-retry branch. Evidence: `data/diagnostics/cpu-goal-resumed/current-baseline/transform-hot-instructions-correct-symbol.log`.

Workers wait on a condition variable while the queue is empty and producers remain active. Pending batches and the active-producer count share one mutex, so completion is checked against actual work rather than queued wake tokens. Checked-out batches retire through a drop guard, including during unwinding. Worker count, hierarchy traversal, and batch ownership remain unchanged; no frame cap, production sleep, or worker limit is added.

The earlier empty-wake-token attempt could skip a later propagation pass: the regression expected child translation 23 but retained 13. `completion_wakes_do_not_skip_next_propagation_pass` preserves that boundary. `empty_work_queue_waits_for_busy_worker_without_spinning` requires at most two CPU ticks during 300 ms waiting; the original spin used 29.

Native CPU comparison is required; this patch is not performance proof. Retire the override when an upstream Bevy release removes the measured empty-queue spin and passes the queue wait, transform-lifecycle, and native CPU checks for this workload.
