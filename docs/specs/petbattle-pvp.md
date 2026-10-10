# PvP pet battles

Retail Pet Journal Find Battle queues account-owned teams. Mainline sources: local Blizzard_Collections/Shared/Blizzard_PetCollection.{lua,xml}; Blizzard_StaticPopup_Game/Mainline/StaticPopupSpecial.xml. Existing battle rendering remains shared with [wild battles](wild-pet-battles.md).

## What it must do

- [x] Find Battle sends Join; queued/proposal states show Leave Queue; pending sends wait for authority.
- [x] Queue-ready popup appears even with journal closed; Accept/Decline carry the current proposal ID.
- [x] Both art skins use identical Retail controls and geometry (98 native battle rectangles match).
- [x] PvP rounds use the existing battle HUD, no trap eligibility, owner-oriented teams and terminal results.

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

- [x] Private two-account, both-skin queue → proposal Accept → initial selection → actual rounds → three-pet win/loss and HUD restoration.
- [x] PvP initial selection automatically shows the existing three-pet picker, permits selecting the current front pet and prevents abilities/pass until both owners choose. Mainline `Blizzard_PetBattleUI.lua:185–198` drives mandatory pre-battle selection; `GlobalStrings.csv` ID22782 supplies `Select a pet!`.
- [ ] Proposal/round timer values and PvP XP formula were not established from local Retail Lua/GlobalStrings/BattlePet DB2; remain unset, never guessed. Lua:523 reads `C_PetBattles.GetTurnTimeInfo()`, XML ready popup:54 queries `CanAcceptQueuedPVPMatch()`, Lua:264 reads current XP. None supplies the required numerical policy. Diagnostic census: `data/diagnostics/petbattle-pvp-2026-10-10/pvp2-retail-evidence.json`.
- [ ] Exact Retail matchmaking rating/level tolerance and penalty/reward policy not yet certified.

## Acceptance — 2026-10-10 (petbattle-pvp2)
Private UDP5598, fresh redb, accounts `fb_pbpvp2_a`/`fb_pbpvp2_b`, characters Pvptwoalpha30/Pvptwobeta31. Three authentic item4401 learns per account equip species39 pets. Each client uses its own runtime project, XDG config and `data/auth_token.127.0.0.1_5598`; only asset directories are shared. Saved-token restart succeeds with its own `layout_accounts.ron`. Modern is explicitly selected in each account-scoped HUD layout; both skins are asserted from `account_state().ui_skin`.

Native pointer proof accepts both proposals, chooses slots3/2, then A attacks while B passes/replaces defeated pets. Both skins deliver actual three-pet victory/defeat, A150/150 vs0/150 and B0/150 vs150/150. Two meshes and98 matching initial-battle rectangles are asserted; ordinary HUD disappears during battle and returns after Continue. 16 individually inspected960×540 PNGs in `/syncthing/AgentShared/2026-10-10/petbattle-pvp/`; B proposal captures lacked the popup and were excluded, not accepted. B proposal/Accept is retained in native snapshots/pointer ledger. Logs, full snapshots, `native-proof.json` and PNG hashes: `data/diagnostics/petbattle-pvp-2026-10-10/pvp2/`.

One driver retry fixed a real file-publication race: truncate/write let GDScript read incomplete JSON and terminate its command coroutine. Atomic rename publishes complete commands; no engine guard or auth workaround added. Initial-selection RED→GREEN,35 filtered UI tests,12 server wild/PvP tests (terminal-snapshot test corrected to read the last State) and3 wire tests cover this branch. No full workspace or independent-verifier acceptance claimed. Timer/XP policy remains the source gap above, not Retail-parity certification.

## Out of scope

- Unsupported battle-engine effects remain rejected; no invented pet content.
