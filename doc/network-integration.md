# Network Integration Plan: game-engine ↔ game-server

Bridges game-engine (3D client) with game-server (headless Bevy + lightyear).
End goal: connect → see world with other players and NPCs.

Login/UI is handled separately — not in scope here.

## Current State

### game-server (working)
- Headless Bevy server, lightyear 0.26, 20Hz tick, UDP on :5000
- Shared crate: `Position`, `Rotation`, `Health`, `Mana`, `MovementSpeed`, `Player`, `Npc`, `Zone`
- Protocol: `MovementChannel` (unreliable), `CombatChannel` + `ChatChannel` (reliable ordered)
- World data: SQLite loader (AzerothCore creature templates, spawns, waypoints)
- Persistence: redb skeleton (DB opens, tables created, no save wired)

### game-server (stubs)
- `client/` crate — empty placeholder
- `zones.rs` — empty `update_zones()`
- No player spawn on connect
- No creature ECS spawning from WorldData
- No input handling from clients

### game-engine (working)
- Full 3D: M2 models, ADT terrain, WMOs, water, animation, distance culling
- FPS camera + WASD movement with terrain collision
- IPC server (Unix socket for CLI commands)

### game-engine (missing)
- Zero lightyear integration
- Movement is local-only (direct Transform mutation)
- No concept of remote entities

---

## Phase 1: Minimal Network Loop

**Goal**: game-engine connects to game-server, server spawns a player entity,
client sees its own replicated position. Auto-connect on launch (no login screen).

### 1a. Server: Spawn player on connect

**File**: `game-server/crates/server/src/networking.rs`

- In `on_client_connected`: spawn entity with `Position`, `Health`, `Player`
- Set initial position to Elwynn Forest (hardcoded)
- Mark entity for replication via lightyear

### 1b. Server: Accept movement inputs

**File**: `game-server/crates/shared/src/protocol.rs`

- Define `PlayerInput` message type (direction: Vec3, jump: bool)
- Register on `CombatChannel` (reliable, client→server)
- Server system: apply validated movement from inputs to `Position`

### 1c. game-engine: Add lightyear client

**File**: `game-engine/Cargo.toml` + new `src/networking.rs`

- Add `lightyear` dep with `client`, `netcode`, `replication`, `udp` features
- Add `shared` crate as path dependency
- `NetworkPlugin`: adds `ClientPlugins`, `ProtocolPlugin`
- Startup system: spawn client entity with `NetcodeClient`, `ReplicationReceiver`
- Auto-connect to `127.0.0.1:5000`

### 1d. game-engine: Receive replicated entities

**File**: `game-engine/src/networking.rs`

- System: detect `Added<Position>` with `Replicated` marker
- For own player: attach camera follow
- For other entities: spawn placeholder mesh

### 1e. game-engine: Send movement inputs

**File**: `game-engine/src/camera.rs` (refactor)

- Extract input capture from `player_movement()` into `capture_input()` → `PlayerInput`
- Send `PlayerInput` via lightyear `MessageSender`
- Apply locally for prediction (keep current movement logic)
- Reconcile when server sends authoritative `Position`

**Deliverable**: Run server + client. Client connects, player spawns at Elwynn,
WASD moves with server authority.

---

## Phase 2: Creature Spawning

**Goal**: Server spawns creatures from WorldData, client renders them as M2 models.

### 2a. Server: Spawn creatures into ECS

**File**: `game-server/crates/server/src/zones.rs`

- On startup (after WorldData loads): iterate `spawns_by_map[0]` (Eastern Kingdoms)
- Spawn ECS entities with `Position`, `Npc { template_id }`, `Health`, `Zone`
- Replicate `Npc` component
- Start with Elwynn Forest zone only (not all 100K spawns)

### 2b. Shared: Model resolution

**File**: `game-server/crates/shared/src/components.rs`

- Add `ModelDisplay { display_id: u32 }` component (replicated)
- Server resolves `creature_template.models[0].display_id` at spawn time
- Client maps `display_id` → FDID → M2 file (needs CreatureDisplayInfo DB2 or lookup table)

### 2c. game-engine: Spawn M2 for replicated NPCs

**File**: `game-engine/src/networking.rs`

- System: on `Added<Npc>` with `Replicated` — resolve model, spawn M2 mesh
- Position from replicated `Position` component
- Idle animation (Stand=0)

**Deliverable**: Connect, see NPCs standing in Elwynn Forest at their spawn points.

---

## Phase 3: Multi-ADT Streaming

**Goal**: Load terrain tiles as player moves through the world.

### 3a. Server: Tell client which ADTs to load

**File**: `game-server/crates/shared/src/protocol.rs`

- Define `LoadTerrain { tile_x: u8, tile_y: u8 }` message
- Server sends when player enters range of a new tile
- Interest management: 3×3 grid around player's current tile

### 3b. game-engine: Multi-tile terrain manager

**File**: `game-engine/src/terrain.rs` (refactor)

- `TerrainManager` resource: tracks loaded tiles as `HashMap<(u8,u8), Entity>`
- On `LoadTerrain` message: extract ADT from CASC if missing, spawn terrain
- On tile out of range: despawn terrain + doodads + WMOs
- Extend `TerrainHeightmap` to multiple tiles (or per-tile heightmaps)

**Deliverable**: Walk from Goldshire to Stormwind, terrain streams in.

---

## Phase 4: Persistence & Polish

### 4a. Server: Wire persistence

**File**: `game-server/crates/server/src/persistence.rs`

- Implement `save_dirty_characters` system (already designed in ARCHITECTURE.md)
- Run every 300 ticks (5 minutes at 20Hz)
- Save on disconnect
- Load character data on connect

### 4b. Server: Zone system

**File**: `game-server/crates/server/src/zones.rs`

- Filter spawns by zone
- Interest management: only replicate entities within render distance
- Zone transition events

### 4c. Client: Interpolation for remote entities

- Smooth position updates for other players (lerp between server ticks)
- Animation state from replicated `MovementState`

---

## Dependency Graph

```
Phase 1a (server: player spawn)  ─┐
Phase 1b (shared: input protocol) ─┤──→ Phase 1c (client: lightyear) ──→ Phase 1d+1e (replicate + input)
                                   │                                            │
Phase 2a (server: creatures) ──────┤──→ Phase 2b (shared: model display) ──→ Phase 2c (NPC meshes)
                                   │                                            │
Phase 3a (server: terrain msgs) ───┘──────────────────────────────────────→ Phase 3b (multi-ADT)
                                                                                │
Phase 4a (persistence) ──→ Phase 4b (zones) ──→ Phase 4c (interpolation)
```

Phases 1-3 can progress in parallel after Phase 1c (lightyear client) is done.

## Crate Dependencies

```
game-server/crates/shared/     ← both server and client depend on this
  └── lightyear (replication)
  └── bevy, serde, bitcode

game-server/crates/server/     ← headless binary
  └── shared
  └── lightyear (server, netcode, udp)
  └── redb, rusqlite

game-engine/                    ← 3D client binary
  └── shared (path = "../game-server/crates/shared")
  └── lightyear (client, netcode, replication, udp)
  └── bevy (full rendering)
```

game-engine uses shared directly. The game-server/client crate is unused —
game-engine IS the client.
