# Scripted movement

Timed client movement provides repeatable performance-test routes without desktop keyboard injection. Playback lives in `src/movement_control.rs`; the camera movement system consumes it. See the [performance investigation](../wiki/investigations/movement-performance.md) for measurement context.

## What it must do

- [x] `movement forward --seconds N [--yaw-degrees D]` starts a finite forward segment in InWorld; `movement stop` cancels it.
- [x] Reject nonfinite, nonpositive, or over-60-second durations and nonfinite headings without replacing active playback. Heading is character-facing yaw: zero points along Bevy +Z, 90 degrees along +X.
- [x] Preserve normal movement speed, collision, animation direction, and network input. Do not teleport. Clip the final movement step to the remaining duration.
- [x] Doodad solidity follows authored M2 collision triangles after broadphase. Visual bounds remain available for zone interactions but never block movement; doodads without authored geometry are non-solid.
- [x] Stop on expiry, explicit stop, manual movement, modal opening, reconnect, or leaving InWorld. Never resume cancelled playback automatically.
- [x] Starting a segment clears an existing map waypoint so pathfinding cannot compete with timed movement.

## How it works

- [Performance investigation](../wiki/investigations/movement-performance.md)

## Implementation inventory

- `src/movement_control.rs` — bounded playback state and time steps.
- `src/rendering/camera/camera.rs` — normal movement integration and cancellation.
- `src/ipc/mod.rs`, `src/ipc/plugin.rs` — request protocol and dispatch.
- `src/bin/game-engine-cli/` — `movement forward` / `movement stop` parsing and IPC transport.

## Tests asserting this spec

- `tests/unit/movement_control_tests.rs` — duration and heading validation, clipped expiry, stop.
- `tests/unit/camera_scripted_movement_tests.rs` — displacement, authored-doodad collision, network direction, cancellation.
- `tests/unit/terrain_objects_collision_tests.rs` — real canopy, rotated/scaled placement, trunk-hit, and empty-authored-geometry behavior.
- `src/bin/game-engine-cli/tests/request_world_and_equipment.rs` — command mapping and negative heading parsing.
- `src/ipc/plugin.rs` — InWorld dispatch, validation, waypoint cancellation, stop response.

## Known gaps (current cycle)

Connected-runtime displacement and a loaded-tile route comparison are recorded in [movement performance](../wiki/investigations/movement-performance.md). No open control-implementation gap; tile-boundary profiling remains outside this control spec.

## Out of scope

Teleport benchmarks, obstacle avoidance, and a general input scripting language. Existing waypoint pathfinding remains separate.
