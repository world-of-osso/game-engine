# Event-driven application updates

Application work is driven by a fixed network schedule, queued commands/messages, dirty data, and active simulation instead of registering every handler as frame work. Bevy rendering remains a separate concern; the small application-driver target is not a five-system limit on the renderer.

## What it must do

### Networking

- [x] Add a 60 logical-ticks-per-second application network schedule whose tick count is independent of render-frame count.
- [x] Gate Lightyear link/transport/message receive and send sets to network-due frames without changing wire formats or the negotiated 20 Hz simulation tick.
- [x] Add one incoming dispatcher over existing typed inboxes; only handlers with pending messages and satisfied eligibility run.
- [x] Preserve dispatcher registration order, per-inbox FIFO, and once-only invocation when multiple routes are ready.
- [x] Add one outgoing dispatcher; registered send work runs only when its queue or dirty-state predicate is ready.
- [ ] Prove integrated connection/reconnection lifecycle and actual client/server delivery after the remaining migration.

### Equipment and other application work

- [ ] Reconcile equipment for affected entities on actual equipment mutation, replicated appearance change, or required model-data arrival/replacement—not by scanning all equipment every frame.
- [ ] Coalesce a batch of equipment commands to its final rendered state while retaining command/result ordering.
- [x] Skip IPC dispatch before resolving heavy parameters when no command is pending; retain receive → requested status refresh → dispatch ordering.
- [ ] Keep required active movement/animation and render-loop work, while avoiding unrelated idle application handlers.

## How it works

- [CPU investigation](../wiki/investigations/empty-window-baseline.md).
- `NetworkTick` shares the main ECS thread. Due ticks can run together at lower render cadences and cannot progress while that thread is blocked; it is a logical 60 Hz cadence, **not an independent OS network thread**.
- Existing Bevy maximum-delta policy bounds catch-up after a long stall. Network timeouts still use real time. The authoritative client/server simulation remains negotiated at **20 Hz**; this schedule does not alter it.
- Typed Lightyear buffers remain the source of truth. The dispatcher checks inbox readiness at logical network ticks; it does not introduce a second wire protocol or claim zero network polling.
- `1aec1091` added the initial tick driver and routed auth handlers. `fc82c5b7` routed profession request/update work. `db7e6e7b` gates idle IPC dispatch before system parameter acquisition.

## Implementation inventory

- `src/network_tick.rs`: fixed-cadence driver and network I/O conditions.
- `src/network_events.rs`: registered handlers and centralized inbox/outbox dispatch.
- `src/game/networking/mod.rs`: application network registration.
- `src/game/equipment/equipment.rs`, `src/game/networking/player.rs`, `src/status_sync.rs`: equipment mutation and reconciliation boundaries.
- `src/ipc/plugin.rs`: empty-queue dispatch condition.

## Tests asserting this spec

- Network clock tests compare equal elapsed time at different render cadences.
- Dispatcher tests cover empty input, real message buffers, once-only routing, and ordered outgoing work.
- Equipment tests cover mutation, late model data, replacement, and unchanged state.
- IPC queue tests cover idle dispatch and FIFO behavior.

## Known gaps

- [ ] Integration and native delivery/appearance proof.
- [ ] Broad application-handler migration: auth and profession are routed; other gameplay/API registrations remain on frame `Update` or await conversion.
- [ ] Idle-work/CPU measurement. No CPU reduction or CPU fix is claimed by this groundwork.

## Out of scope

Renderer FPS caps, CPU/worker limits, wire-format changes, production deployment, or claiming all Bevy internal systems have been reduced to five callbacks.
