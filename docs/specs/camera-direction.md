# Live camera direction

`game-engine-cli camera set` changes the running in-world camera's viewing angles. The CLI lives in `src/bin/game-engine-cli/`; runtime dispatch lives in `src/ipc/plugin/camera_direction.rs`.

## What it must do

- [x] Accept `--yaw-degrees`, `--pitch-degrees`, or both; preserve omitted axes. Positive pitch looks upward; yaw follows the existing camera coordinate system.
- [x] Reject empty requests, non-finite angles, and pitch outside −88° through +88° without partially changing the camera.
- [x] Require InWorld and exactly one active camera carrying `WowCamera`; return explicit errors otherwise.
- [ ] Update the camera's normal input-owned angles without changing player facing, zoom, or collision behavior. Normal input remains available afterward.
- [x] Return resulting yaw and pitch in degrees; subsequent camera-follow frames reflect the requested direction subject to normal collision constraints.

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

Observed at `4e1de84d`: the dev build of both binaries completed; the live InWorld client accepted pitch `60°`, yaw/pitch `180°/45°`, `90°/20°`, and `270°/88°`, each changing the rendered view. A mixed `270°` yaw plus `NaN` pitch returned `camera angles must be finite` with exit 1. Captures are local diagnostic data under `data/diagnostics/camera-direction/` and are not tracked.

## Known gaps (current cycle)

- [ ] Player-facing preservation, normal input after IPC control, and collision-constrained live pitch are not independently proven.
- [ ] Full camera-filter suite has one preexisting char-select test failure caused by missing `MessageSenders<SelectCharacter>::connection`, reproduced on the hash-verified `4a6f62cd` baseline.
- [ ] `cargo fmt --check` reports 104 unchanged vendor files; changed feature files are clean. Nine new tests and scoped dev `cargo check` passed at `4e1de84d`; do not represent the tree as fully format-clean.

## Out of scope

Skybox rendering fixes, non-world orbit cameras, movement, and bypassing camera collision.
