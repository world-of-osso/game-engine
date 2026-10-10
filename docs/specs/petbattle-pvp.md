# PvP pet battles

Retail Pet Journal Find Battle queues account-owned teams. Mainline sources: local Blizzard_Collections/Shared/Blizzard_PetCollection.{lua,xml}; Blizzard_StaticPopup_Game/Mainline/StaticPopupSpecial.xml. Existing battle rendering remains shared with [wild battles](wild-pet-battles.md).

## What it must do

- [x] Find Battle sends Join; queued/proposal states show Leave Queue; pending sends wait for authority.
- [x] Queue-ready popup appears even with journal closed; Accept/Decline carry the current proposal ID.
- [ ] Both art skins use identical Retail controls and geometry.
- [ ] PvP rounds use the existing battle HUD, no trap eligibility, owner-oriented teams and terminal results.

## How it works

- [Battle pets](../wiki/systems/battle-pets.md).

## Implementation inventory

- `godot/ui-model/src/pet_journal.rs`: queue state, buttons and Mainline ready popup.
- `godot/rust/src/pet_journal.rs`: native inputs and popup lifetime.
- `godot/rust/src/account.rs`, `godot/network/src/lib.rs`: queue wire transport.
- `godot/rust/src/wild_pet_battle.rs`: shared battle renderer.

## Tests asserting this spec

- `godot/ui-model/tests/pet_journal.rs`: queue decisions and both-skin closed-journal ready popup.
- Server sibling `crates/server/src/pvp_pet_battles_tests.rs`: authoritative engine lifecycle.

## Known gaps (current cycle)

- [ ] Private two-account, both-skin screenshot proof: genuine Modern queue/proposal reached; initial unresolved border fixed, then corrected native startup blocked on saved-token/account-layout mismatch after bounded retries. No accepted PNGs published.
- [x] PvP initial selection automatically shows the existing three-pet picker, permits selecting the current front pet and prevents abilities/pass until both owners choose. Mainline `Blizzard_PetBattleUI.lua:185–198` drives mandatory pre-battle selection; `GlobalStrings.csv` ID22782 supplies `Select a pet!`.
- [ ] Proposal/round timer values and PvP XP formula were not established from local Retail Lua/GlobalStrings/BattlePet DB2; remain unset, never guessed. Lua:523 reads `C_PetBattles.GetTurnTimeInfo()`, XML ready popup:54 queries `CanAcceptQueuedPVPMatch()`, Lua:264 reads current XP. None supplies the required numerical policy. Diagnostic census: `data/diagnostics/petbattle-pvp-2026-10-10/pvp2-retail-evidence.json`.
- [ ] Exact Retail matchmaking rating/level tolerance and penalty/reward policy not yet certified.

## Out of scope

- Unsupported battle-engine effects remain rejected; no invented pet content.
