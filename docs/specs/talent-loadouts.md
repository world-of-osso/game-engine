# Retail talent loadouts

Named trait configurations per specialization in the Talents footer. Mainline defines behaviour and elements; Forever only skins them. [Implementation](../wiki/systems/talents.md#named-loadouts--2026-10-10).

## What it must do
- [ ] Create from current staged allocation; persist ID/name/purchases and selected ID in server redb; migrate each existing single allocation to that spec's unnamed default exactly once.
- [ ] Rename/delete/switch only owner configurations of the active spec; reject invalid names, wrong spec/unknown ID and invalid allocations without mutation.
- [ ] Apply updates selected loadout; switches update effective talents/spells and relog restores selection/configs.
- [ ] Both skins show server-named dropdown rows, selected check/caption, row gear, New/rename name dialogs (30 letters), confirmed deletion and pending-edit discard confirmation. New selection waits for acknowledgement.
- [x] Import/export use Retail version2 loadout strings. Copy to Clipboard exports the current committed allocation with a zero tree hash; disable it while pending changes or spendable class/spec/selected-hero points exist (installed ClassTalentsFrame.lua560–565). Disabled tooltip is GlobalStrings49827; successful callback adds GlobalStrings49523 system chat feedback. Import accepts a code (1000 letters) and name (30 letters) in the Retail460×275 dialog, validates before sending the existing authoritative Create request, and waits for server snapshots before selecting/applying. Reject malformed/truncated codes, wrong version/spec (including other classes), invalid choices/ranks/allocations with installed Retail error text. Both skins share controls/behaviour.

- [x] Starter Build is a server-advertised, noneditable built-in selection, not a named user config. Selecting it waits for authoritative allocation/selection snapshots. The server builds affordable ranks from installed DB2 rows, persists selection, and automatically spends newly earned points. Accepted manual edits leave guided mode; node/choice deviation prompts use Retail's Continue/Cancel confirmation. Both skins share behavior.

## How it works
- [Talent system](../wiki/systems/talents.md#named-loadouts--2026-10-10)
- [Server persistence](../../../game-server/docs/wiki/systems/talents.md#persistence-and-messages)

## Implementation inventory
- `godot/core/src/talent_data/loadout_string.rs`: version2 bitstream over the complete class `TraitTree`, grants/purchases, ordered choices/tiered entries and rule validation.
- `godot/ui-model/src/talent_loadouts.rs`: editor request/dialog lifecycle.
- `godot/ui-model/src/ui/screens/spellbook_frame_component/talents_loadouts.rs`: Mainline dropdown rows and dialogs.
- `godot/network/src/lib.rs`: ordered talent relay includes loadout lists.
- `godot/rust/src/account.rs`, `spells/spellbook.rs`: owner dispatch, requests and native input.

## Tests asserting this spec
- `godot/core/tests/talent_loadout_string.rs`: real Mage and Warrior data, independently executed Blizzard-Lua strings, zero/nonzero hashes, rank/choice/prerequisite/spec/version/alphabet/truncation rejection.
- `godot/tests/talent_starter_live.gd`: private level10/11/80 activation, automatic allocation, complete relog projections and confirmed hero deviation.
- `godot/tests/talent_import_export_live.gd`: private native Create/ACK, clipboard_set argument, Retail chat feedback, exact-string reimport and complete rank/entry projection after relog; strict1920×1080 capture assertion.
- `godot/ui-model/tests/talent_loadouts.rs`, `talent_footer.rs`: actions and both-skin projection.
- `godot/network/src/wire_tests.rs`: real UDP lifecycle ordering.
- Server `class_progression_loadout_tests.rs`, `talent_loadout_persistence_tests.rs`: validation, effective spells, CRUD/relog and migration.

## Retail serialization source
Serialization version2 is corroborated by two lead-approved installed Retail addons: `/syncthing/World of Warcraft/_retail_/Interface/AddOns/TalentLoadoutManager/core/ImportExportV2.lua:5` (SHA256 `722bc71b10336d6246940b1f0a5ad0391f1926a28fee0b78b07c9b46354964df`) and `Simulationcraft/core.lua:63` under the same AddOns root (SHA256 `07cd3b7831bcbd96dfaaae9a470b8e46e20b342b9955bbe028309ea6ca7a1d1b`). Blizzard's installed `Blizzard_ClassTalentImportExport.lua:8–45,69–159,287–321,324–412` defines version8/spec16/hash128 bits, ascending all-class-tree node order, selected1/purchased1/partial1/optional ranks6/choice1/optional index2, tiered-entry distribution. Installed `Blizzard_SharedXMLBase/ExportUtil.lua:6–69,120–152` defines `A–Z a–z 0–9 + /`, LSB-first six-bit symbols without `=` padding.
## Import/export proof — 2026-10-15
Core5/5, ui-model9/9 and native extension build exit0 at20ab17bc8; exact command/revision receipts in `data/diagnostics/talent-import-export-2026-10-15/*-current-receipt.json`. Private UDP5881/server50fcd1c5/shared7313ec4 proves both skins' Create/ACK, native clipboard_set arguments, exact Retail feedback, same-string reimport and complete rank/entry equality after relog. Subsequent bounded native passes cover current modal-only errors and RomanFriz10/white code text (`InputScrollFrameTemplate`207 → `FontStyles.xml`92 → `Fonts.xml`39). Twelve inspected, hash-checked native1920×1080 PNGs are delivered under `/syncthing/AgentShared/2026-10-15/talent-import-export/`; logs/JSON stay in diagnostics. No OS paste-delivery claim.

## Starter proof — 2026-10-15
Server Starter5 scoped behavioral/data passes cover activation, online level-up, named isolation, rejected edits, spec switching, login and installed-class legality; Arms apex rows defer until authored levels. UI12, real UDP relay2 and protocol roundtrip1 pass; the later focused both-skin caption test proves Retail blue plus reset to normal after a named switch. Serverc5704b30 check/build and engine2eebb1ea4 native build pass without compiler warnings. Exact scoped revisions/commands and supersession notes live in `data/diagnostics/talent-import-export-2026-10-15/starter-*-receipt.json` and the proof ledger.

`talent_starter_live.gd` passes on real private UDP5881 for both skins: activate at10, automatically allocate at11, relog with complete rank/entry projection equality, automatically rebuild at80, cancel then confirm a hero deviation, Apply and relog with identical full allocation outside Starter mode. Native tooltip text/yellow color is asserted. Twelve inspected native1920×1080 PNGs: `/syncthing/AgentShared/2026-10-15/talent-import-export/starter/`; hashes/dimensions/inspection receipt `starter-delivered-pngs.json`. Both clients and private server stopped; no default-realm operation.

## Known gaps (current cycle)
- [ ] Lead-owned coupled Starter integration/landing gate. Branch proof does not certify unrelated talent combat behavior.
- [ ] OS clipboard handoff under headless Cage (no seat serial): clipboard_set arguments are observed through native `account_state.talent_export_clipboard`; native typed Import and authoritative relog prove the string path, not OS paste delivery.
- [ ] Nonzero tree-hash comparison (`C_Traits.GetTreeHash` algorithm) unavailable locally. Export zero-fills as Blizzard Lua16–17 explicitly allows. Per lead approval, import consumes nonzero hashes without comparison, then validates every allocation against installed nodes/choices/maxima/spec/level/currency/prerequisites. This cannot detect structurally valid but semantically changed historical nodes; no hash equality claim.
- [x] PvP tray/War Mode are supplied by the existing owner-backed [PvP progression](pvp-progression.md) implementation. [Talent rendering/live proof](../wiki/systems/talents.md#third-pass-rendering-and-live-hero-selection--verified2026-10-11) covers their footer integration; PvP policy/runtime acceptance stays in its own contract.

## Out of scope
Per-loadout action bar policies are not implemented by this trait-only task; existing server bar rewrite/pruning remains. No schema/world-data writes, master merge or default-realm mutation.
