# Empty-Stage Replicated-Unit NOOP Workload

The empty InWorld diagnostic stage suppresses world visuals and, since `96e1308a`, does not spawn the world camera; it still does not suppress networking or replicated entities. The investigation found semantic no-op work at both the client ECS boundary and the server replication boundary. The fixes are committed; `e745d35e` also removes the steady-state full scan of replicated NPC visibility policies. `c446d81c` additionally removes camera/input/player movement/follow dispatch before `Character`, including Empty-only collision/pathing collection and raycast setup. Machine-side empty-stage relaunch proof confirms the corrected client remains connected without the prior UI log flood, but no runtime performance improvement is claimed.

## Reproduction boundary

The preserved pre-`96e1308a` empty-stage client remained connected with networking, IPC, window, world camera, and the performance panel active. Current `Empty` behavior removes that world camera while retaining the standalone performance panel/UI camera. IPC reported `remote_entities=133` and `local_players=1`; the scene contained 132 NPCs plus one local player, so the `RemoteEntity` count includes the local player despite its field name. NPC visuals were suppressed (`is_displayed=false`), but replicated unit entities remained present.

## Confirmed NOOP boundaries

### Client per-frame ECS writes

- Before `538e8329`, `interpolate_remote_entities` ran every rendered `Update` frame for non-local `RemoteEntity` entities and could assign remote visual `Transform` translation and rotation even when interpolation was already at the target. At the preserved workload this was 132 hidden NPC roots per frame.
- `538e8329` gates remote visual interpolation behind cumulative `Npcs`. `Empty`, `Character`, `Skybox`, and `Terrain` no longer mutate remote visual `Transform`; connection/replication receive and `sync_replicated_transforms` remain active and continue updating interpolation targets for later stages.
- Before `e745d35e`, `apply_npc_visibility_policy` ran every `Update` frame for replicated NPCs and recalculated every policy. It originated in `1a1a8179` as straightforward reconciliation after local alive-state synchronization; `6ffa6ce0` added dawn/night policies without changing the per-frame scan. `3c77d346` stopped repeated equal `Visibility` assignments, but did not remove the scan.
- `e745d35e` makes visibility trigger-driven: `Changed<Npc>` updates only the changed/added NPC entity; semantic `LocalAliveState` changes trigger a one-time scan that reapplies only `DeadOnly` policies; dawn/dusk phase transitions trigger a one-time scan that reapplies only scheduled policies; state or NPC-stage activation performs one full reconciliation. Between those triggers, no full replicated-NPC visibility query runs.
- Equal `Transform` writes woke Bevy transform change detection and propagation. Equal `Visibility` writes woke visibility propagation queries, although hidden roots without visual children avoided render-queue work.

### Strict Empty camera/input boundary

`c446d81c` gates the chained `sync_camera_options`, `camera_input`, `cursor_grab`, `player_movement`, and `camera_follow` systems at the exact `Character` stage. `Empty` skips their per-frame dispatch; `Character` and later stages retain camera control, movement, collision, and follow behavior. This is separate from networking/replication receive and does not alter replicated state.

The source evidence is `player_movement`: before the guard, it allocated collision/pathing collections and prepared raycast state every dispatched frame even though strict `Empty` has no world camera, terrain, or displayed character. The guard removes that Empty-only setup without changing the later-stage movement path. RED/GREEN evidence is recorded in `/tmp/claude/game-engine-perf/character-stage-guard-red.log`, `character-stage-guard-green.log`, and `character-stage-tests-green.log`; formatting/readability evidence is in `character-stage-cargo-fmt-check.log` and `character-stage-readability/`. The protected camera instrumentation was preserved through partial staging and excluded from `c446d81c`.

Rebuilt PID `2176863` remained connected with zero world camera/terrain/displayed NPCs and measured **10.01 FPS / 99.89 ms** plus **11.10% of one core** over 10 seconds. The prior paced client measured 11.20%, but remote entities changed from 70 to 75; the 0.10-point difference is not accepted as measurable improvement. The 10 FPS limiter remains temporary, and Character remains blocked pending the `<=10%` Empty gate.

## Server-to-client replication

Lightyear's sender uses Bevy change ticks. Its receiver compares an incoming component with the existing value and suppresses the final ECS replacement when equal. That prevents downstream client `Changed<T>` observers, but it does not avoid server change detection, serialization, transmission, deserialization, or equality comparison.

The server had two confirmed same-value replicated-state sources:

- `apply_movement_input` rewrote `Rotation` and `MovementSpeed` for identical input values.
- `apply_terrain_gravity` acquired mutable replicated state and rebuilt grounded state in a way that could dirty unchanged `Position` and `Health` values.

The nearby NPC population must be separated from those NOOPs. Current nearby `MovementType=2` NPCs have no `waypoint_data` rows, so waypoint-delay dirties do not explain this workload. Wander movement is real semantic `Position` change and is not a no-op; the preserved 100-yard set contained 79 wanderers, 49 static NPCs, and 5 movement-type-2 NPCs without waypoint paths.

## Fixes and tests

- `game-engine` commit `3c77d346` assigns interpolated `Transform` fields only when the exact result differs and assigns NPC `Visibility` only when the desired state differs. Tests cover stable interpolation, stable visibility, and real visibility changes.
- `game-engine` commit `e745d35e` replaces the per-frame NPC visibility scan with entity-scoped and trigger-scoped reconciliation, and makes `LocalAliveState` change only on semantic transitions. Focused RED/GREEN proof: the pre-implementation run reported 6 failures and 1 pass; the current GREEN run, `cargo test --bin game-engine event_driven_npc_visibility -- --nocapture`, passes 7 tests covering added/changed NPCs, alive transitions, dawn/dusk and wrapped time jumps, stage activation, and unchanged health. This is behavioral proof only; no CPU measurement is claimed.
- `game-engine` commit `96e1308a` removes the world `WowCamera`/`Camera3d` from `Empty` while retaining it for `Character` and later stages; the standalone performance panel/UI camera remains. Tests cover the Empty zero-camera boundary and Character re-entry.
- `game-engine` commit `538e8329` gates remote visual interpolation at cumulative `Npcs` while preserving replicated target synchronization earlier. The RED run (`/tmp/claude/game-engine-perf/strict-empty-interpolation-red.log`) failed `registered_interpolation_does_not_move_remote_before_npcs` with `Vec3(0.1118857, 0.0, 0.0)` at `Empty` while the `Npcs` test passed; the GREEN run (`/tmp/claude/game-engine-perf/strict-empty-interpolation-green.log`) passed both stage-boundary tests.
- `game-server` commit `2927382` assigns `Rotation` and `MovementSpeed` only when their desired values differ. Its regression test proves identical input does not dirty either component while changed input still applies.
- `game-server` commit `ae81c65` calculates gravity state off-component and writes `Position`, `VerticalVelocity`, `FallTracker`, and `Health` only when values differ. Tests cover grounded stability and real falling changes.

These changes preserve actual movement, interpolation, policy transitions, jumping/falling, and fall damage. They do not change NPC waypoint behavior.

## Runtime status

The fixes are source- and test-backed. The corrected server binary is running, and `/tmp/claude/game-engine-perf/pre-ui-empty-508891a6-live.json` records the corrected empty-stage client using `game-engine` `508891a6` and `ui-toolkit` `50e4a17`: connected `InWorld`, one link, one local player, and 133 remote entities. The toolkit UI tree and `MainActionBar` filter were empty; stderr had zero `[UI]` and zero `UIActionBar.BLP` lines, with no font panic, GPU OOM, device-loss, or panic. `ping` and `performance` were responsive.

Commit `538e83290769c70a6980ec0903db74bf2981c0fb` received a live replacement on PID `2960624`, start ticks `182917558`, socket `/tmp/game-engine-2960624.sock`, with server PID `82964` unchanged at start ticks `178818377`. Its pre-gate comparison used commit `96e1308a31940ed5c03046b574ca0b52fe15d8e2`, PID `2592665`, start ticks `182782909`, and socket `/tmp/game-engine-2592665.sock`. Both clients had no world `WowCamera`/`Camera3d`; remote counts were `133` post-gate versus `134` pre-gate. Passive aggregate CPU changed `325.49% → 288.12%`, compute-pool CPU `229.17% → 197.62%`, and client gfx occupancy `6.93% → 5.60%`. FPS/frame direction is not acceptance evidence because runqueue delay changed `2.389 → 4.565` seconds and live conditions drifted. The subsequent three-sample eu-stack capture for PID `2960624` contained no `interpolate_remote_entities` stack, but about `2.9` CPU cores remain; the Empty-stage root-cause loop continues and no final performance acceptance claim is established. See [[procedural-cloud-regeneration]] for the complete evidence paths.

## Sources

- [client networking](../../../src/game/networking/mod.rs) — interpolation, replication receive path, and remote-entity transforms
- [client NPC networking](../../../src/game/networking/npc.rs) — NPC visibility policy and trigger registration
- [client player networking](../../../src/game/networking/player.rs) — semantic local alive-state updates
- [client networking tests](../../../tests/unit/networking_tests/sync_interp.rs) — stable interpolation and visibility behavior
- [server networking](../../../../game-server/crates/server/src/networking.rs) — movement input and terrain gravity
- [server movement tests](../../../../game-server/crates/server/src/networking_tests/movement.rs) — unchanged movement-state regression
- [server physics tests](../../../../game-server/crates/server/src/networking_tests/physics.rs) — grounded and falling gravity regression
- `game-engine` commit `3c77d346` — skip unchanged replicated unit writes
- `game-server` commit `2927382` — avoid dirtying unchanged player movement state
- `game-server` commit `ae81c65` — avoid dirtying unchanged gravity state
- `game-engine` commit `96e1308a` — skip world camera at Empty stage
- `game-engine` commit `e745d35e` — event-driven NPC visibility
- `game-engine` commit `c446d81c` — gate camera/input/player movement/follow at Character
- `/tmp/claude/game-engine-perf/character-stage-guard-red.log`, `character-stage-guard-green.log`, `character-stage-tests-green.log` — stage-boundary behavioral evidence

## See Also

- [[networking]] — replication architecture and server/client boundaries
- [[procedural-cloud-regeneration]] — broader in-world performance investigation; this NOOP finding is separate, with no comparative FPS claim
- [[ui-system]] — pre-`Ui` game-UI and toolkit scheduling boundary
