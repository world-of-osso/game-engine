# Temporary task-submission batching experiment

User-approved experiment in `vendor/bevy_ecs` and `vendor/bevy_tasks` tests whether bulk submission reduces executor coordination cost. [Native evidence](../wiki/investigations/empty-window-baseline.md#native-instruction-attribution) motivates the experiment; it does not establish an optimization.

## What it must do

- [ ] Retain every system and its execution count, dependency ordering, run conditions, and deferred-command visibility.
- [ ] Keep each system independently runnable, with normal worker pools, non-Send/exclusive execution paths, per-system completion, and panic propagation.
- [ ] Provide baseline and batched submission in the same feature configuration; do not alter CPU availability, pipelining, renderer behavior, or compiler optimization settings.
- [ ] Measure engine CPU per update and throughput in bounded 10-second samples, without changing focus or testing the profiler.
- [ ] Reject batching if behavior changes or throughput falls; do not present clock-confounded observations as causal improvements.

## How it works

- [Native instruction attribution](../wiki/investigations/empty-window-baseline.md#native-instruction-attribution).

## Implementation inventory

- `Cargo.toml`: temporary local patches for unchanged Bevy 0.19.0 package versions.
- `vendor/bevy_tasks/src/task_pool.rs`: scoped task submission.
- `vendor/bevy_tasks/src/executor.rs`: executor backend adapter.
- `vendor/bevy_ecs/src/schedule/executor/multi_threaded.rs`: ready-system dispatch.

## Tests asserting this spec

- Pending scoped-task and scheduler behavioral tests alongside patched sources.

## Known gaps (current cycle)

- [ ] Implement submission batching and prove scheduler/task-scope semantics.
- [ ] Build and compare baseline/batched engine behavior and CPU/update.

## Out of scope

Profiler tests, serial system-body fusion, worker/FPS limits, compiler optimization experiments, unrelated fixes, and production deployment.
