# Empty-window baseline

`--empty-window` provides a native Wayland window/event-loop baseline outside the game runtime. It exists to measure external idle CPU separately from Bevy/game integration; no runtime measurement has been recorded.

## Runtime boundary

`main` checks the complete argument list before normal startup. The mode accepts only `--empty-window`; any extra argument is an error. A successful selection calls `empty_window::run()` then returns before thread-pool configuration, asset-root discovery, resource limits, simple-flag handling, CLI parsing, and `run_app`.

The mode therefore does not construct a Bevy `App`, register plugins, initialize game assets, networking, IPC, renderer plugins, or game task pools. Normal invocations do not select this branch.

## Native window behavior

`empty_window::run()` creates a winit event loop with `ControlFlow::Wait`, a softbuffer context, and one application handler. On resume it creates one window and requests its first redraw.

Painting occurs only for `RedrawRequested`. Resize and scale-factor events request another redraw. The paint path resizes the surface, fills it with a flat background, notifies the window before presentation, then presents. It requests no timer, periodic redraw, update loop, or continuous rendering. Zero-size windows return before a surface resize or buffer acquisition. Close exits the event loop. Window, surface, context, event-loop, and presentation errors include context and return a nonzero process status through `main`.

Linux window attributes set the Wayland application name. The direct softbuffer dependency enables only `wayland` and `wayland-dlopen`; this diagnostic does not provide X11 or KMS painting. `winit` is reused as a direct dependency for its public event-loop API. `softbuffer` is the only added dependency, used for a shared-memory surface without starting a GPU renderer. Existing locked versions remain unchanged.

## Measurement scope

Measure process/thread idle CPU externally after the window is visible. Do not add an FPS overlay or diagnostic server. Because the mode deliberately has no game workload or continuous rendering, FPS is not a comparable-performance constraint and this baseline cannot establish a game optimization.

## Sources

- [empty-window baseline spec](../../specs/empty-window-baseline.md) — intended boundary and measurement scope
- [main startup](../../../src/main.rs) — early exclusive route and error exit
- [CLI parsing](../../../src/cli_args.rs) — exclusive argument selection
- [empty window runtime](../../../src/empty_window.rs) — winit/softbuffer event handling and presentation
- [Cargo manifest](../../../Cargo.toml) — direct dependency features

## See Also

- [[movement-performance]] — records comparative game-runtime CPU investigation results
