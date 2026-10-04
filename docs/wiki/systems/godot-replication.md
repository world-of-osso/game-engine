# Godot Replication

The Godot client receives bevy_replicon 0.41.1 replication without replicon's client or any ECS copy of component data. The network worker forwards replicon's raw payloads and acknowledges mutations. `game_engine_network::replica::Replica`, owned by `GameClient` on the main thread, decodes them straight into typed per-component storage. That storage is the client's only copy of replicated state; `WorldUnits` nodes and the HUD read it.

## Content

### Wire path (unchanged server, unchanged protocol)

- **Lightyear bridge.** The server's `send_server_packets` prefixes each replicon Updates/Mutations payload with a 7-byte checkpoint header (`LY`, version 1, LE u32 tick). The worker strips it with `lightyear_replication::checkpoint::unwrap_server_payload`, the same call lightyear's `receive_client_packets` makes.
- **Update message.** `flags: u8` (MAPPINGS 1, DESPAWNS 2, REMOVALS 4, CHANGES 8), then the message `RepliconTick`, then one array per set flag in bit order. Every array except the last set flag carries a varint length; the last runs to the message end. Mapping entries are entity plus a fixint LE u64 signature hash; the client spawns no signature entities, so every hash is unknown and skipped, as replicon's client skips unknown hashes. Despawn entries are an entity. Removal and change entries are entity, varint data size, then `FnsId` varints (removals) or `FnsId` plus postcard component bytes (changes). Component bytes have no length, so every registered `FnsId` must decode.
- **Entity encoding** (`compact_entity`): varint `index << 1 | has_generation`, then a varint generation when set. `server_id` is the server `Entity::to_bits()`.
- **Mutate message.** `update_tick`, `message_tick`, `messages_count` (present because the server tracks mutate messages: lightyear's `SendPlugin` enables it), fixint LE u16 `MutateIndex`, then change-style entries. The server sends an empty one every tick.
- **Acks.** The worker concatenates the 2-byte `MutateIndex` of every mutate message received in a frame and sends it on replicon's `MutationAcks` channel in that frame, as replicon's client does before applying. Empty mutate messages are acked, then dropped before reaching the host.
- **Apply order** (`bevy_replicon::client::receive_replication`): update messages first; mutate messages are buffered sorted by `message_tick` (newest first) and applied once `update_tick <= ServerUpdateTick`; a mutation older than or equal to the entity's last applied tick is skipped. Removals for an unknown server entity are an error. Replicon's client logs a malformed message and continues; `Replica::apply` returns the error, which the host reports as a frame error.

### Protocol fingerprint

`client_app` builds lightyear's `ClientPlugins` with `LightyearRepliconClientBackend` and `PredictionPlugin` disabled. Replicon's `ProtocolHash` comes from the shared registrations and `track_mutate_messages`, which lightyear's `InterpolationPlugin` (inside `SharedPlugins`) enables before the disabled backend would; the backend's own call is a no-op duplicate. `replacing_replicon_client_keeps_the_protocol_fingerprint` compares message/channel hashes and `ProtocolHash` with the stock client app; `full_wire_fingerprint_matches_stock_client_including_layout_hashes` compares the full `ProtocolFingerprint`, layout hashes included, both clients sending to one real server.

### Schema and codecs

`Schema::from_world` reads the worker's `ReplicationRules` (the registrations behind the protocol hash) and maps each `FnsId` to a codec in `replica/codec.rs` by `TypeId`. The codec list has one line per type: lightyear's `Predicted`, `Interpolated`, `Controlled`, `ChildOf` (its custom entity encoding) and every `shared` replicated component. A registered type without a codec fails the worker at connect, naming the type. A new replicated component in `shared` therefore needs one `Codec::of::<T>()` line.

### Store and host projection

- Components live in one column per type (`Vec<Option<C>>` indexed by a reusable entity slot); a write reuses the slot (the column grows only when a new slot index appears), so component inserts and mutations need no per-component heap allocation beyond the value's own `String`/`Vec` fields. `Unit::get::<C>()` reads them.
- `drain_changes` returns `Changed { server_id, components }` (a bitset of changed codecs) per entity, in first-change order, and `Despawned(server_id)`.
- `GameClient::project_replication` upserts the world node of each changed player or creature (`replicated::is_unit`), calls `aura_set_changed` when `UnitAuras` changed, and removes the node of an entity that despawned or stopped being a unit. `AccountEvent::ReplicationEnded` (on `Disconnected`) and a new connection's `ReplicationStarted` clear the store.
- `godot/rust/src/replicated.rs` derives the values several HUD systems read (`in_combat`, `faction_template`, `unit_target`, `threat_list`, ...).

## Proof

- `replica::tests` (Depot `--test -p game-engine-network`): a real lightyear/replicon server replicates to a stock lightyear client over loopback UDP; a tap brackets stock replicon's receive calls in Connected `PreUpdate` and `OnEnter(Connected)`, recording the consumed payloads and their acknowledgments. `Replica` applies the same frames and must hold byte-identical values of every registered component for the same server entities after each frame: spawn, insert, 10 mutations, removal, visibility loss, despawn. Reordered replays (all mutations before any update; mutations newest first, one per frame) run through replicon's own `ClientPlugin` and `Replica` side by side. Acks equal replicon's bytes.
- `replica::coverage` tests: fingerprint with layout hashes, missing codec, malformed flags, unknown `FnsId`, truncated data, real signature mappings, visibility regain before a drain, slot reuse after despawn, packet-split same-tick mutations, malformed mutate headers not acknowledged.
- `wire_tests`: the bridge's own worker against a real server, including the server receiving `MutationAcks`.
- Live on a private server (UDP 5095, game-server `a6a704f`, shared-protocol `2acb8a9`, engine `448c9339`): `world_chat_flow`, `auras_live`, `polymorph_mob` and `spellcast_anim` exit 0. `world_target_flow` passes selection, server echo, Escape and Tab; it fails only its TargetFrame geometry check (frame 88.7×34 at UI scale 0.667 vs expected 124.7×47.8), which fails identically on master `c92480ba`.

### Connection-transition acknowledgment capture

The old test Tap captured `ClientMessages` unconditionally in `PreUpdate` and paired them with outgoing acknowledgments in `PostUpdate`. Replicon's `ClientPlugin` only consumes in Connected `PreUpdate`, plus `OnEnter(Connected)`. A packet arriving while Connecting therefore produced a captured mutation with no acknowledgment, and could be captured repeatedly before the transition consumed it. Scheduling delays made this ordering intermittent in UDP tests; it was a harness defect, not a worker acknowledgment defect.

`acknowledgment_capture_follows_connection_transition_consumption` queues the concrete empty mutation `[0, 0, 1, 0, 0]` while Connecting and then enters Connected. The original Tap deterministically fails with expected `[b"\0\0"]`, captured `[]`. Capture now brackets both actual consumption schedules and follows the same Connected condition; the byte-equality assertion, worker, sleeps and deadlines are unchanged.

## Measured cost (2026-09-30)

Northshire start, about 93 replicated players/creatures, standing still, debug extension, 10 s windows after world load (median across windows), master `c92480ba` vs this branch, both with the same temporary probe (not committed): worker thread CPU from `/proc/self/task`, allocations counted by a global allocator on the worker thread and inside the host's unit handling (old: `UnitUpdated`/`UnitRemoved` arms; new: `apply_replication`), time of that handling per frame. The machine was shared with other agents' clients (load 15-27), so absolute numbers vary between runs; the allocation rates were stable.

| | master | branch |
|---|---|---|
| Worker CPU | 103-146 ms/s | 62-113 ms/s |
| Worker allocations | 1440-1530/s, 417-439 KB/s | 660-666/s, 134-137 KB/s |
| Host unit handling, per-frame p50 | 317-734 µs | 509-1089 µs |
| Host unit handling allocations, per-frame p50 | 12-18 | 19-20 |

Decoding moved from the worker to the main thread, so the main thread pays about 0.2-0.35 ms more per frame at the p50 while the worker's work and allocation traffic roughly halve. Means are dominated by synchronous model loads inside `WorldUnits::upsert` in both builds (seconds-long frames). Unoptimized, the `game-engine-network` decoder cost more on the main thread; the dev profile now builds it at opt-level 2 like `game-engine-core`.

## Sources

- `godot/network/src/replica/` — decoder, store, worker receive, tests
- `~/.cargo/registry/src/*/bevy_replicon-0.41.1/src/client.rs` — reference receive logic
- `~/.cargo/registry/src/*/lightyear_replication-0.28.0/src/{client,checkpoint,channels}.rs` — lightyear bridge

## See Also

- [[networking]] — Bevy client networking and the server side
- [[godot-conversion]] — Godot client architecture
