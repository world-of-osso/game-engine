# Raid entry and saved-instance UI
Retail Mainline raid entry uses the [server raid contract](../../../game-server/docs/specs/raid-entry.md) and existing InstanceInfo/SetSavedInstanceExtend transport. Both skins share the same element inventory and behavior.

## What it must do
- [ ] Player portrait right-click opens the Retail difficulty submenus. Raid Difficulty has Normal/Heroic/Mythic, then Legacy Raid Size10/25; primary Normal/Heroic maps the legacy size to normal/heroic as Retail UnitPopup.lua SetRaidDifficulties does. Mythic disables legacy size.
- [ ] Difficulty changes are disabled inside instances and for non-leader group members; server remains authoritative.
- [ ] Raid Info shows authentic map/difficulty names, reset countdown, expired state and extension marker; selection enables Extend/Cancel Extension/Reactivate according to the existing lock and DB2 extension flag.
- [ ] `/raidinfo` opens Raid Info and requests current saves. Close/Escape dismiss it. Large saved lists remain scrollable.
- [ ] Both Modern and Forever skins use these same controls; Forever changes art only.

## How it works
- Local Retail Blizzard_UnitPopup/Mainline/UnitPopup.lua and Blizzard_RaidFrame/Mainline/RaidInfoFrameSocialView.lua/XML define selection and row behavior.

## Implementation inventory
- `godot/ui-model/src/raid_info.rs`: catalog, saved-row state and rendering.
- `godot/rust/src/{raid_info,unit_menu,account,chat}.rs`: native UI and network actions.
- `godot/network/src/lib.rs`: receive raid difficulty updates.

## Tests asserting this spec
- `godot/ui-model/tests/raid_entry.rs`.
- `godot/rust/src/account_dungeonclient_tests.rs`.

## Known gaps (current cycle)
- [ ] Both-skin private live proof pending; generic SocialUI raid-tab frame is not part of this slice.
- [ ] Ambiguous/multi-actor raid bosses remain explicit server data gaps; no invented credit.

## Out of scope
Raid encounter mechanics and rewards.
