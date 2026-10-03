# Raid Target Icons

Retail raid target icons (Star, Circle, Diamond, Triangle, Moon, Square, Cross, Skull): set on a unit from the TargetFrame menu or key bindings, shown on its nameplate and the TargetFrame, shared with the group. Code: `godot/rust/src/raid_targets.rs`, `godot/rust/src/nameplates.rs`, `godot/rust/src/unit_menu.rs`, `src/ui/screens/inworld_unit_frames_component.rs` (`raid_target_icon`), `src/input_bindings_data.rs`; server `crates/server/src/group/registry.rs` (`set_raid_target`); protocol `SetRaidTarget` / `RaidTargetIcons` (shared-protocol `group_messages.rs`).

References:
- `Blizzard_UnitFrame/Mainline/TargetFrame.lua:672-694` (`UpdateRaidTargetIcon`, `SetRaidTargetIconTexture`, `SetRaidTargetIcon`), `TargetFrame.xml:275-279`
- `Blizzard_NamePlates.xml:185-193`, `Blizzard_NamePlateUnitFrame.lua:809-817`, `Blizzard_NamePlateRaidTarget.lua`
- `Blizzard_UnitPopupShared/UnitPopupSharedButtonMixins.lua:2458-2640`, `UnitPopupSharedMenus.lua:323-333` (TARGET menu)
- `Bindings_Standard.xml:1573-1599`; GlobalStrings `RAID_TARGET_1..8`, `RAID_TARGET_NONE`, `BINDING_NAME_RAIDTARGET*`
- TrinityCore a352b1fa `Group::SetTargetIcon` / `SendTargetIconList` (Group.cpp:775-806), `Group::AddMember` (Group.cpp:474-478), `HandleUpdateRaidTargetOpcode` (GroupHandler.cpp:433-448)

## What it must do

- [x] Server keeps 8 icon slots per group (`m_targetIcons`). `SetRaidTarget { target, icon }`: icon 1-8 moves that icon onto the unit and clears the unit's other icon; 0 clears the unit's icon; above 8 is ignored.
- [x] In a raid only the leader marks. TrinityCore also allows assistants; assistants are not modeled.
- [x] Outside a group a player marks for themselves only (Retail; TrinityCore ignores ungrouped requests). Solo icons are dropped when the player joins a group or goes offline.
- [x] A party's icons reset when a member joins (`Group::AddMember`); a raid's stay.
- [x] `RaidTargetIcons` (the full 8 slots) goes to every online member after each change and with every roster (join, relog); a member leaving or a disbanded group gets empty icons.
- [x] `SetRaidTargetIcon(unit, n)` toggles on the client: the unit's own icon sends 0.
- [x] TargetFrame menu (any target, NPC or player): Skull, Cross, Square, Moon, Triangle, Diamond, Circle, Star, None. Retail nests them in a "Target Marker Icon" submenu with the icon texture per row; here they are flat text rows.
- [x] Bindings `RAIDTARGET1..8` ("Assign Star to Target" ...) toggle the icon on the target; `RAIDTARGETNONE` ("Clear Target Marker Icon") clears it. All unbound by default, listed in the Targeting section (Retail: "Target Markers" header).
- [x] Art: `UI-RaidTargetingIcons` (FDID 137009), `SetSpriteSheetCell(index, 4, 4)` cells left to right, top to bottom.
- [x] Nameplate `RaidTargetFrame` 22×22: RIGHT on the health bars' LEFT; on a name-only plate its BOTTOM 10 px above the name's TOP. Shown on name-only plates too.
- [x] TargetFrame `RaidTargetIcon` 26×26 centred on the portrait's TOP (portrait TOPRIGHT -26,-19 of the 232×100 frame), mapped into the portrait-off art like the rare star; hidden without an icon.
- [ ] Hostile player targets are not rejected (TrinityCore refuses marking a player hostile to the marker).
- [ ] Party/raid frames, chat `{skull}` icons and the combat log do not show icons. World markers (ground flares) are out of scope.

## Proof

- shared-protocol `protocol_group_tests.rs`: registration, `icon_of`.
- game-server `group/tests.rs`: solo player marks an NPC; group mark reaches both members, Skull moves to another unit, Cross takes Skull off the unit, 0 clears; raid non-leader ignored; party join clears and leaver gets empty icons.
- `godot/ui-model/tests/target_raid_icon.rs` (texcoords, TargetFrame placement), `nameplates.rs` / `raid_targets.rs` unit tests (plate placement, toggle), `godot/core/tests/input_bindings_data.rs`.
- Live: `godot/tests/raidicons_live.gd` against a private server (UDP 5174, fresh redb): Tab-targeted Blackrock Worg, Skull from the TargetFrame menu shows on its nameplate and the TargetFrame, Skull again clears it. `data/diagnostics/raidicons-2026-10-02/`.
