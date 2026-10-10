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
- [ ] Import/export: `Blizzard_ClassTalentImportExport.lua:172,200` calls `C_Traits.GetLoadoutSerializationVersion`; local generated API documentation declares only a number return. No verified numeric version exists in the supplied Lua/API contract. Do not infer it from Internet example strings or select a value by popularity.
- [ ] Starter Build: authentic `TraitTreeLoadout`/entries exist; availability, activation and automatic next-purchase/deviation semantics are C APIs not represented by current server rules. No fake named Starter row.
- [ ] PvP talents/Warmode: no authoritative slot eligibility/selection/activation or realm Warmode state/change contract. No fabricated slots or booleans.

## Out of scope
Per-loadout action bar policies are not implemented by this trait-only task; existing server bar rewrite/pruning remains. No schema/world-data writes, master merge or default-realm mutation.
