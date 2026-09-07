# Event-driven application updates

Application work is driven by a fixed network schedule, queued commands/messages, dirty data, and active simulation instead of registering every handler as frame work. Bevy rendering remains a separate concern; the small application-driver target is not a five-system limit on the renderer.

## What it must do

### Networking

- [x] Add a 60 logical-ticks-per-second application network schedule whose tick count is independent of render-frame count.
- [x] Keep Lightyear link/transport/message maintenance on every frame so delta-based transport timers retain real elapsed-time behavior; defer only typed application inbox dispatch to due network ticks.
- [x] Add one incoming dispatcher over application-owned typed inboxes; only handlers with pending messages and satisfied eligibility run.
- [x] Preserve dispatcher registration order, per-inbox FIFO, and once-only invocation when multiple routes are ready.
- [x] Add one outgoing dispatcher; registered send work runs only when its queue or dirty-state predicate is ready. Application handlers use worker-backed typed sender/receiver adapters; the main client lifecycle has not started the worker yet.
- [x] Prove one actual client/server login plus routed Who/friends replies. Reconnection and full reply coverage remain unproven.

### Equipment and other application work

- [x] Reconcile equipment for affected entities on actual equipment mutation, replicated appearance change, or required model-data arrival/replacement—not by scanning all equipment every frame.
- [x] Coalesce a batch of equipment commands to its final rendered state while retaining command/result ordering.
- [x] Give collection/death replies and character-creation responses one consuming handler; notify the character-create UI locally after auth consumes its response.
- [x] Skip IPC dispatch before resolving heavy parameters when no command is pending; retain receive → requested status refresh → dispatch ordering.
- [ ] Keep required active movement/animation and render-loop work, while avoiding unrelated idle application handlers.

## How it works

- [CPU investigation](../wiki/investigations/empty-window-baseline.md).
- `NetworkTick` shares the main ECS thread. Due ticks can run together at lower render cadences and cannot progress while that thread is blocked; it is a logical 60 Hz cadence, **not an independent OS network thread**.
- `a35b1c5c` through `01cfcade` add an unwired dedicated-world foundation: a separately clocked 60 Hz worker preserving the 20 Hz client simulation, worker-backed `MessageSenders`/`MessageReceivers`, and application-owned `Inbox<M>` FIFO dispatch. Worker relays append owned message batches to the main world; old main-world Lightyear receiver park/restore is removed.
- `cec56837` adds an unintegrated replication mirror foundation keyed by server entity identity. It must still be connected to worker lifecycle, render-world identity mapping, and reconnect cleanup before it affects production behavior.
- Existing Bevy maximum-delta policy bounds catch-up after a long stall. The authoritative client/server simulation remains negotiated at **20 Hz**; neither foundation alters it.
- The main binary does not yet start a worker or move its live client, transport, replication, connection markers, or reconnect lifecycle. The current path is therefore not runnable as independent network execution.
- `1aec1091` added the initial tick driver and routed auth handlers. `fc82c5b7`, `0cf03d06`, `8969c18c`, `50b2df4e`, and `e56ce620` migrated application API handlers. `db7e6e7b` gates idle IPC dispatch before system parameter acquisition.

## Implementation inventory

- `src/network_tick.rs`: legacy main-thread logical application driver; it remains unable to progress during a main-thread block.
- `src/network_events.rs`: registered handlers plus centralized application-owned `Inbox<M>`/outbox dispatch and worker-relay registration.
- `src/network_runtime/worker.rs`, `messages.rs`, `replication.rs`: committed dedicated-world, typed queue, and replication-mirror foundations. The main binary has not integrated their lifecycle.
- `src/game/networking/mod.rs`: application network registration.
- `src/game/equipment/equipment.rs`, `src/game/networking/player.rs`, `src/status_sync.rs`: equipment mutation and reconciliation boundaries.
- `src/ipc/plugin.rs`: empty-queue dispatch condition.

## Tests asserting this spec

- Historical main-thread transport tests: **8/8** cover equal elapsed time at different render cadences, unchanged time, no-tick buffering, once-only routing, and ordered outgoing work; they do not prove the new worker lifecycle.
- `runtime-tests-transport.log`: **13/13** worker/module tests, including real Lightyear packet loopback while the main app is not updated, FIFO worker commands/replies, and shutdown/failure behavior.
- `owned-inbox-red-behavior.log` then `owned-inbox-green.log`: RED followed by **8/8** application-owned inbox dispatcher tests for FIFO, readiness, reset clearing, and queue gating.
- API tests: historical **55/55** cover queue/readiness gating and existing state mapping; the migrated adapter call sites still require full lifecycle integration proof.
- Equipment agent tests: **13 passed** plus **2 appearance-event tests**; IPC FIFO test: **1/1**.
- Character-create response tests: **3/3** cover success, failure, and a response after the scene exits.

## Native evidence (2026-09-07)

- Empty-stage login reached InWorld. A 10-second unfocused sample recorded **265.779%** one-core CPU and **503.746 application updates/s**. The prior intact Empty sample recorded **293.279%** and **422.366 updates/s**. Different clock ranges and runs make this non-causal; it does not establish an idle-work reduction or CPU fix.
- A full-world client logged in, entered the world, and completed routed Who and friends requests. Its visual smoke is invalid: repeated `bevy_render::slab_allocator` unallocated-key errors and a white/dark screenshot occurred. The same error predates this work in `data/diagnostics/movement-perf-20260905/connected-warm2/client.log:149`; no equipment-event causality is established. Test clients were stopped.

## Known gaps

- [ ] Reconnection lifecycle, complete connected API coverage, and clean native appearance proof.
- [ ] Independent network-world/thread execution. Integrate the committed worker so it exclusively owns Lightyear client/transport/replication state; bridge typed outgoing commands, incoming FIFO data, replicated entity/component snapshots, connection state, and reconnect cleanup into the render world. The current live path remains main-thread logical application work with frame-driven transport.
- [ ] Remaining non-network application work and active movement/animation scheduling review.
- [ ] Idle-work/CPU measurement. No CPU reduction or CPU fix is claimed by this groundwork.

## Out of scope

Renderer FPS caps, CPU/worker limits, wire-format changes, production deployment, or claiming all Bevy internal systems have been reduced to five callbacks.
