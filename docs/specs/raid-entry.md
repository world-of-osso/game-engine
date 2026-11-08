# Raid entry and saved-instance UI
Retail Mainline raid entry uses the [server raid contract](../../../game-server/docs/specs/raid-entry.md) and existing InstanceInfo/SetSavedInstanceExtend transport. Both skins share the same element inventory and behavior.

## What it must do
- [x] Player portrait right-click opens the Retail difficulty submenus. Raid Difficulty has Normal/Heroic/Mythic, then Legacy Raid Size10/25; primary Normal/Heroic maps the legacy size to normal/heroic as Retail UnitPopup.lua SetRaidDifficulties does. Mythic disables legacy size.
- [x] Difficulty changes are disabled inside instances and for non-leader group members; server remains authoritative.
- [x] Raid Info shows authentic map/difficulty names, reset countdown, expired state and extension marker; selection enables Extend/Cancel Extension/Reactivate according to the existing lock and DB2 extension flag.
- [x] `/raidinfo` opens Raid Info and requests current saves. Close/Escape dismiss it. Large saved lists remain scrollable.
- [x] Both Modern and Forever skins use these same controls; Forever changes art only.

## How it works
- [Native implementation and portal ownership](../wiki/systems/raid-entry.md). Local Retail Blizzard_UnitPopup/Mainline/UnitPopup.lua and Blizzard_RaidFrame/Mainline/RaidInfoFrameSocialView.lua/XML define selection and row behavior.

## Implementation inventory
- `godot/ui-model/src/raid_info.rs`: catalog, saved-row state and rendering.
- `godot/rust/src/{raid_info,unit_menu,account,chat}.rs`: native UI and network actions.
- `godot/network/src/lib.rs`: receive raid difficulty updates.

## Tests asserting this spec
- `godot/ui-model/tests/raid_entry.rs`.
- `godot/rust/src/account_dungeonclient_tests.rs`.
- [Server actor-set source tests](../../../game-server/scripts/tests/test_raid_encounters.py).
- [Server multi-actor death-credit tests](../../../game-server/crates/server-combat/src/scripted_ai/tests/multi_boss_credit.rs).

## Known gaps (current cycle)
- [x] Two-account Modern/Forever native frame/menu/portal/reset proof recorded.
- [ ] Generic SocialUI raid-tab opener is not converted; native frame is currently reachable through /raidinfo and its show API.
- [x] Source-backed all-actor completion is tested for Four Horsemen and Theralion/Valiona using 12.1.5.70448 actor rows. The generator accepts 17 multi-actor journal groups whose full sets join uniquely.
- [x] VERIFIED (read-only runtime query): the lead-published bindings include all 59 actor rows for 17 multi-actor encounters; `raid_encounter_creature` has 555 rows. The lead reports `PRAGMA integrity_check` returned `ok`.
- [ ] 17 multi-actor journal groups have partial/ambiguous display joins and 161 have no same-map joins; they remain uncredited. Single-actor data also has 7 partial and 130 no-join groups. See the [server source-gap census](../../../game-server/docs/wiki/systems/raid-entry.md#multi-actor-source-census--2026-11-08).

## Out of scope
Raid encounter mechanics and rewards.

## Portal terrain ownership
`NewWorld` selects the destination map; `LoadTerrain` only bootstraps an empty world or refreshes tiles of the selected map. Late terrain hints from the departure map must not replace the destination. Native portal replay reproduced Loading(409) with fully loaded Azeroth terrain, no pending assets, and no failures after a back-to-back portal transfer. The stale map hint is rejected explicitly at the client protocol boundary, not substituted with another map.
