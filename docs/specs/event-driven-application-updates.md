# Event-driven application updates

Application work is driven by a fixed network schedule, queued commands/messages, dirty data, and active simulation instead of registering every handler as frame work. Bevy rendering remains a separate concern; the small application-driver target is not a five-system limit on the renderer.

## What it must do

### Networking

- [ ] Advance the application network schedule at 60 logical ticks per second, independently of render-frame count.
- [ ] Gate Lightyear link/transport/message receive and send work to network-due frames without changing wire formats or the negotiated simulation tick rate.
- [ ] Use one incoming dispatcher over existing typed inboxes; only handlers with pending messages and satisfied eligibility run.
- [ ] Preserve per-inbox FIFO and deterministic handler registration order; run a handler once when multiple routed inboxes are ready.
- [ ] Use one outgoing dispatcher; heavy send handlers run only when their existing queues or dirty state require work.
- [ ] Preserve connection/reconnection lifecycle and actual client/server delivery.

### Equipment and other application work

- [ ] Reconcile equipment for affected entities on actual equipment mutation, replicated appearance change, or required model-data arrival/replacement—not by scanning all equipment every frame.
- [ ] Coalesce a batch of equipment commands to its final rendered state while retaining command/result ordering.
- [ ] Skip IPC dispatch before resolving heavy parameters when no command is pending; retain receive → requested status refresh → dispatch ordering.
- [ ] Keep required active movement/animation and render-loop work, while avoiding unrelated idle application handlers.

## How it works

- [CPU investigation](../wiki/investigations/empty-window-baseline.md).
- The network schedule shares the main ECS thread. Due ticks can run together at lower render cadences; it cannot progress while that thread is blocked. It is not a separate OS network thread.
- Existing Bevy maximum-delta policy bounds catch-up after a long stall. Network timeouts still use real time.
- Typed Lightyear buffers remain the source of truth. The dispatcher checks inbox readiness at network ticks; it does not introduce a second wire protocol or claim zero network polling.

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
- [ ] Remaining application registrations and idle-work measurement.

## Out of scope

Renderer FPS caps, CPU/worker limits, wire-format changes, production deployment, or claiming all Bevy internal systems have been reduced to five callbacks.
