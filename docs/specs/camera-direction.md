# Live camera direction

`game-engine-cli camera set` changes the running in-world camera's viewing angles. The CLI lives in `src/bin/game-engine-cli/`; runtime dispatch lives in `src/ipc/plugin/camera_direction.rs`.

## What it must do

- [ ] Accept `--yaw-degrees`, `--pitch-degrees`, or both; preserve omitted axes. Positive pitch looks upward; yaw follows the existing camera coordinate system.
- [ ] Reject empty requests, non-finite angles, and pitch outside −88° through +88° without partially changing the camera.
- [ ] Require InWorld and exactly one active camera carrying `WowCamera`; return explicit errors otherwise.
- [ ] Update the camera's normal input-owned angles without changing player facing, zoom, or collision behavior. Normal input remains available afterward.
- [ ] Return resulting yaw and pitch in degrees; subsequent camera-follow frames reflect the requested direction subject to normal collision constraints.

## How it works

- [Rendering pipeline](../wiki/systems/rendering-pipeline.md)

## Implementation inventory

- `src/ipc/mod.rs` — request serialization.
- `src/bin/game-engine-cli/main.rs`, `requests.rs` — command arguments and request mapping.
- `src/ipc/plugin.rs`, `plugin/camera_direction.rs` — main-thread active-camera selection and response.
- `src/camera_control.rs` — shared `WowCamera` component, atomic validation and angle update.
- `src/lib.rs`, `src/rendering/camera/camera.rs` — expose the same component to IPC and runtime camera systems.

## Tests asserting this spec

- `src/bin/game-engine-cli/tests/camera.rs` — command parsing and wire request.
- `tests/unit/camera_tests.rs` — angle updates and rejected inputs.
- `src/ipc/plugin/camera_direction.rs` — scene and camera selection.
- `src/rendering/camera/camera_follow.rs` — resulting view direction persists across follow updates.

## Known gaps (current cycle)

- [ ] Targeted tests and live CLI screenshot proof pending.

## Out of scope

Skybox rendering fixes, non-world orbit cameras, movement, and bypassing camera collision.
