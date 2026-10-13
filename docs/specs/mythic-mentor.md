# Built-in Mythic Mentor Keys panel

Native independent Keys window, modelled on installed `BossHelper/UI/GroupFinderPanel.lua` and `KeystoneWidget.lua`. [Runtime and proof](../wiki/systems/mythic-mentor.md); [server authority](../../../game-server/docs/specs/mythic-mentor.md).

## What it must do

- [x] Launcher **Mythic Mentor** opens the panel; close/Escape closes it. Physical native fixture asserts these controls in both skins.
- [x] Reproduce list geometry:176-wide panel,22-unit header,8-unit padding,160×36 rows,4-unit gaps,26-unit dungeon icons, +level, class-coloured player name and leader crown. Authored dungeon time limit appears beside player name. Both-skin model tests and live1280×720 captures assert observable output.
- [x] Replace server rows completely: self first, then party, including “No key”; solo remains; raid/empty hides. Bag/roster/leader updates must not retain old rows. Loading retains the initial snapshot; logout clears it.
- [x] Title drag clamps to screen and saves per-character placement. Native drag/close/reopen proof records the saved position in both skins.
- [x] Use local-CASC Retail art only; Forever changes the same chrome's art, not elements or behaviour. Dungeon icons join local MapChallengeMode.MapID to JournalInstance.MapID/ButtonSmallFileDataID: eight installed-addon maps plus Azure401; square authored LFG art, not stretched panoramas. Leader132062; no-key/unknown texture134400 per addon.
- [x] Installed addon's eight current-reference dungeon summaries, plus Azure401, use imported authored names/timers and square Journal/LFG icons. Summary publication is independent of full run qualification; no realm season/acquisition policy is inferred.
- [x] Dungeon-icon teleport mappings/content/countdown match installed `MythicTeleport.lua`:50% black hover only for mapped teleports, known/unlearned spell text and server-fed category1407 countdown without swipe. Both-skin actual click denial and real shared8h cast/update proof back this contract; final foreground-active-cooldown pixels are bounded separately below. Spell tooltip remains above the dialog canvas, including when screen clamping overlaps the panel. Existing PlayerSpells/action-button countdown formatting is shared, with Retail TimeUtil's hour abbreviation for8h; casts use normal server authority. Azure401 has no addon teleport mapping. Missing destination rows remain explicit gaps, never inferred.

## How it works

- [Native window, lifetime, art and final proof](../wiki/systems/mythic-mentor.md).
- [Server summaries and catalog scope](../../../game-server/docs/specs/mythic-mentor.md).

## Implementation inventory

- `godot/ui-model/src/mythic_mentor.rs` — list screen and addon teleport mapping.
- `godot/rust/src/mythic_mentor.rs`, `ui/mentor_host.rs` — window/drag/layout/host.
- `godot/rust/src/challenge.rs` — lifetime and native UI visitation.
- `godot/network/src/lib.rs`, `godot/rust/src/account.rs` — authoritative snapshot transport/reducer.
- `godot/ui-model/src/launcher.rs`, `godot/rust/src/launcher.rs`, `window_stack.rs` — open/close integration.

## Tests asserting this spec

- `godot/ui-model/src/mythic_mentor.rs` — both-skin labels, level replacement, no-key, crown, pruning and hide.
- `godot/rust/src/account_mythic_mentor_tests.rs` — loading retention/transport-stop clearing.
- `godot/tests/mythic_mentor_live.gd`, `godot/network/examples/native_mythic_mentor_peer.rs` — real private5751 loading/party/control/no-key/bag-grant/leadership/raid/prune proof in both skins; atomic observable row/placement records, no preview injection.

## Known gaps (current cycle)

- [ ] Final post-canvas-fix active-cooldown tooltip pixels in both skins: source install auto-patched to12.1.5.70340 on2026-10-13; pinned builddcfc90fffd79ba00406ae46f5f657592 is no longer in local `.build.info`. Both-skin real8h proof predates only the canvas correction; post-fix foreground-unlearned pixels pass both skins. No active post-fix proof is claimed.
- [ ] Eight mapped teleports have no local TDB target-position rows; known casts are denied without movement/cooldown spending. No destinations inferred. All eight DB2 category recoveries are8h/1407; full playable Mythic+ runs remain Azure401-only. Realm active season remains unset.

## Out of scope

Addon messaging and fabricated IDs/assets prohibited. Group Finder service excluded by lead approval; addon settings/start-page cards are not this list panel. Dungeon acquisition/season policy is not defined by this panel; authored catalog summaries and supported teleport/cooldown parity are in scope.
