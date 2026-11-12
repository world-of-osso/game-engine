# World camera collision

The in-world orbit camera must use the geometry around the player and camera. Native queries live in `godot/rust/src/camera.rs`; orbit and distance recovery live in `godot/core/src/camera_follow_data.rs`. See the [surface-rule investigation](../wiki/investigations/dungeon-camera-adt.md).

## What it must do

- [x] Preserve a clear orbit above a WMO floor even when ADT terrain is far overhead, including Ruby Life Pools' lower entrance.
- [x] Stop before actual outdoor ADT surfaces and nearby WMO walls. Actual collidable geometry, not ADT height, determines obstruction.
- [x] Preserve the existing orbit/zoom, collision offset, recovery, smoothing, and final visibility check.

## How it works

- [Geometry and player support](../wiki/investigations/dungeon-camera-adt.md#decided-rule).

## Implementation inventory

- `godot/rust/src/camera.rs` — live physics queries and camera ownership.
- `godot/core/src/camera_follow_data.rs` — pure orbit and collision-distance calculation.

## Tests asserting this spec

- `godot/rust/src/wmo/collision_tests.rs` — real Ruby support/clear orbit; real outdoor ADT obstruction; Stockade walls/stairs.
- `godot/core/tests/camera_data.rs` — orbit, zoom, obstruction and recovery behavior.
- `godot/tests/dungeon_camera.gd` — private-server Forever capture and independent geometric orbit oracle.

## Known gaps (current cycle)

- None for this cycle.

## Out of scope

Player movement, rendering visibility policy, camera volume sweeps, and water-camera behavior.
