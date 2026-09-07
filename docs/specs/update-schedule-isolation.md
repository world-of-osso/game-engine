# Whole-Update diagnostic

`--skip-update-after SECONDS` removes main `Update` from `MainScheduleOrder` after the deadline. This intentionally freezes gameplay and any networking/IPC/UI callbacks in that schedule, while other schedules and rendering remain enabled. It is destructive attribution, not an optimization.

`--skip-main-work-after SECONDS` is the broader diagnostic: it removes `PreUpdate`, `Update`, and `PostUpdate` together. `First`, `Last`, fixed schedules, state transitions, extraction, and rendering remain scheduled. `--skip-pre-update-after` and `--skip-post-update-after` select only their named schedule for the next bisection. All cutoff flags are mutually exclusive. This also freezes camera/transform preparation and network processing in those stages; only frozen-scene attribution is valid.

## What it must do

- [ ] Remove only Update once after the deadline; a zero deadline permits the first update.
- [ ] Preserve other main schedules and startup order.
- [ ] Count app updates independently in Last and log elapsed time/counts once per second.
- [ ] Reject missing, invalid, duplicate deadlines and an already-absent Update label.
- [ ] Remain inactive without the flag. Continuous blank renderer accepts this diagnostic tail.

## How it works

- [CPU investigation](../wiki/investigations/empty-window-baseline.md).

## Implementation inventory

- `src/update_schedule_isolation.rs`: cutoff, independent counter, behavioral tests.
- `src/main.rs`: normal-client configuration.
- `src/render_window.rs`: blank-client configuration.
- `src/cli_args.rs`: diagnostic routing.

## Tests asserting this spec

- [x] Initial implementation behavioral proof: 5/5 tests passed.
- `src/update_schedule_isolation.rs::tests` exercises real App updates before and after cutoff.

## Known gaps

Runtime cutoffs are documented in [[empty-window-baseline]]; no additional test gate is introduced here.

## Out of scope

Equivalent-work savings, deployment, CPU/FPS restrictions, worker/pipeline changes, and profiler testing.
