# Networking

The game-engine connects to game-server over UDP using Lightyear 0.28 (netcode). The server runs at 20Hz; the client receives replicated entity components (`Position`, `Health`, `Mana`, etc.) and sends `PlayerInput` messages.

## Architecture

```
game-engine (client)              game-server (headless Bevy)
  lightyear client plugin    ←→   lightyear server, UDP :5000
  shared crate components         shared crate components
  Remote entity detection        Player spawn on connect
  PlayerInput → server            Apply movement, creature spawning
```

The `game-server/crates/shared/` crate is depended on by both sides. It defines replicated components and protocol channels:
- `MovementChannel` — unreliable, for position updates
- `CombatChannel` / `ChatChannel` — reliable ordered

## Application network cadence and dispatch

`NetworkTick` is a logical 60 Hz application schedule using real-time accumulation. It shares the main ECS thread, so due ticks may batch after lower render cadence and cannot run while that thread is blocked. It is not an independent OS networking thread. The authoritative client/server simulation remains negotiated at 20 Hz; the application cadence changes neither wire formats nor simulation rate.

The historical main-world path keeps Lightyear link, transport, and message maintenance on every frame because transport sender maintenance advances delta-based timers from frame delta. Its receiver park/restore workaround is retired from `network_events`: application handlers now consume owned `Inbox<M>` data rather than main-world Lightyear receivers.

`network_events` owns one incoming and one outgoing registry. Route registration initializes `Inbox<M>` and records a typed worker relay. A worker relay drains its transport receiver into an owned FIFO batch; main-world updates append that batch to `Inbox<M>`. Incoming routing samples each inbox once and invokes eligible handlers in registration order only when buffered work exists. Outgoing routing invokes registered handlers only when their cheap queue/dirty predicate is ready. `MessageReceivers<M>` presents one main inbox; `MessageSenders<M>` presents the worker sender only while connected. Auth, gameplay/API, profession, auction, and social handlers use these adapters. Collection/death retain their reply-owning consumers rather than competing generic handlers. Auth solely consumes character-creation responses and emits a local result event; the character-create scene observes that event only while active, avoiding a second network consumer.

### Pending dedicated network world

Commits `a35b1c5c` through `01cfcade` add `src/network_runtime/worker.rs`, `messages.rs`, and `replication.rs`: a separately clocked 60 Hz Bevy network-world owner, typed owned-message adapters, and a server-entity-keyed replication-mirror foundation. The worker configures Lightyear's client plugin at the existing 50 ms simulation interval and provides FIFO commands, main-world update closures, explicit stop/join, and failure propagation. `runtime-tests-transport.log` records 13/13 module tests, including actual encoded Lightyear loopback packet delivery while the main app remains unupdated. `owned-inbox-red-behavior.log` then `owned-inbox-green.log` record the dispatcher migration's RED and 8/8 GREEN proof.

No production plugin starts this worker yet. The main binary still owns its live client, transport, replication, connection markers, and reconnect lifecycle; the new replication foundation is not attached to live worker updates. Consequently, neither those module tests nor the adapter migration proves runnable independent network execution. Integration must transfer exclusive Lightyear ownership, create lifecycle updates, bridge server-entity-keyed replicated snapshots and despawns into render entities before Player/Npc observers, and reset both maps on reconnect. A socket-only thread is insufficient because these responsibilities currently live in the main ECS world.

Equipment uses owner-specific `EquipmentChanged` notifications for direct mutation, model dependency arrival, and replication confirmation. Rendering requests coalesce while work is pending; a final changed IPC batch renders once. `db7e6e7b` makes IPC dispatch conditional on nonempty `PendingIpcCommands`, before `dispatch_ipc_commands` acquires heavy parameters. IPC ordering remains receive, requested-status refresh, then dispatch. These changes have no native CPU comparison or CPU-fix claim.

## Current proof and limits

Focused proof records historical main-thread network **8/8**, migrated API **55/55**, equipment **13 passed** plus **2 appearance-event tests**, equipment IPC FIFO **1/1**, and character-create response **3/3**. New module proof records worker/adapter **13/13** and owned-inbox dispatcher **8/8**. It does not prove a connected/reconnect lifecycle, native appearance delivery, independent transport/thread execution, or an idle-CPU improvement.

## Auth Flow

1. **Register**: client sends `RegisterRequest { username, password }`. Server hashes with argon2, stores in redb `PASSWORDS` table, returns session token.
2. **Login (password)**: client sends `LoginRequest { token: None, username, password }`. Server verifies argon2 hash, returns token.
3. **Login (cached token)**: client sends `LoginRequest { token: Some(cached), username, password: "" }`. Server validates token directly.

Token is stored client-side at `data/auth_token`. Delete to force password re-entry.

The client uses Lightyear 0.28's `Remote` receive marker; `Replicated` is only the deprecated compatibility alias.

**Security note**: passwords are transmitted in plaintext over UDP — netcode has no encryption. Acceptable for LAN/dev only.

## Entity Replication

On `Added<Position>` with Lightyear's `Remote` marker:
- Own player entity: attach camera follow
- Other entities: spawn placeholder mesh or M2 model

NPC display resolution: `Npc { template_id }` → `ModelDisplay { display_id }` → FDID → M2 file.

## Empty-Stage Replication NOOPs

The empty InWorld diagnostic stage suppresses visuals, not networking. A preserved run reported `remote_entities=133` with one local player; the scene contained 132 NPCs plus that local player, so the count includes the local player despite its name.

Before `538e8329`, `interpolate_remote_entities` performed remote visual `Transform` interpolation every rendered `Update` frame, including hidden NPCs in pre-`Npcs` stages. `538e8329` moves that system behind the cumulative `Npcs` stage gate. `Empty`, `Character`, `Skybox`, and `Terrain` now leave remote visual `Transform` unchanged while connection/auth, replication receive, and `sync_replicated_transforms` remain active; replicated network positions and rotations still update `InterpolationTarget`/`RotationTarget` for later visual stages. NPC policy evaluation remains a separate visibility boundary.

Lightyear equality-suppresses the final client ECS replacement for equal replicated components, but server-side same-value `Rotation`/`MovementSpeed` input writes and grounded gravity state still entered change detection, serialization, and transmission. Actual wander movement remains real `Position` change, not a replication NOOP. The current nearby movement-type-2 NPCs have no waypoint rows, so waypoint-delay behavior is not implicated in this workload.

`e745d35e` removes the steady-state NPC visibility scan. `Changed<Npc>` applies policy to only the added or changed entity. `sync_local_alive_state` writes `LocalAliveState` only when its boolean changes; that transition triggers a one-time scan that reapplies only `DeadOnly` policies. Dawn/dusk phase transitions trigger a one-time scan that reapplies only scheduled policies. Game-state or NPC-stage activation performs one full reconciliation. No full replicated-NPC visibility query runs between those triggers. History: `1a1a8179` introduced per-frame reconciliation, `6ffa6ce0` retained it for day/night policies, and `3c77d346` only guarded equal writes.

Conditional client writes and server state updates are committed in `3c77d346`, `e745d35e`, `2927382`, and `ae81c65`, with regression tests for stable and changing state. The focused `e745d35e` GREEN run passes 7 event-driven visibility tests; no CPU improvement is claimed. The post-gate live replacement used game-engine commit `538e83290769c70a6980ec0903db74bf2981c0fb`, PID `2960624`, start ticks `182917558`, and socket `/tmp/game-engine-2960624.sock`; its pre-gate comparison used commit `96e1308a31940ed5c03046b574ca0b52fe15d8e2`, PID `2592665`, start ticks `182782909`, and socket `/tmp/game-engine-2592665.sock`. Both had no world camera; remote counts were `133` and `134`. Aggregate CPU changed `325.49% → 288.12%`, compute-pool CPU `229.17% → 197.62%`, and client gfx occupancy `6.93% → 5.60%`. FPS/frame direction is not acceptance evidence because runqueue delay and live conditions drifted. The subsequent strict-Empty eu-stack capture contains no interpolation stack, but about `2.9` cores remain, so the root-cause loop continues. See [[replicated-unit-noops]].

## Network World Reset Scheduling

Commit `ce0ce2d0` (`Gate network reset flush until due`) keeps the exclusive `flush_pending_network_world_reset` system out of frames with no pending reset or a target frame that has not arrived. `network_world_reset_is_due` compares `NetworkUpdateFrame` with the pending target; once due, the existing flush preserves the earliest target, one-frame deferral, exactly-once reset, and all existing reset content.

The pre-fix final Empty PID `2390217` profile sampled the exclusive flush wrapper at **16.12%**. That interval may include a real one-time startup reset, so it was attribution evidence only. RED/GREEN evidence is `/tmp/claude/game-engine-perf/network-reset-due-gate-red.log` and `network-reset-due-gate-green.log`; formatting is `network-reset-due-gate-cargo-fmt.log`; Rust-readability artifacts are under `network-reset-due-gate-readability/`.

Post-fix PID `2402583` remained focused, `InWorld`, and connected with one link/player; it retained FPS text and zero terrain, `Camera3d`, or displayed NPCs. The steady profile contained neither `flush_pending_network_world_reset` nor `network_world_reset_is_due`. However, twelve passive windows after the same 30-second warm-up measured **12.05% mean**, **11.75% median**, and **14.00% maximum** CPU; **0/12** met `<=10.0%`. The due gate is behaviorally verified but did not improve the Empty CPU result. Character remains blocked.

## Initial Reconnect Disconnect Marker

Lightyear `NetcodeClient` requires an initial `Disconnected { reason: None }` marker. Before `0d215316`, the client observer misclassified that marker as a real loss while `GameState::InWorld` and `ReconnectPhase::PendingConnect`; it queued another network reset and replaced the client entity and client ID about every 100 ms before the netcode handshake. This was the source of the reconnect storm, not auth/token rejection.

Commit `0d215316` ignores only a reasonless initial marker with no forced-disconnect notice during `PendingConnect`. Reasoned pending failures, connected disconnects, forced disconnects, and existing authentication/token, character-selection, world-reset, and retry behavior remain handled. RED evidence is `/tmp/claude/game-engine/reconnect-initial-marker-red.log`; GREEN evidence is `/tmp/claude/game-engine/reconnect-initial-marker-green.log` (**14 passed**); formatting is `/tmp/claude/game-engine/reconnect-initial-marker-fmt.log`; readability artifacts are `/tmp/claude/game-engine/reconnect-initial-marker-readability.json` and `/tmp/claude/game-engine/reconnect-initial-marker-readability-metrics/`.

Post-build proof is `/tmp/claude/game-engine/reconnect-fixed-runtime.json` and `/tmp/claude/game-engine/reconnect-fixed-live-current.log`, recorded at engine docs commit `6b034959` with server HEAD `4aca4d3`. The old client PID `2846177` (SHA `84f6ecc...`) was terminated before launching the fixed binary. Exactly one strict-Empty client remained: PID `3715288`, start ticks `192167263`, SHA `aef6f08d318a21063d698816b8202ed21f6ca70b7cf4d015a2520a5f107619c3`, current socket `/tmp/game-engine-3715288.sock`. Across the ten-second stability check, PID/start identity and client ID stayed unchanged; `InWorld`, `connected=true`, `connected_links=1`, `local_players=1`, ping remained `pong`, and performance was **10.21 FPS / 97.98 ms**, `focused=false`. Scene output had 78 NPC entries, all undisplayed, with zero camera/terrain/WMO/doodad/particle terms; terrain tiles/cache counts were zero. Startup showed one expected initial Connecting marker, one Netcode connect/login/InWorld path, and zero InWorld disconnects, reconnect loop, panic, OOM, or device-loss messages. Server PID `3123827` remained cargo-watch managed, matched its target SHA, and admin ping was `pong`. This proves stable rebuilt runtime health without another server restart; the real reconnect ordering remains covered by the App-level RED/GREEN test.

## Demand-Driven IPC Status Snapshots

Commit `abf68fd9` moves the eight expensive status snapshots out of unconditional InWorld `Update` work. IPC now orders `Receive → RefreshStatus → Dispatch`; queued commands select only their required network, terrain, sound, character, gear, appearance, roster, or map refresh. Idle updates with no IPC command perform no status-snapshot rebuild, and `Ping`/`Performance` request none. The duplicate map-sync registration in `game/networking/mod.rs` was removed. See [[procedural-cloud-regeneration]] for the exact dependency matrix and RED/GREEN evidence. The rebuilt status-demand client still consumed 369.05% of one core; without a matched pre-change workload this establishes no isolated status-refresh CPU effect and did not satisfy the Empty gate.

## Strict Empty Pacing

`281d291a` removes the former forced 100 ms / 10 FPS Empty limiter. Only the configured global frame-rate limit now applies; disabling it leaves Empty uncapped. Prior near-10-FPS Empty readings are historical capped data, not CPU-baseline or CPU-savings evidence. Canonical retirement and pending uncapped Green status: [[procedural-cloud-regeneration#strict-empty-pacing-retirement]].

## Who Query Runtime

`WhoPlugin` keeps query and response handling active across stages because IPC Who requests need immediate unavailable replies, while Lightyear inbound messages must be consumed before its `Last`-schedule receiver clear. Commit `2c265ffa` (`Avoid dirtying idle Who runtime`) fixes the idle sender path: `send_pending_queries` previously called `pop_front()` on an empty queue every `Update`, which advanced `WhoRuntimeState` change ticks. A read-only empty guard now skips the destructive loop while preserving FIFO sends, unavailable replies, inbound updates, reset cleanup, and later-stage behavior.

The behavioral RED/GREEN evidence is `/tmp/claude/game-engine-perf/who-idle-change-tick-red.log` and `/tmp/claude/game-engine-perf/who-idle-change-tick-green.log`. `cargo fmt` passed, and Rust-readability evidence is under `/tmp/claude/game-engine-perf/who-idle-change-tick-readability/`. This is a semantic idle-state correction, not a measured CPU-savings or Character-readiness claim.

## Multi-ADT Terrain Streaming (Planned Phase 3)

Server sends `LoadTerrain { tile_x, tile_y }` messages as player moves. Client `TerrainManager` tracks loaded tiles in a `HashMap<(u8,u8), Entity>` and despawns out-of-range tiles.

## Known Issues

**Remote login panic (2026-03-06)**: after netcode connection is established, lightyear server panics in `src/send/components.rs:1130` ("not yet implemented") on a `ReplicationMode::SingleSender` path. The server restarts under systemd and the client times out. This is a server-side replication bug, not a firewall issue — UDP traffic was confirmed to flow both ways. Fix: add diagnostics around `Replicate` insertion and `ReplicationSender` component on connect. See [remote-login-debug-2026-03-06.md](../remote-login-debug-2026-03-06.md).

## Sources

- [network-integration.md](../../network-integration.md) — phased integration plan, crate deps, phase deliverables
- [remote-login-debug-2026-03-06.md](../../remote-login-debug-2026-03-06.md) — remote login failure, lightyear replication panic
- [authentication.md](../../authentication.md) — auth flow, token storage, argon2, redb tables
- [replicated-unit-noops](../investigations/replicated-unit-noops.md) — empty-stage NOOP evidence and suppression commits
- [application network tick source](../../../src/network_tick.rs) — legacy main-thread logical cadence
- [application network dispatcher](../../../src/network_events.rs) — typed owned inbox/outbox registration and worker relays
- [network runtime foundations](../../../src/network_runtime/mod.rs) — worker, adapters, and replication mirror; not yet started by the binary
- [client networking source](../../../src/game/networking/mod.rs) — current main-world interpolation, replication receive, and client lifecycle
- [equipment source](../../../src/game/equipment/equipment.rs) — owner-specific render invalidation and coalescing
- [character-create source](../../../src/scenes/char_create/mod.rs) — local creation-result UI observer
- [event-driven application updates](../../specs/event-driven-application-updates.md) — behavior contract and focused proof
- [client disconnect handling](../../../src/game/networking/disconnect.rs) — initial disconnect-marker filtering and reconnect/reset behavior
- `game-engine` commit `0d215316` — ignore initial reconnect disconnect marker
- `/tmp/claude/game-engine/reconnect-fixed-runtime.json` and `reconnect-fixed-live-current.log` — fixed rebuilt-client stability, scene, IPC, and server identity proof
- [client NPC networking source](../../../src/game/networking/npc.rs) — event-driven visibility policy systems
- [client player networking source](../../../src/game/networking/player.rs) — semantic alive-state updates
- [client Who runtime source](../../../src/who.rs) — Who query/send/receive/reset behavior
- [server networking source](../../../../game-server/crates/server/src/networking.rs) — movement and gravity mutation boundaries
- `game-engine` commit `e745d35e` — event-driven NPC visibility
- `/tmp/claude/game-engine-perf/pre-ui-empty-508891a6-live.json` — connected empty-stage relaunch proof

## See Also

- [[ui-system]] — login UI that feeds into the auth flow
- [[terrain]] — terrain streaming (Phase 3 networking dependency)
- [[lore-knowledge-graph]] — server-side graph authority model
- [[replicated-unit-noops]] — empty-stage semantic no-op boundaries and suppression fixes
