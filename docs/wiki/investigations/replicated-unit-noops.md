# Empty-Stage Replicated-Unit NOOP Workload

The empty InWorld diagnostic stage suppresses world visuals, but it does not suppress networking or replicated entities. The investigation found semantic no-op work at both the client ECS boundary and the server replication boundary. The fixes are committed, but no runtime performance improvement is claimed until the corrected binaries are relaunched and measured.

## Reproduction boundary

The preserved empty-stage client remained connected with networking, IPC, window, camera, and the performance panel active. IPC reported `remote_entities=133` and `local_players=1`; the scene contained 132 NPCs plus one local player, so the `RemoteEntity` count includes the local player despite its field name. NPC visuals were suppressed (`is_displayed=false`), but replicated unit entities remained present.

## Confirmed NOOP boundaries

### Client per-frame ECS writes

- `interpolate_remote_entities` ran every rendered `Update` frame for non-local `RemoteEntity` entities and assigned `Transform` translation and rotation even when interpolation was already at the target. At the preserved workload this was 132 hidden NPC roots per frame.
- `apply_npc_visibility_policy` ran every `Update` frame for replicated NPCs and assigned the same `Visibility` value repeatedly. These assignments advanced `Changed<Visibility>` despite no policy change.
- Equal `Transform` writes woke Bevy transform change detection and propagation. Equal `Visibility` writes woke visibility propagation queries, although hidden roots without visual children avoided render-queue work.

### Server-to-client replication

Lightyear's sender uses Bevy change ticks. Its receiver compares an incoming component with the existing value and suppresses the final ECS replacement when equal. That prevents downstream client `Changed<T>` observers, but it does not avoid server change detection, serialization, transmission, deserialization, or equality comparison.

The server had two confirmed same-value replicated-state sources:

- `apply_movement_input` rewrote `Rotation` and `MovementSpeed` for identical input values.
- `apply_terrain_gravity` acquired mutable replicated state and rebuilt grounded state in a way that could dirty unchanged `Position` and `Health` values.

The nearby NPC population must be separated from those NOOPs. Current nearby `MovementType=2` NPCs have no `waypoint_data` rows, so waypoint-delay dirties do not explain this workload. Wander movement is real semantic `Position` change and is not a no-op; the preserved 100-yard set contained 79 wanderers, 49 static NPCs, and 5 movement-type-2 NPCs without waypoint paths.

## Fixes and tests

- `game-engine` commit `3c77d346` assigns interpolated `Transform` fields only when the exact result differs and assigns NPC `Visibility` only when the desired state differs. Tests cover stable interpolation, stable visibility, and real visibility changes.
- `game-server` commit `2927382` assigns `Rotation` and `MovementSpeed` only when their desired values differ. Its regression test proves identical input does not dirty either component while changed input still applies.
- `game-server` commit `ae81c65` calculates gravity state off-component and writes `Position`, `VerticalVelocity`, `FallTracker`, and `Health` only when values differ. Tests cover grounded stability and real falling changes.

These changes preserve actual movement, interpolation, policy transitions, jumping/falling, and fall damage. They do not change NPC waypoint behavior.

## Runtime status

The fixes are source- and test-backed. The corrected server binary is running, while a clean corrected-client relaunch remains pending before comparing runtime FPS or frame time. No performance improvement is established by this investigation alone.

## Sources

- [client networking](../../../src/game/networking/mod.rs) — interpolation, replication receive path, and remote-entity transforms
- [client NPC networking](../../../src/game/networking/npc.rs) — NPC visibility policy
- [client networking tests](../../../tests/unit/networking_tests/sync_interp.rs) — stable interpolation and visibility behavior
- [server networking](../../../../game-server/crates/server/src/networking.rs) — movement input and terrain gravity
- [server movement tests](../../../../game-server/crates/server/src/networking_tests/movement.rs) — unchanged movement-state regression
- [server physics tests](../../../../game-server/crates/server/src/networking_tests/physics.rs) — grounded and falling gravity regression
- `game-engine` commit `3c77d346` — skip unchanged replicated unit writes
- `game-server` commit `2927382` — avoid dirtying unchanged player movement state
- `game-server` commit `ae81c65` — avoid dirtying unchanged gravity state

## See Also

- [[networking]] — replication architecture and server/client boundaries
- [[procedural-cloud-regeneration]] — broader in-world performance investigation; this NOOP finding is a separate workload cause and has no FPS claim yet
