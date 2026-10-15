# Retail talent loadouts

Named trait configurations per specialization in the Talents footer. Mainline defines behaviour and elements; Forever only skins them. [Implementation](../wiki/systems/talents.md#named-loadouts--2026-10-10).

## What it must do
- [ ] Create from current staged allocation; persist ID/name/purchases and selected ID in server redb; migrate each existing single allocation to that spec's unnamed default exactly once.
- [ ] Rename/delete/switch only owner configurations of the active spec; reject invalid names, wrong spec/unknown ID and invalid allocations without mutation.
- [ ] Apply updates selected loadout; switches update effective talents/spells and relog restores selection/configs.
- [ ] Both skins show server-named dropdown rows, selected check/caption, row gear, New/rename name dialogs (30 letters), confirmed deletion and pending-edit discard confirmation. New selection waits for acknowledgement.
- [ ] Import/export use Retail loadout strings only after verifying the pinned client's serialization version. Until then their sentinels stay disabled; no custom or guessed format is emitted.

## How it works
- [Talent system](../wiki/systems/talents.md#named-loadouts--2026-10-10)
- [Server persistence](../../../game-server/docs/wiki/systems/talents.md#persistence-and-messages)

## Implementation inventory
- `godot/ui-model/src/talent_loadouts.rs`: editor request/dialog lifecycle.
- `godot/ui-model/src/ui/screens/spellbook_frame_component/talents_loadouts.rs`: Mainline dropdown rows and dialogs.
- `godot/network/src/lib.rs`: ordered talent relay includes loadout lists.
- `godot/rust/src/account.rs`, `spells/spellbook.rs`: owner dispatch, requests and native input.

## Tests asserting this spec
- `godot/ui-model/tests/talent_loadouts.rs`, `talent_footer.rs`: actions and both-skin projection.
- `godot/network/src/wire_tests.rs`: real UDP lifecycle ordering.
- Server `class_progression_loadout_tests.rs`, `talent_loadout_persistence_tests.rs`: validation, effective spells, CRUD/relog and migration.

## Known gaps (current cycle)
- [ ] Native/private UDP proof, independent gate and current-revision test results.
- [x] Serialization version2 is corroborated by two lead-approved installed Retail addons: `/syncthing/World of Warcraft/_retail_/Interface/AddOns/TalentLoadoutManager/core/ImportExportV2.lua:5` (SHA256 `722bc71b10336d6246940b1f0a5ad0391f1926a28fee0b78b07c9b46354964df`) and `Simulationcraft/core.lua:63` under the same AddOns root (SHA256 `07cd3b7831bcbd96dfaaae9a470b8e46e20b342b9955bbe028309ea6ca7a1d1b`). Blizzard's installed `Blizzard_ClassTalentImportExport.lua:8–45,69–159,287–321,324–412` defines version8/spec16/hash128 bits, ascending all-class-tree node order, selected1/purchased1/partial1/optional ranks6/choice1/optional index2, tiered-entry distribution. Installed `Blizzard_SharedXMLBase/ExportUtil.lua:6–69,120–152` defines `A–Z a–z 0–9 + /`, LSB-first six-bit symbols without `=` padding.
- [ ] Nonzero tree-hash comparison (`C_Traits.GetTreeHash` algorithm) unavailable locally. Export zero-fills as Blizzard Lua16–17 explicitly allows. Per lead approval, import consumes nonzero hashes without comparison, then validates every allocation against installed nodes/choices/maxima/spec/level/currency/prerequisites. This cannot detect structurally valid but semantically changed historical nodes; no hash equality claim.
- [ ] Starter Build: authentic `TraitTreeLoadout`/entries exist; availability, activation and automatic next-purchase/deviation semantics are C APIs not represented by current server rules. No fake named Starter row.
- [x] PvP tray/War Mode are supplied by the existing owner-backed [PvP progression](pvp-progression.md) implementation. [Talent rendering/live proof](../wiki/systems/talents.md#third-pass-rendering-and-live-hero-selection--verified2026-10-11) covers their footer integration; PvP policy/runtime acceptance stays in its own contract.

## Out of scope
Per-loadout action bar policies are not implemented by this trait-only task; existing server bar rewrite/pruning remains. No schema/world-data writes, master merge or default-realm mutation.
