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

Two client systems previously performed unconditional same-value writes every frame: remote interpolation rewrote `Transform`, and NPC policy evaluation rewrote `Visibility`. Lightyear equality-suppresses the final client ECS replacement for equal replicated components, but server-side same-value `Rotation`/`MovementSpeed` input writes and grounded gravity state still entered change detection, serialization, and transmission. Actual wander movement remains real `Position` change, not a replication NOOP. The current nearby movement-type-2 NPCs have no waypoint rows, so waypoint-delay behavior is not implicated in this workload.

Conditional client writes and server state updates are committed in `3c77d346`, `2927382`, and `ae81c65`, with regression tests for stable and changing state. The corrected empty-stage client relaunch remained connected with one link, one local player, and 133 remote entities, without the prior UI log flood; the recorded performance samples are not comparative evidence. See [[replicated-unit-noops]].

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
- [server networking source](../../../../game-server/crates/server/src/networking.rs) — movement and gravity mutation boundaries
- `/tmp/claude/game-engine-perf/pre-ui-empty-508891a6-live.json` — connected empty-stage relaunch proof

## See Also

- [[ui-system]] — login UI that feeds into the auth flow
- [[terrain]] — terrain streaming (Phase 3 networking dependency)
- [[lore-knowledge-graph]] — server-side graph authority model
- [[replicated-unit-noops]] — empty-stage semantic no-op boundaries and suppression fixes
