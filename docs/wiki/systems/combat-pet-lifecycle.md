# Combat pet lifecycle UI

Verified: 2026-10-11, protocol203acd3/serverf771781/client16fdd7f18 on private UDP54134. Requirements: [pet lifecycle](../../specs/pet-lifecycle.md). Retail Mainline controls/layout apply to both art skins.

## Stable host

`StableSession` accepts server contents only while its matching NPC interaction is open. `GameClient::stable_npc_message` opens it from the actual StableMaster role; queued ordered requests go through `Account` and the network bridge. Closing/leaving world resets pending state, screens, portraits and detached model work; delayed replies cannot reopen the window.

The installed Blizzard_StableUI supplies1040×638 layout, stabled rows/search/scroll, selected pet/model panel and five Call Pet positions plus the Beast Mastery secondary position. The server supplies205 positions, actual occupied records, unlocked calls and imported family/spec metadata. Pending operations disable mutations. Native controls select, move/swap by drag, Stable/Make Active, rename, choose spec, and confirm permanent release. No skin-specific extra controls or fabricated records.

Portraits use the existing circular portrait renderer; the selected model uses the existing detached creature scene. The stable host registers both Portrait and Plain metal styles: the latter is required for the rename dialog. Its absence reproduced `Unconverted native frame decoration: StableRenameNineSlice`;16fdd7f18 fixes that host registration, not the projection's explicit error.

## Native proof

| Boundary | Proof |
| --- | --- |
| Real Erma hello/gossip→authoritative stable | Modern and Forever native mouse runs; menu9821/option0 |
| Store→rename→spec→active | Slot0→6→0, Ash/Cunning79 and Coal/Ferocity74; bar clears when stabled |
| Call/dismiss/current and dismissed reconnect | Both skins, own wolf model/frame/bar; no cast errors |
| Wound/Mend/dead persistence/Revive | Real damage3000 and lethal999999; rendered Mend fraction increases; confirmed native Log Out keeps death; Revive and second confirmed logout keep full health, both skins |
| Six authored demons summon/menu-dismiss | Existing frame/bar messages in both skins; server reopened-redb tests cover each demon's wounded/current/dismissed state |
| Demon current/dismissed native logout | Felguard restored after Log Out, then command3 dismiss survives second Log Out, both skins |

Final logout fixtures wait for server-confirmed CharacterSelect before rebuilding the client connection. Earlier abrupt GameClient disposal permitted overlapping netcode connections and intermittent post-reconnect casts without completion; those runs are retained as diagnostic failures, not successful logout proof.

## Evidence limits

Imp renders. Voidwalker1132/Succubus4162 are outside imported appearance coverage; Felhunter850/Felguard14255 lack verified model receipts. Their native bars prove lifecycle state, not body rendering. Some player/nearby NPC receipts and Mend owner-attached visuals are also missing; no errors were suppressed or substitute assets invented. Feed's local spell is present, but Retail food-classification provenance remains unresolved; Incubus's authored creature184600 is absent. [Server lifecycle](../../../../game-server/docs/wiki/systems/combat-pet-lifecycle.md) owns storage, migration, spell and content details.

Only personally inspected genuine1920×1080 PNGs are published under `/syncthing/AgentShared/2026-10-11/pet-lifecycle/`; logs/scripts live in canonical game-engine `data/diagnostics/pet-lifecycle-2026-10-11/`. No world.db writes, shared realm changes or master merge.

## Sources

- [Stable model](../../../godot/ui-collections/src/pet_stable.rs), [screen](../../../godot/ui-collections/src/ui/screens/pet_stable_component.rs), [native host](../../../godot/rust/src/pet_stable.rs), [chrome registration](../../../godot/rust/src/ui/mod.rs).
- [Account transport](../../../godot/rust/src/account.rs); protocol/server branch `pet-lifecycle` and scoped proof ledger `/home/osso/.worktrees/handoff-pet-lifecycle.md`.
- Installed Retail `Blizzard_StableUI` and `Blizzard_APIDocumentationGenerated/PetConstantsDocumentation.lua`; pinned TrinityCorea352b1fa pet/spell references retained in the diagnostic `reference/` directory.

## See Also

- [[native-hud-edit-mode]] — shared HUD layouts and art-skin selection.
- [[player-cast-feedback]] — normal cast intent and feedback paths.
