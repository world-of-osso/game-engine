# Event-driven application updates

Application work is driven by a fixed network schedule, queued commands/messages, dirty data, and active simulation instead of registering every handler as frame work. Bevy rendering remains a separate concern; the small application-driver target is not a five-system limit on the renderer.

## What it must do

### Networking

- [x] Add a 60 logical-ticks-per-second application network schedule whose tick count is independent of render-frame count.
- [x] Run Lightyear link/transport/message maintenance in a dedicated 60 Hz worker so it retains real elapsed-time behavior independently of main-world/render updates.
- [x] Add one incoming dispatcher over application-owned typed inboxes; only handlers with pending messages and satisfied eligibility run.
- [x] Preserve dispatcher registration order, per-inbox FIFO, and once-only invocation when multiple routes are ready.
- [x] Add one outgoing dispatcher; registered send work runs only when its queue or dirty-state predicate is ready. Application handlers use worker-backed typed sender/receiver adapters; the main lifecycle starts one dedicated worker per connection.
- [x] Prove historical actual client/server login plus routed Who/friends replies. Reconnection and full reply coverage remain unproven.

### Equipment and other application work

- [x] Reconcile equipment for affected entities on actual equipment mutation, replicated appearance change, or required model-data arrival/replacement—not by scanning all equipment every frame.
- [x] Coalesce a batch of equipment commands to its final rendered state while retaining command/result ordering.
- [x] Give collection/death replies and character-creation responses one consuming handler; notify the character-create UI locally after auth consumes its response.
- [x] Skip IPC dispatch before resolving heavy parameters when no command is pending; retain receive → requested status refresh → dispatch ordering.
- [x] Move confirmed idle application work to queued, change-driven, relevance-driven, or 60 Hz logical-tick execution; retain rendering, interpolation, animation, and active cooldown presentation where needed.

## How it works

- [CPU investigation](../wiki/investigations/empty-window-baseline.md).
- `NetworkTick` is legacy main-world application cadence. Live Lightyear transport runs in the dedicated 60 Hz worker and continues when the main world is blocked.
- `a35b1c5c` through `01cfcade` add the separately clocked 60 Hz worker, worker-backed `MessageSenders`/`MessageReceivers`, and application-owned `Inbox<M>` FIFO dispatch. `aa6fda57` moves live Lightyear client, UDP transport, protocol registration, replication receiver, and typed receiver relays into one worker world per connection. The old main-world receiver park/restore path is removed.
- `cec56837` and `aa6fda57` bridge replicated snapshots, despawns, and server-entity-to-render-entity identity into the main world. Main-world `Client`, `Connected`, and `Disconnected` markers are lifecycle proxies; they do not own Lightyear transport state. `52508d1d` maps target/emote/combat entity fields at the worker/main boundary; `a5f6eab0` maps duel, inspect, and current/default spell targets. Explicit numeric spell selectors remain server IDs.
- The worker runs its app at 60 Hz and retains Lightyear's **20 Hz** simulation interval. Main-world receive, apply, send, reconnect, reset, and active spellbook cooldown work run in `NetworkTick`; no wire format or negotiated simulation behavior changes.
- This is an implementation boundary, not completion proof. A UDP/replication bridge run exposed Bevy B0002 from an `EntityRef` resource query; `54411453` excludes resources. `independent-udp-handshake.log` records **1/1** real UDP handshake proof without a main-app update; `worker-restart-tests.log` records **3/3** shutdown cleanup and second-worker handshake cases. `worker-char-create-response.log`, `worker-auth.log`, `wire-identity.log`, and `migrated-ui-reconnect-fixtures.log` record scoped **3/3**, **23/23**, **19/19**, and **11/11** results. A native client reached InWorld with player/NPC mirrors and Who result `Theron`, one result; dark scene/white UI leaves visual proof invalid. Complete replication/equipment lifecycle and CPU proof remain pending.
- `1aec1091` added the initial tick driver and routed auth handlers. `fc82c5b7`, `0cf03d06`, `8969c18c`, `50b2df4e`, and `e56ce620` migrated application API handlers. `db7e6e7b` gates idle IPC dispatch before system parameter acquisition.

## Implementation inventory

- `src/network_tick.rs`: legacy main-thread logical application driver; it does not own live transport.
- `src/network_events.rs`: registered handlers plus centralized application-owned `Inbox<M>`/outbox dispatch and worker-relay registration.
- `src/network_runtime/worker.rs`, `messages.rs`, `replication.rs`, `connection.rs`: dedicated worker, typed queues, replication mirror, and per-connection lifecycle bridge.
- `src/game/networking/mod.rs`: starts the worker connection; main reconnect/reset lifecycle work runs at `NetworkTick` cadence. Remote interpolation remains render-frame-driven.
- `src/game/equipment/equipment.rs`, `src/game/networking/player.rs`, `src/status_sync.rs`: equipment and local-player work use mutation/component-change boundaries.
- `src/ui/game_plugin.rs`, `automation.rs`, `addon_runtime/mod.rs`: UI sync/pointer, automation, addon application, and active cooldown work are change/request/active-cooldown driven.
- `src/sound/runtime.rs`, `runtime_ambient.rs`, `runtime_music.rs`: footstep attachment is relevance-driven; ambient/music reconciliation follows input or playback removal.
- `src/ipc/plugin.rs`: empty-queue dispatch condition.

## Tests asserting this spec

- Historical main-thread transport tests: **8/8** cover equal elapsed time at different render cadences, unchanged time, no-tick buffering, once-only routing, and ordered outgoing work; they do not prove the new worker lifecycle.
- `runtime-tests-transport.log`: **13/13** worker/module tests, including real Lightyear packet loopback while the main app is not updated, FIFO worker commands/replies, and shutdown/failure behavior.
- `owned-inbox-red-behavior.log` then `owned-inbox-green.log`: RED followed by **8/8** application-owned inbox dispatcher tests for FIFO, readiness, reset clearing, and queue gating.
- API tests: historical **55/55** cover queue/readiness gating and existing state mapping; the migrated adapter call sites still require full lifecycle integration proof.
- Equipment agent tests: **13 passed** plus **2 appearance-event tests**; IPC FIFO test: **1/1**.
- `worker-char-create-response.log`: **3/3** success, failure, and response-after-scene-exit tests through a separate worker App with real transport encode/decode and relay to the main inbox.
- `worker-restart-tests.log`: **3/3** worker shutdown cleanup and restart tests; `migrated-ui-reconnect-fixtures.log`: **11/11** UI/reconnect fixture checks.

## Native evidence (2026-09-07)

- Empty-stage login reached InWorld. A 10-second unfocused sample recorded **265.779%** one-core CPU and **503.746 application updates/s**. The prior intact Empty sample recorded **293.279%** and **422.366 updates/s**. Different clock ranges and runs make this non-causal; it does not establish an idle-work reduction or CPU fix.
- A full-world client logged in, entered the world, and completed routed Who and friends requests. Its visual smoke is invalid: repeated `bevy_render::slab_allocator` unallocated-key errors and a white/dark screenshot occurred. The same error predates this work in `data/diagnostics/movement-perf-20260905/connected-warm2/client.log:149`; no equipment-event causality is established. Test clients were stopped.

## Known gaps

- [ ] Complete connected API coverage, replicated component/equipment lifecycle proof against the server, and clean native appearance proof.
- [ ] Verify the integrated dedicated network world. It exclusively owns Lightyear client/transport/replication state and bridges typed outgoing commands, incoming FIFO data, replicated snapshots, and connection state into the render world; focused real UDP handshaking is proven, but end-to-end lifecycle and native proof remain incomplete.
- [ ] Convert remaining wire entity-bit boundaries between server identity and render entities. Target, duel, inspect, spell current/default, emote, and combat now map explicitly; related UI and remaining protocol fields need an inventory.
- [x] Review confirmed non-render application frame work: UI, automation, addons, local-player state, reconnect lifecycle, and sound maintenance no longer perform their prior idle scans/application.
- [ ] Controlled idle-work/CPU measurement. No CPU reduction or CPU fix is claimed by this groundwork.

## Out of scope

Renderer FPS caps, CPU/worker limits, wire-format changes, production deployment, or claiming all Bevy internal systems have been reduced to five callbacks.
