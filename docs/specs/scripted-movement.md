# Scripted movement

Timed client movement provides repeatable performance-test routes without desktop keyboard injection. Playback lives in `src/movement_control.rs`; the camera movement system consumes it. See the [performance investigation](../wiki/investigations/procedural-cloud-regeneration.md) for measurement context.

## What it must do

- [ ] `movement forward --seconds N [--yaw-degrees D]` starts a finite forward segment in InWorld; `movement stop` cancels it.
- [ ] Reject nonfinite, nonpositive, or over-60-second durations and nonfinite headings without replacing active playback. Heading is character-facing yaw: zero points along Bevy +Z, 90 degrees along +X.
- [ ] Preserve normal movement speed, collision, animation direction, and network input. Do not teleport. Clip the final movement step to the remaining duration.
- [ ] Stop on expiry, explicit stop, manual movement, modal opening, reconnect, or leaving InWorld. Never resume cancelled playback automatically.
- [ ] Starting a segment clears an existing map waypoint so pathfinding cannot compete with timed movement.

## How it works

- [Performance investigation](../wiki/investigations/procedural-cloud-regeneration.md)

## Implementation inventory

- `src/movement_control.rs` — bounded playback state and time steps.
- `src/rendering/camera/camera.rs` — normal movement integration and cancellation.
- `src/ipc/mod.rs`, `src/ipc/plugin.rs` — request protocol and dispatch.
- `src/bin/game-engine-cli/` — command parsing and transport.

## Tests asserting this spec

- `tests/unit/movement_control_tests.rs` — duration and heading validation, clipped expiry, stop.
- `tests/unit/camera_scripted_movement_tests.rs` — displacement, collision, network direction, cancellation.
- `src/bin/game-engine-cli/tests/` — CLI request behavior.
- `src/ipc/plugin.rs` — InWorld dispatch, validation, waypoint cancellation, stop response.

## Known gaps (current cycle)

- [ ] Verify an actual live route before using it for performance comparisons.

## Out of scope

Teleport benchmarks, obstacle avoidance, and a general input scripting language. Existing waypoint pathfinding remains separate.
