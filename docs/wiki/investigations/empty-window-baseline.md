# Empty-window baseline

`--empty-window` provides a native Wayland window/event-loop baseline outside the game runtime. It measures external idle CPU separately from Bevy/game integration. The initial native baseline is verified below; it does not identify the full game's CPU cause.

## Runtime boundary

`main` checks the complete argument list before normal startup. The mode accepts only `--empty-window`; any extra argument is an error. A successful selection calls `empty_window::run()` then returns before thread-pool configuration, asset-root discovery, resource limits, simple-flag handling, CLI parsing, and `run_app`.

The mode therefore does not construct a Bevy `App`, register plugins, initialize game assets, networking, IPC, renderer plugins, or game task pools. Normal invocations do not select this branch.

## Native window behavior

`empty_window::run()` creates a winit event loop with `ControlFlow::Wait`, a softbuffer context, and one application handler. On resume it creates one window and requests its first redraw.

Painting occurs only for `RedrawRequested`. Resize and scale-factor events request another redraw. The paint path resizes the surface, fills it with a flat background, notifies the window before presentation, then presents. It requests no application timer, periodic redraw, update loop, or continuous rendering. Zero-size windows return before a surface resize or buffer acquisition. Close exits the event loop. Window, surface, context, event-loop, and presentation errors include context and return a nonzero process status through `main`.

Linux window attributes set the Wayland application name. The direct softbuffer dependency enables only `wayland` and `wayland-dlopen`; this diagnostic does not provide X11 or KMS painting. `winit` is reused as a direct dependency for its public event-loop API. `softbuffer` is the only added dependency, used for a shared-memory surface without starting a GPU renderer. Existing locked versions remain unchanged.

## Additive Bevy-core stage

`--service-window` retains the same native event loop, `ControlFlow::Wait`, softbuffer surface, and resize/close path, then constructs an optional Bevy app before normal startup. It installs `MinimalPlugins` without `ScheduleRunnerPlugin`: task pools, frame counter, time, and core schedules. The native handler calls `App::update()` from `about_to_wait`; it adds no timer, redraw request, renderer, assets, game services, networking, IPC, sound, UI, or resource-limit setup. It logs the first update and each 64th update.

This separates Bevy core-service overhead from both native waiting and later renderer or game workload. `--service-window` is exclusive, has its own title/application name, and leaves ordinary startup unchanged.

## Measured additive core result

Source revisions `217b4de8` and `6f6e10e3` used binary SHA-256 `822ebfde5fa13dd13c9489a8bd519e41be91fb746954eea5a19c894b4c45fd25` and the same 1280×1198 native window loop as the native baseline. The focused 12-second core sample recorded **2 CPU ticks**: **0.16664% of one core**, across 25 threads, with all 13 samples focused. The corresponding focused native sample recorded **0 ticks**, one thread, and all 13 samples focused.

Whole-host busy CPU was 5.407% during the core sample and 5.799% during the native sample. That difference is not attributable to this single process. An earlier native sample with 0/13 focused observations is excluded because the lid was closed and no compositor window was focused; the user subsequently unlocked the desktop and the later focused samples are valid despite the lid state. There is no bulk CPU source at this core-service stage.

The source verifier passed formatting, locked checking, and readability; targeted tests passed 6/6. The routing RED was valid. The core module test did not demonstrate a pre-implementation RED, so it is not claimed as such. Runtime measurement has not received an independent data audit.

## Measurement scope

Measure process/thread idle CPU externally after the window is visible, and record host CPU, window/focus state, GPU activity, clocks, and limits alongside it. Do not add an FPS overlay or diagnostic server. Both diagnostic modes deliberately have no game workload or continuous rendering, so FPS is not a comparable-performance constraint and neither establishes a game optimization.

## Verified native baseline

Commit `c3568e59`, binary SHA-256 `8cde96f5b86da49cf4cf6bf854a9e0d1cd0d9aabf048db12d0300508eace3b4a`, launched from an empty working directory. A visible flat window resized from 1280×1198 to 1480×1198 and exited through the window manager's close request.

Both 12-second idle samples recorded **zero CPU ticks**, one thread in sleeping state, and `do_epoll_wait`. At the host's 100-Hz accounting resolution, each result is below approximately **0.0833% of one CPU core**, not proof of mathematical zero. Native screenshots decode to the expected uniform background before and after resizing.

No game IPC socket, game-asset descriptor, GPU-device descriptor, or GPU-driver mapping was observed. A reopened instance had no owned TCP/UDP sockets; its event-loop timerfd was disarmed. The second instance was left visible for the next initialization step.

Independent verification passed routing tests (2/2), formatting, locked checking, readability, dependency-version review, and native artifact review. Proof lives under `data/diagnostics/movement-perf-20260905/empty-window-baseline/`, including `runtime/`, `visible/`, and `verification/verifier-report-c3568e59.md`.

## Sources

- [empty-window baseline spec](../../specs/empty-window-baseline.md) — native boundary and measurement scope
- [service-window baseline spec](../../specs/service-window-baseline.md) — additive core-service contract and measured result
- [main startup](../../../src/main.rs) — early diagnostic routing and error exit
- [CLI parsing](../../../src/cli_args.rs) — exclusive argument selection
- [empty window runtime](../../../src/empty_window.rs) — shared winit/softbuffer loop and optional app update
- [service window runtime](../../../src/service_window.rs) — minimal Bevy-core plugin boundary
- [Cargo manifest](../../../Cargo.toml) — direct dependency features

## See Also

- [[movement-performance]] — comparative game-runtime CPU investigation results
- [[rendering-pipeline]] — later renderer layers remain outside this diagnostic stage
