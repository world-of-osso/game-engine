# Native trainer frame

Native `ClassTrainerFrame` in `godot/ui-model/src/trainer*.rs` and `godot/rust/src/trainer.rs`, opened by the existing NPC gossip trainer role. [Professions contract](professions-frame.md) retains historical Retail requirements and recorded user decisions; retired-client checkboxes are not native proof.

## What it must do

- [ ] A concrete `TrainerList` opens the trainer; closing sends `CloseInteraction`, and matching `InteractionClosed` or world reset closes it.
- [ ] Available / Unavailable / Used filters project server states without modifying them; selection remains visible.
- [ ] Services display catalog names/icons, cost, level, skill rank and prerequisite abilities. Known services show Already known without cost. Learned relevant skill ranks come from `ProfessionSnapshot`.
- [ ] Train sends exactly one `TrainerBuySpell` for an available affordable selected service, waits for authority, and refreshes service state from the next list and money from replicated Gold.
- [ ] Adding a primary profession requires Accept / Cancel and a free primary slot; changing selection or closing cancels confirmation.
- [ ] `TrainerBuyFailed` displays its reason without optimistic spending. Both Modern and Forever use the shared window chrome and same decisions.

## Retail references

Under `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`:

- `Blizzard_TrainerUI/Mainline/Blizzard_TrainerUI.lua:58-89`: filter toggles and Available / Unavailable / Used checkboxes.
- Same file `:187-307`: service projection, requirement lines, known-state text, cost and enabled Train decision.
- Same file `:11-33`, `:400-406`: profession popup and purchase action.
- `Blizzard_TrainerUI/Mainline/Blizzard_TrainerUI.xml:28-105`: service icon, name, subtext, cost, selection.
- `Blizzard_TrainerUI/Mainline/Blizzard_TrainerUI.lua:156-172`: learned rank text.
- Refusals use existing protocol `TrainerFailReason::message` (NotEnoughMoney → "You don't have enough money."); Unavailable has no dedicated Retail string and is explicitly shown as "This service is unavailable."

## How it works

- [Native professions](professions-frame.md#native-godot-coverage-2026-10-07): shared catalogs and owner snapshot.
- [Live execution](../headless-live-run.md): isolated private server and rendered proof.

## Implementation inventory

- `godot/ui-model/src/trainer.rs`: list/filter/selection/request/confirmation/refusal decisions.
- `godot/ui-model/src/trainer_frame.rs`: skinned window, service list, filters, rank, money and confirmation.
- `godot/ui-model/src/professions_catalog.rs`: primary profession classification from SkillLine.
- `godot/rust/src/trainer.rs`: NPC-driven native UI, Gold and snapshot projection, TrainerChannel sends.
- `godot/rust/src/{account,merchant,professions,window_stack,lib}.rs`, `godot/network/src/lib.rs`: existing consumer/lifecycle integration.

## Tests asserting this spec

- `godot/ui-model/tests/trainer.rs`: concrete list, filters, exact once request, authoritative refresh, failure text, confirmation, disabled services and both-skin rendered registry projection.
- Native/private evidence is appended to canonical `data/diagnostics/trainerframe-2026-10-07/proof-ledger.txt`.

## Known gaps (current cycle)

- [ ] Current-head targeted tests and inspected private gossip → training → money/state captures remain required before native acceptance.
- [ ] Exact Retail 338×424 geometry, trainer-specific background/row art, portrait, rank fill, per-requirement colouring, tooltip and SkillStepButton parity remain unverified. No user-requested deviation is removed or invented.

## Out of scope

- Server/protocol changes, reputation discounts, additional trainer mechanics, merge and push are excluded by this task.
