# Scripted Movement

`ScriptedMovement` provides finite, repeatable forward movement for connected performance runs without desktop input injection or teleportation.

## Behavior

`src/movement_control.rs` owns one optional timed segment. A start request accepts a finite duration greater than zero and no more than 60 seconds, plus an optional finite character-facing heading. Headings normalize to `[0, 360)` degrees before conversion to radians.

Each frame consumes at most the remaining segment time, so the final movement step is clipped instead of exceeding the requested route. Invalid starts leave an active segment unchanged; stop clears it.

`player_movement` consumes the segment through its existing forward-movement path. It uses the ordinary speed, collision clamps, transform updates, animation direction, and network input flow—not a position teleport. A segment is cancelled on manual movement input, a modal, reconnect/input loss, taxi travel, loss of the local player, or leaving `InWorld`. Starting a valid timed route clears an existing map waypoint, keeping waypoint pathing from competing with the route.

## Measurement boundary

This system makes a route reproducible; it does not establish a performance result. The September 5 movement investigation has no valid moving-frame comparison yet: waypoint attempts did not displace the player, and the scripted IPC/CLI route still requires connected-runtime proof.

## Sources

- [scripted movement spec](../../specs/scripted-movement.md) — user-facing contract and exclusions
- [movement state](../../../src/movement_control.rs) — duration, heading, and step semantics
- [camera movement](../../../src/rendering/camera/camera.rs) — normal movement-path integration and cancellation
- [movement-state tests](../../../tests/unit/movement_control_tests.rs) — validation and clipped-expiry behavior
- [camera movement tests](../../../tests/unit/camera_scripted_movement_tests.rs) — normal-path behavior

## See Also

- [[networking]] — connection/input lifecycle
- [[terrain]] — terrain and object collision
- [[procedural-cloud-regeneration]] — performance investigation and measurement status
