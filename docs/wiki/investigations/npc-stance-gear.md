# NPC stance, weapons and armor (The Stockade, 2026-09-27)

Symptom: Stockade Guards had no weapons, no combat stance and plain clothing; Petty Criminals stood in a cluster instead of lying on the floor (Retail: guards Ready1H with sword and shield, criminals asleep).

## Causes
1. The protocol had no stand/sheath/emote state: TDB `creature_addon`/`creature_template_addon` rows (guard 375782 emote 333, criminal template 46382 StandState 3) were imported but never applied. shared-protocol `UnitPose`, game-server `unit_pose`.
2. The client never rendered creature virtual items (`EquipmentAppearance` from `creature_equip_template`) nor the display's `NPCModelItemSlotDisplayInfo` armor (guard display 2989 → Extra 1274: gloves 9449, boots 6229, tabard 6255 switch geosets 4/5/12). `src/game/networking/npc_gear.rs`, `src/game/creatures/npc_gear_data.rs`.
3. The client's replication mirror (`network_runtime/replication.rs`) copied a fixed component list; `UnitPose` was missing, so live NPCs never received their pose even though the systems worked in tests.
4. Sit (97) and Sleep (100) of HumanMale HD live in external `.anim` files; the loader read skeleton bytes at the `.anim` offsets and the body collapsed, leaving floating item models. See [[m2-format]].
5. Server: an engaging creature kept its sleep pose (TrinityCore `Unit::Attack` stands it up; `HomeMovementGenerator::DoFinalize` → `LoadCreaturesAddon` restores it).

## Not a bug
Petty Criminal positions equal TDB exactly (18 spawns, `wander_distance` 0, `MovementType` 0). The cluster in the report was criminals that had aggroed and chased the player, drawn standing.

## Evidence
`data/diagnostics/npcaddon-20260927/`: `guards-yaw330.webp` (Ready1H guards with sword and shield, ReadyRifle riflemen), `injured3-yaw90.webp` (sleeping injured guards, sitting injured riflemen), `criminals-yaw180.webp` / `criminals-yaw270.webp` (criminals asleep/sitting), `dump-tree-equip.txt` (12 swords + 12 shields, helms, rifles).

## Gaps
- Stand state SitChair (2) has no established Retail animation (error logged; 29 spawns world-wide).
- Godot client (2026-09-28): renders pose, virtual items and display armor through the shared engine-free `NpcGearData` (see [[godot-conversion]] and [npc-appearance](../../specs/npc-appearance.md)); external `.anim` files load (95b478f6). Live proof `godot/tests/npc_pose_gear.gd`, close-ups `data/diagnostics/npcposes/npc-pose-gear-{guards,criminals}-yaw{0,90,180,270}.png` (Ready1H guards with sword, shield, tabard, gloves, boots; riflemen with rifles; criminals asleep/sitting), matching the Bevy shots above. A live sheath change is not yet exercised.
- HumanFemale HD `.anim` 1000800/1000774/1000773 did not fit the AFSB layout with the pre-2026-09-30 `1000764.skel`; with the re-extracted skeleton, 1000800's tracks all fit (see [m2-format](../formats/m2-format.md)).
