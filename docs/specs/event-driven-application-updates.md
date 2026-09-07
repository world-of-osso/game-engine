# Event-driven application updates

Application work is driven by a fixed network schedule, queued commands/messages, dirty data, and active simulation instead of registering every handler as frame work. Bevy rendering remains a separate concern; the small application-driver target is not a five-system limit on the renderer.

## What it must do

### Networking

- [x] Add a 60 logical-ticks-per-second application network schedule whose tick count is independent of render-frame count.
- [x] Keep Lightyear link/transport/message maintenance on every frame so delta-based transport timers retain real elapsed-time behavior; defer only typed application inbox dispatch to due network ticks.
- [x] Add one incoming dispatcher over existing typed inboxes; only handlers with pending messages and satisfied eligibility run.
- [x] Preserve dispatcher registration order, per-inbox FIFO, and once-only invocation when multiple routes are ready.
- [x] Add one outgoing dispatcher; registered send work runs only when its queue or dirty-state predicate is ready.
- [ ] Prove integrated connection/reconnection lifecycle and actual client/server delivery after the remaining migration.

### Equipment and other application work

- [x] Reconcile equipment for affected entities on actual equipment mutation, replicated appearance change, or required model-data arrival/replacement—not by scanning all equipment every frame.
- [x] Coalesce a batch of equipment commands to its final rendered state while retaining command/result ordering.
- [x] Give collection/death replies and character-creation responses one consuming handler; notify the character-create UI locally after auth consumes its response.
- [x] Skip IPC dispatch before resolving heavy parameters when no command is pending; retain receive → requested status refresh → dispatch ordering.
- [ ] Keep required active movement/animation and render-loop work, while avoiding unrelated idle application handlers.

## How it works

- [CPU investigation](../wiki/investigations/empty-window-baseline.md).
- `NetworkTick` shares the main ECS thread. Due ticks can run together at lower render cadences and cannot progress while that thread is blocked; it is a logical 60 Hz cadence, **not an independent OS network thread**.
- Existing Bevy maximum-delta policy bounds catch-up after a long stall. The authoritative client/server simulation remains negotiated at **20 Hz**; this schedule does not alter it.
- Link, transport, and message maintenance remain per-frame because transport senders advance delta-based timers there. Typed application inbox contents are parked before Lightyear's `Last` clear on frames with no due tick, restored in `First`, then dispatched at logical network ticks. This preserves buffered message metadata without a second wire protocol.
- `1aec1091` added the initial tick driver and routed auth handlers. `fc82c5b7`, `0cf03d06`, `8969c18c`, `50b2df4e`, and `e56ce620` migrated application API handlers. `2a8abacb` preserves transport timers while deferring application inboxes; `71d80355` makes its cadence fixtures use Bevy manual time. `db7e6e7b` gates idle IPC dispatch before system parameter acquisition.

## Implementation inventory

- `src/network_tick.rs`: fixed-cadence driver and network I/O conditions.
- `src/network_events.rs`: registered handlers and centralized inbox/outbox dispatch.
- `src/game/networking/mod.rs`: application network registration.
- `src/game/equipment/equipment.rs`, `src/game/networking/player.rs`, `src/status_sync.rs`: equipment mutation and reconciliation boundaries.
- `src/ipc/plugin.rs`: empty-queue dispatch condition.

## Tests asserting this spec

- Network tests: **8/8** cover equal elapsed time at different render cadences, unchanged time, real message buffers parked through no-tick `Last` frames, once-only routing, and ordered outgoing work.
- API tests: **55/55** cover queue/readiness gating and existing state mapping across migrated handlers.
- Equipment agent tests: **13 passed** plus **2 appearance-event tests**; IPC FIFO test: **1/1**.
- Character-create response tests: **3/3** cover success, failure, and a response after the scene exits.

## Known gaps

- [ ] Integrated connection/reconnection and native delivery/appearance proof.
- [ ] Independent transport/thread execution. The current 60 Hz cadence is main-thread logical work only.
- [ ] Remaining non-network application work and active movement/animation scheduling review.
- [ ] Idle-work/CPU measurement. No CPU reduction or CPU fix is claimed by this groundwork.

## Out of scope

Renderer FPS caps, CPU/worker limits, wire-format changes, production deployment, or claiming all Bevy internal systems have been reduced to five callbacks.
