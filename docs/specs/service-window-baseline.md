# Additive service-window baseline

`--service-window <core|render|continuous>` adds engine services above the [native empty-window baseline](empty-window-baseline.md) without entering normal game startup.

## What it must do

- [x] Require exactly one stage after `--service-window`; reject missing/unknown stages and combinations with native-baseline or game arguments. No implicit core-stage alias. Preserve `--empty-window` and normal startup without diagnostic flags.
- [x] `core`: reuse the native winit `Wait` loop and softbuffer presentation. Add Bevy's task pools, frame counter, clock and core schedules; run one app update at the native loop's `about_to_wait` boundary. Do not introduce timers or continuous redraws.
- [x] `render`: add the minimal blank Camera2d GPU renderer with Bevy's window/event integration and normal pipelined rendering. Use Bevy's desktop event policy (focused reactive timeout five seconds, unfocused low-power timeout sixty seconds), not continuous frames. Preserve default Bevy task pools and the game's default Mailbox presentation, with no FPS limiter.
- [x] `continuous`: retain the same renderer, camera, presentation and pools, changing only to Bevy's normal game event policy (focused continuous updates). Record actual update rate together with CPU; this deliberately adds frame work and is not a same-throughput optimization.
- [x] Keep default Bevy pool availability. Do not initialize project game assets, networking, IPC, sound or UI; skip normal game initialization and resource-limit setup. `core` has no GPU renderer; render stages initialize only assets required by Bevy's blank rendering path.
- [x] Keep a distinct visible window title, native resize/close behavior and explicit error reporting. Log update 1 and every 64 updates without installing a polling diagnostic service.
- [x] Measure focused idle process/thread CPU and whole-host CPU separately, with window dimensions and GPU/clock/limit context. All retained twelve-second windows had 13/13 focused samples at 1280×1198: native 0 ticks/one thread; core 0.16664% one core/25 threads; reactive blank GPU 0.08331%/28 threads; continuous blank GPU 216.31478%/28 threads. Reactive telemetry was 0.2 updates/s; continuous mean was 1212.906 updates/s, range 1071.767–1306.683. This is update rate, not presented FPS. Whole-host 5.799%, 5.407%, 7.165%, and 21.071% respectively is not single-process attribution. The continuous stage differs only in Winit policy; Mailbox, pools, pipelining, renderer, camera and no FPS limiter remain fixed. Raw GPU metrics recorded average graphics activity 0–1 reactive versus 80–86 continuous and average GPU clocks 618–682 versus 1945–2326 MHz. Independent source verification and runtime audit 248 pass the bounded finding: continuous blank updates are the first major CPU rise before project services. No presented FPS, frame pacing, render-thread/per-thread CPU attribution, or hardware-counter proof exists.

## Optional CPU attribution

GPU stages honor the existing `cpu-system-profile` feature and `WOO_CPU_PROFILE_OUTPUT` environment variable. With both enabled, export named system-span thread CPU through the existing profiler; without opt-in, preserve default logging. Keep pools, pipelining, presentation and update policy unchanged. Enable the existing `bevy_transform` crate's `trace` feature through an optional direct dependency: Bevy 0.19's aggregate tracing feature omits it. This adds no new package or normal-build feature. Include Bevy's existing `producer_mark_dirty`, `consumer_mark_dirty` and `par_traversal_mark_dirty` spans to measure dirty-tree worker CPU on the threads executing it. Keep worker CPU separate from its parent system's thread CPU; span entries count task polls, not necessarily distinct tasks. Named spans do not cover all executor, driver or uninstrumented work. Measure profiling overhead separately; update counts are not presented frames.

## Implementation and tests

- `src/service_window.rs` constructs the core app using `MinimalPlugins` without `ScheduleRunnerPlugin`.
- `src/empty_window.rs` owns native presentation and calls the optional app at event-loop boundaries.
- `src/main.rs` routes before ordinary initialization; `src/cli_args.rs` enforces exclusive stage selection.
- `src/render_window.rs` installs Bevy logging, transforms, input, accessibility resources, window/assets/event integration, renderer, image/mesh assets, camera, pipelining and core render pipelines. Input messages are required by Winit's keyboard-focus systems and accessibility resources by its window creation; mesh assets are required by RenderPlugin's mesh extraction even for a clear-only camera. No PBR, lights, sprites or UI plugins.
- Render stages log cumulative updates and interval update rate at most once per second during existing app updates, without a separate timer. Update rate is not presented/display FPS.
- `tests/unit/empty_window_args_tests.rs` exercises stage selection and invalid combinations; the service module test executes updates and checks time advancement. The render module test executes Startup and checks one active Camera2d with the specified clear color. Native smoke/measurement must also exercise the actual GPU/plugin lifecycle.

## Out of scope

Project service registration and full-game optimization remain separate work. No new dependency packages, normal startup changes, CPU restrictions or artificial FPS caps.
