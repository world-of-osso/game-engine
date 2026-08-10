# Networking

The game-engine connects to game-server over UDP using lightyear 0.26 (netcode). The server runs at 20Hz; the client receives replicated entity components (`Position`, `Health`, `Mana`, etc.) and sends `PlayerInput` messages.

## Architecture

```
game-engine (client)              game-server (headless Bevy)
  lightyear client plugin    ←→   lightyear server, UDP :5000
  shared crate components         shared crate components
  Replicated entity detection     Player spawn on connect
  PlayerInput → server            Apply movement, creature spawning
```

The `game-server/crates/shared/` crate is depended on by both sides. It defines replicated components and protocol channels:
- `MovementChannel` — unreliable, for position updates
- `CombatChannel` / `ChatChannel` — reliable ordered

## Auth Flow

1. **Register**: client sends `RegisterRequest { username, password }`. Server hashes with argon2, stores in redb `PASSWORDS` table, returns session token.
2. **Login (password)**: client sends `LoginRequest { token: None, username, password }`. Server verifies argon2 hash, returns token.
3. **Login (cached token)**: client sends `LoginRequest { token: Some(cached), username, password: "" }`. Server validates token directly.

Token is stored client-side at `data/auth_token`. Delete to force password re-entry.

**Security note**: passwords are transmitted in plaintext over UDP — netcode has no encryption. Acceptable for LAN/dev only.

## Entity Replication

On `Added<Position>` with `Replicated` marker:
- Own player entity: attach camera follow
- Other entities: spawn placeholder mesh or M2 model

NPC display resolution: `Npc { template_id }` → `ModelDisplay { display_id }` → FDID → M2 file.

## Empty-Stage Replication NOOPs

The empty InWorld diagnostic stage suppresses visuals, not networking. A preserved run reported `remote_entities=133` with one local player; the scene contained 132 NPCs plus that local player, so the count includes the local player despite its name.

Before `538e8329`, `interpolate_remote_entities` performed remote visual `Transform` interpolation every rendered `Update` frame, including hidden NPCs in pre-`Npcs` stages. `538e8329` moves that system behind the cumulative `Npcs` stage gate. `Empty`, `Character`, `Skybox`, and `Terrain` now leave remote visual `Transform` unchanged while connection/auth, replication receive, and `sync_replicated_transforms` remain active; replicated network positions and rotations still update `InterpolationTarget`/`RotationTarget` for later visual stages. NPC policy evaluation remains a separate visibility boundary.

Lightyear equality-suppresses the final client ECS replacement for equal replicated components, but server-side same-value `Rotation`/`MovementSpeed` input writes and grounded gravity state still entered change detection, serialization, and transmission. Actual wander movement remains real `Position` change, not a replication NOOP. The current nearby movement-type-2 NPCs have no waypoint rows, so waypoint-delay behavior is not implicated in this workload.

`e745d35e` removes the steady-state NPC visibility scan. `Changed<Npc>` applies policy to only the added or changed entity. `sync_local_alive_state` writes `LocalAliveState` only when its boolean changes; that transition triggers a one-time scan that reapplies only `DeadOnly` policies. Dawn/dusk phase transitions trigger a one-time scan that reapplies only scheduled policies. Game-state or NPC-stage activation performs one full reconciliation. No full replicated-NPC visibility query runs between those triggers. History: `1a1a8179` introduced per-frame reconciliation, `6ffa6ce0` retained it for day/night policies, and `3c77d346` only guarded equal writes.

Conditional client writes and server state updates are committed in `3c77d346`, `e745d35e`, `2927382`, and `ae81c65`, with regression tests for stable and changing state. The focused `e745d35e` GREEN run passes 7 event-driven visibility tests; no CPU improvement is claimed. The post-gate live replacement used game-engine commit `538e83290769c70a6980ec0903db74bf2981c0fb`, PID `2960624`, start ticks `182917558`, and socket `/tmp/game-engine-2960624.sock`; its pre-gate comparison used commit `96e1308a31940ed5c03046b574ca0b52fe15d8e2`, PID `2592665`, start ticks `182782909`, and socket `/tmp/game-engine-2592665.sock`. Both had no world camera; remote counts were `133` and `134`. Aggregate CPU changed `325.49% → 288.12%`, compute-pool CPU `229.17% → 197.62%`, and client gfx occupancy `6.93% → 5.60%`. FPS/frame direction is not acceptance evidence because runqueue delay and live conditions drifted. The subsequent strict-Empty eu-stack capture contains no interpolation stack, but about `2.9` cores remain, so the root-cause loop continues. See [[replicated-unit-noops]].

## Demand-Driven IPC Status Snapshots

Commit `abf68fd9` moves the eight expensive status snapshots out of unconditional InWorld `Update` work. IPC now orders `Receive → RefreshStatus → Dispatch`; queued commands select only their required network, terrain, sound, character, gear, appearance, roster, or map refresh. Idle updates with no IPC command perform no status-snapshot rebuild, and `Ping`/`Performance` request none. The duplicate map-sync registration in `game/networking/mod.rs` was removed. See [[procedural-cloud-regeneration]] for the exact dependency matrix and RED/GREEN evidence. The rebuilt status-demand client still consumed 369.05% of one core; without a matched pre-change workload this establishes no isolated status-refresh CPU effect and did not satisfy the Empty gate.

## Strict Empty Diagnostic Pacing

Commit `4fb2e5c9` adds a diagnostic frame limiter after the demand-driven IPC status refresh. Only exact `GameState::InWorld` plus exact `InWorldSceneStage::Empty` uses a fixed **100 ms** interval (**10 FPS**) through the existing limiter. `Character` and later stages, plus all other states, retain the persisted/global graphics frame-rate limit and `PresentMode`; FPS overlay, networking, and IPC remain active. This is frame-cadence control, not removal of render/application work. PID `2130439` measured **9.98 FPS / 100.23 ms** and **11.20% of one core**, so the `<=10%` gate still failed. Alessio chose to keep 10 FPS temporarily for investigation, not as the final fix; Character remains blocked. See [[procedural-cloud-regeneration]].

## Multi-ADT Terrain Streaming (Planned Phase 3)

Server sends `LoadTerrain { tile_x, tile_y }` messages as player moves. Client `TerrainManager` tracks loaded tiles in a `HashMap<(u8,u8), Entity>` and despawns out-of-range tiles.

## Known Issues

**Remote login panic (2026-03-06)**: after netcode connection is established, lightyear server panics in `src/send/components.rs:1130` ("not yet implemented") on a `ReplicationMode::SingleSender` path. The server restarts under systemd and the client times out. This is a server-side replication bug, not a firewall issue — UDP traffic was confirmed to flow both ways. Fix: add diagnostics around `Replicate` insertion and `ReplicationSender` component on connect. See [remote-login-debug-2026-03-06.md](../remote-login-debug-2026-03-06.md).

## Sources

- [network-integration.md](../../network-integration.md) — phased integration plan, crate deps, phase deliverables
- [remote-login-debug-2026-03-06.md](../../remote-login-debug-2026-03-06.md) — remote login failure, lightyear replication panic
- [authentication.md](../../authentication.md) — auth flow, token storage, argon2, redb tables
- [replicated-unit-noops](../investigations/replicated-unit-noops.md) — empty-stage NOOP evidence and suppression commits
- [client networking source](../../../src/game/networking/mod.rs) — interpolation and replication receive systems
- [client NPC networking source](../../../src/game/networking/npc.rs) — event-driven visibility policy systems
- [client player networking source](../../../src/game/networking/player.rs) — semantic alive-state updates
- [server networking source](../../../../game-server/crates/server/src/networking.rs) — movement and gravity mutation boundaries
- `game-engine` commit `e745d35e` — event-driven NPC visibility
- `/tmp/claude/game-engine-perf/pre-ui-empty-508891a6-live.json` — connected empty-stage relaunch proof

## See Also

- [[ui-system]] — login UI that feeds into the auth flow
- [[terrain]] — terrain streaming (Phase 3 networking dependency)
- [[lore-knowledge-graph]] — server-side graph authority model
- [[replicated-unit-noops]] — empty-stage semantic no-op boundaries and suppression fixes
