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

## Known gaps (current cycle)
- [x] Two-account Modern/Forever native frame/menu/portal/reset proof recorded.
- [ ] Generic SocialUI raid-tab opener is not converted; native frame is currently reachable through /raidinfo and its show API.
- [ ] Ambiguous/multi-actor raid bosses remain explicit server data gaps; no invented credit.

## Out of scope
Raid encounter mechanics and rewards.

## Portal terrain ownership
`NewWorld` selects the destination map; `LoadTerrain` only bootstraps an empty world or refreshes tiles of the selected map. Late terrain hints from the departure map must not replace the destination. Native portal replay reproduced Loading(409) with fully loaded Azeroth terrain, no pending assets, and no failures after a back-to-back portal transfer. The stale map hint is rejected explicitly at the client protocol boundary, not substituted with another map.
