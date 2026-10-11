# Quest sharing, resets and timed quests

Native quest UI (`godot/rust/src/quests.rs`) consumes server-authoritative sharing and timers. [Runtime wiring and evidence](../wiki/systems/quest-share-resets.md).

## What it must do

- [x] Share the selected active quest with party members; server-issued offers show Retail QUEST_ACCEPT Yes/No popup. Send offer ID on accept/decline, never a client-authored grant.
- [x] Render supported sender and recipient rejection notices from local GlobalStrings ERR_QUEST_PUSH_* English text.
- [x] Both Modern and Forever use identical actions, popup and tracker behavior; Forever changes art only.
- [x] Show updating m:ss countdown using the sampled server clock, including an injected realm clock. Authoritative expiry replaces objectives with failed text.
- [x] Preserve server-authoritative daily/weekly availability, including re-availability after reset; do not infer regional reset times.

## How it works

- [Quest sharing/reset runtime](../wiki/systems/quest-share-resets.md)
- [Server contract](../../../game-server/docs/specs/quest-share-resets.md)

## Implementation inventory

- `godot/rust/src/{account,quests,objective_tracker}.rs` — routing, popup, receipt clock and native tracker.
- `godot/network/src/lib.rs` — share offer/result relays (mandatory, not just protocol registration).
- `godot/ui-model/src/game/{quest_actions,quest_runtime}.rs` — actions and Retail notices.
- `godot/ui-model/src/ui/screens/{quest_log_frame_component,objective_tracker_component}.rs` — Share button, countdown/failure rendering.

## Tests asserting this spec

- `godot/ui-model/tests/quest_flow.rs`: selected Share action, popup, both-skin tracker, sender/recipient strings and injected-clock countdown.
- Server tests and private UDP5517 live proof described in the [runtime page](../wiki/systems/quest-share-resets.md).

## Known gaps (current cycle)

No remaining requested behavior gap. Broader existing quest eligibility/content limitations are not expanded here.

## Out of scope

Scripted escort execution, world quests, quest pools, reputation/expansion/covenant gate implementation, artwork recovery and realm deployment. Gate protocol → server → engine; no mixed-version compatibility.
