# Empty-window baseline

`--empty-window` isolates native window/event handling from the game runtime. Sources: `src/empty_window.rs` and the early startup branch in `src/main.rs`. Results belong in the [performance investigation](../wiki/investigations/movement-performance.md).

## What it must do

- [x] Require `--empty-window` alone; reject combinations with game/asset/server arguments. Without the flag, keep normal startup selection.
- [ ] Create one visible empty native window, without constructing a Bevy app or initializing game systems, game assets, networking, IPC, renderer plugins, or game task pools.
- [ ] Wait for OS events. Paint a flat background only for initial/OS-requested redraws and resizing; no timers, periodic redraw requests, or update loop. Zero-sized windows must not allocate a surface buffer.
- [ ] Handle resizing and OS close requests; report initialization/presentation/event-loop failures explicitly and exit nonzero on failure.
- [ ] Measure idle process/thread CPU externally. Do not install an in-window FPS overlay or diagnostic server. This zero-work baseline is not a comparable-render-FPS optimization.

## How it works

- [Performance investigation](../wiki/investigations/movement-performance.md) — measurements and limits.

## Implementation inventory

- `src/main.rs` — early route before normal thread-pool, asset-root, and resource-limit setup.
- `src/cli_args.rs` — exclusive mode selection and help.
- `src/empty_window.rs` — native event loop and event-driven software painting.
- `Cargo.toml` — direct use of already-locked `winit` for its public event-loop API; `softbuffer` provides the initial shared-memory surface without starting a GPU renderer. Linux painting enables Wayland only, not X11 or KMS; unsupported surfaces fail explicitly. Normal game backend configuration is unchanged.

## Tests asserting this spec

- `tests/unit/empty_window_args_tests.rs` — explicit/exclusive selection and unchanged normal argument routing.
- Native runtime proof — visibility, idle CPU, resize, close, and absence of game initialization are recorded with the investigation artifacts.

## Known gaps (current cycle)

- [ ] Complete native runtime and independent verification.

## Out of scope

- Game initialization, continuous rendering, or FPS comparisons.
- Altering normal worker counts, CPU limits, graphics settings, or deployment.
