# Built-in Mythic Mentor Keys panel

Native independent Keys window, modelled on installed `BossHelper/UI/GroupFinderPanel.lua` and `KeystoneWidget.lua`. [Runtime and proof](../wiki/systems/mythic-mentor.md); [server authority](../../../game-server/docs/specs/mythic-mentor.md).

## What it must do

- [x] Launcher **Mythic Mentor** opens the panel; close/Escape closes it. Physical native fixture asserts these controls in both skins.
- [x] Reproduce list geometry:176-wide panel,22-unit header,8-unit padding,160×36 rows,4-unit gaps,26-unit dungeon icons, +level, class-coloured player name and leader crown. Authored dungeon time limit appears beside player name. Both-skin model tests and live1280×720 captures assert observable output.
- [x] Replace server rows completely: self first, then party, including “No key”; solo remains; raid/empty hides. Bag/roster/leader updates must not retain old rows. Loading retains the initial snapshot; logout clears it.
- [x] Title drag clamps to screen and saves per-character placement. Native drag/close/reopen proof records the saved position in both skins.
- [x] Use local-CASC Retail art only; Forever changes the same chrome's art, not elements or behaviour. Dungeon icons join local MapChallengeMode.MapID to JournalInstance.MapID/ButtonSmallFileDataID: eight installed-addon maps plus Azure401; square authored LFG art, not stretched panoramas. Leader132062; no-key/unknown texture134400 per addon.
- [ ] Current-season hosted-dungeon summaries use imported authored names/timers and real icon identities; no inferred seasonal/acquisition policy.
- [ ] Dungeon-icon teleport buttons match installed `MythicTeleport.lua`, including 50% black hover only for mapped teleports, known/unlearned spell hover and server-fed category1407 cooldown countdown without a swipe. Spell tooltip remains above the dialog canvas, including when screen clamping overlaps the panel. Existing PlayerSpells/action-button countdown formatting is shared, with Retail TimeUtil's hour abbreviation for8h; casts use normal server authority. Azure401 has no addon teleport mapping. Missing destination rows remain explicit gaps, never inferred.

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

- [ ] Existing server qualification is Azure Vault401; expand panel summaries for current-season dungeons the realm actually hosts from authentic catalog data.
- [ ] Qualify native teleport hover/countdown against real server updates. Eight mapped teleports have no local TDB target-position rows; record that destination gap rather than fabricate travel. All eight DB2 category recoveries are8h/1407; full playable Mythic+ runs remain Azure401-only.

## Out of scope

Addon messaging and fabricated IDs/assets prohibited. Group Finder service excluded by lead approval; addon settings/start-page cards are not this list panel. Dungeon acquisition/season policy is not defined by this panel; authored catalog summaries and supported teleport/cooldown parity are in scope.
