# Additive service-window baseline

`--service-window` adds engine services to the [native empty-window baseline](empty-window-baseline.md) without entering normal game startup.

## What it must do

- [x] Require `--service-window` alone; reject combinations with native-baseline or game arguments. Preserve normal startup without either diagnostic flag.
- [x] Reuse the native winit `Wait` loop and softbuffer presentation. Add Bevy's task pools, frame counter, clock and core schedules; run one app update at the native loop's `about_to_wait` boundary. Do not introduce timers or continuous redraws.
- [x] Keep default Bevy pool availability. Do not initialize game assets, networking, IPC, sound, UI or the GPU renderer; skip normal game initialization and resource-limit setup.
- [x] Keep a distinct visible window title, native resize/close behavior and explicit error reporting. Log update 1 and every 64 updates without installing a polling diagnostic service.
- [ ] Measure focused idle process/thread CPU and whole-host CPU separately, with window dimensions and GPU/clock/limit context. Record the event-driven workload; this is not a rendered-FPS optimization or a comparison against the full game.

## Implementation and tests

- `src/service_window.rs` constructs the core app using `MinimalPlugins` without `ScheduleRunnerPlugin`.
- `src/empty_window.rs` owns native presentation and calls the optional app at event-loop boundaries.
- `src/main.rs` routes before ordinary initialization; `src/cli_args.rs` enforces exclusive selection.
- `tests/unit/empty_window_args_tests.rs` exercises selection and invalid combinations; the service module test executes updates and checks time advancement.

## Out of scope

Continuous game updates, renderer initialization and project service registration are subsequent measured layers, not part of this core-service stage. No dependency additions or normal startup changes.
