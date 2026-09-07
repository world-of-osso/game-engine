# Retired task-submission batching experiment

The temporary September 7, 2026 experiment tested bulk registration of independent ECS tasks, not event batching or serial execution of callback bodies. The dependency patches were removed after measurement: no causal performance improvement was established. Stock Bevy 0.19.0 dependencies are restored.

## What it had to do

- Preserve systems, access checks, conditions, dependencies, non-Send/exclusive paths, deferred commands, and per-system completion.
- Preserve worker pools, CPU availability, pipelining, and rendering; never substitute lower throughput for an efficiency gain.
- Compare the same binary with baseline/batched submission using bounded engine samples, without profiler tests or focus actions.

## How it worked

- [Experiment and measured limitations](../wiki/investigations/empty-window-baseline.md#retired-task-submission-batching-experiment).

## Implementation inventory

No active batching implementation or environment control remains. Historical source is retained in commits `2a818246`, `9b68de16`, `876fa5db`, and `fa15321b`; initial vendor import was `bc827e0f`.

## Historical test evidence

- Scoped submission tests: 3 passed, covering borrowed results, independent progress, and panic cleanup.
- ECS targeted tests at `876fa5db`: 5 passed, covering conditions/dependencies, conflicting access, deferred/exclusive/non-Send work, panic propagation, and configuration.
- `fa15321b` corrected backend gates and expanded multi-batch coverage; those follow-up tests were not run before retirement. The measured Linux binary predates this cfg/test-only follow-up.

## Remaining gaps

The experiment did not establish whole-game performance improvement or explain the bulk CPU cost. It did not test callback removal, callback-body fusion, or batching events.

## Out of scope

Profiler tests, worker/FPS limits, compiler optimization experiments, unrelated fixes, and deployment.
