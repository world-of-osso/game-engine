# Skirmish results Queue Again

The native results screen exposes Queue Again for the finished skirmish. Retail PVPMatchResults.lua152–159/490–494 supplies visibility and immediate-disable authority. Server ordering is the explicit project policy `SKIRMISH_REQUEUE_LEAVE_THEN_QUEUE_SAME_MODE`, not a proven Retail native transition; retain `SKIRMISH_REQUEUE_TRANSITION_UNQUALIFIED`. [UI flow](../wiki/systems/battlegrounds.md#skirmish-queue-again--2026-10-20).

## What it must do

- [x] Show a labeled Queue Again action beside Leave for accepted Ended Skirmish2v2/Skirmish3v3 in both skins.
- [x] Hide/refuse it for ordinary BG, rated arena, SoloShuffle, unaccepted and active/preparation states.
- [x] Disable immediately on click and dispatch one `RequeueSkirmish` request carrying the current match identity only; repeated pending clicks dispatch nothing.
- [x] Do not issue a client-side Leave followed by QueueForArena. The server owns ordinary return, successful origin ACK and admission of the actual played mode.
- [ ] Preserve normal queue error display after a failed admission at the returned origin.

## How it works

- [UI flow](../wiki/systems/battlegrounds.md#skirmish-queue-again--2026-10-20).

## Implementation inventory

- `godot/ui-model/src/battleground.rs` — result visibility, pending state and match-bound action.
- `godot/rust/src/account.rs` — one typed request dispatch.
- `godot/ui-model/tests/battleground_requeue.rs` — both-skin registry/action controls.

## Tests asserting this spec

- `godot/ui-model/tests/battleground_requeue.rs`, included by `tests/integration.rs`.
- Native server lifecycle tests own return/ACK/admission ordering, not a UI mock queue.

## Known gaps (current cycle)

- [ ] Queue-error display after a failed admission at origin reuses the existing PvP error path; no UI test drives that server reply.
- [ ] `godot/rust/src/account.rs` dispatch compile/runtime not exercised by these ui-model tests; rendered/live client unproven.

## Out of scope

- Rated/BG/Shuffle Queue Again, changed queue policy/rewards and claims about Retail's hidden native ordering.
- No UI redesign, new art or client-side leave/queue race.
