# NPC stance, weapons and armor (The Stockade, 2026-09-27)

Symptom: Stockade Guards had no weapons, no combat stance and plain clothing; Petty Criminals stood in a cluster instead of lying on the floor (Retail: guards Ready1H with sword and shield, criminals asleep).

## Causes
1. The protocol had no stand/sheath/emote state: TDB `creature_addon`/`creature_template_addon` rows (guard 375782 emote 333, criminal template 46382 StandState 3) were imported but never applied. shared-protocol `UnitPose`, game-server `unit_pose`.
2. The client never rendered creature virtual items (`EquipmentAppearance` from `creature_equip_template`) nor the display's `NPCModelItemSlotDisplayInfo` armor (guard display 2989 → Extra 1274: gloves 9449, boots 6229, tabard 6255 switch geosets 4/5/12). `src/game/networking/npc_gear.rs`, `src/game/creatures/npc_gear_data.rs`.
3. The client's replication mirror (`network_runtime/replication.rs`) copied a fixed component list; `UnitPose` was missing, so live NPCs never received their pose even though the systems worked in tests.
4. Sit (97) and Sleep (100) of HumanMale HD live in external `.anim` files; the loader read skeleton bytes at the `.anim` offsets and the body collapsed, leaving floating item models. See [[m2-format]].
5. Server: an engaging creature kept its sleep pose (TrinityCore `Unit::Attack` stands it up; `HomeMovementGenerator::DoFinalize` → `LoadCreaturesAddon` restores it).

## Zaralda rigid waist binding (verified task date 2026-10-01)

Genuine display **138959** resolves to model **1100258**. Saved `native-appearance-covered` run reached authenticated InWorld but exited 1 with:

> Creature model 1100258: Waist item FDID 6378872: Bound equipment bone Bone[0] has no character joint

Attachment failure frees the creature model; subsequent generic pick failure does not establish physical occlusion. Actual waist data has one bone (key -1, flags 0, parent -1), CRC with no character-joint match, base mesh part 0 and local bounding-box range approximately [-0.638, 0.152]. This is an untransformed rigid root, not evidence that clothing should be removed or a missing skeletal joint ignored.

Commits **016b0fce** (2026-10-01 19:57:00 -0500) and **207aea4e** (20:01:57 -0500) share the positive classifier `slot_uses_bound_joints(slot, path, bones, mesh_parts)`: rigid Waist part-0/single-untransformed-root models use existing authored belt attachment **53**, and native mesh selection retains part 0. True bound **18xx** collections and missing-character-joint errors remain. No fallback, replacement appearance or clothing drop.

| Boundary | Evidence | Status |
|---|---|---|
| Genuine appearance cache | Independent `/tmp/zaralda-appearance-verify-20261001/report.md`: +1 coverage, +1 profile, +17 choices; all 113,120 old rows/schema preserved | Bounded data PASS; SQL consumer contract only |
| Authored geosets | No target rows in available original section; two encrypted records unavailable | Complete authored-geoset coverage unproven |
| Paired protocol artifacts | Depot **jc35z10fr8** and current server build share protocol **7ba6223**; earlier concrete mismatch superseded by matched authentication/InWorld observation | Not merchant/render acceptance |
| Binding classifier and native mesh selection | Supplied Depot CPU targeted evidence **z7mpr71q0v**, **27/27 PASS** for fix commits | CPU only |
| Retained binding check | Independent `/tmp/zaralda-rigid-waist-verify-20261001/report.md`: scoped fmt and retained cargo check PASS for 207aea4e; CPU 27/27 remains valid (pre-existing warnings retained) | Binding proof, not merchant proof |
| Fixed native model and physical pick | Supplied native observation: 116 meshes, inside frustum, real posed-torso ray valid | Model/pick boundary reached |
| Merchant window with approved faction 35 | Saved native-friendly exit0 / 529.10s; prior faction-0 AutoAttack is historical | **Observed scoped success; independent artifact gate116 scoped PASS** |

Sources: [saved paired build](../../../data/diagnostics/zaralda-20261001/paired-build/paired-artifacts.json), [failed native run](../../../data/diagnostics/zaralda-20261001/native-appearance-covered/result.json) and its `stdout.log`/`stderr.log`; independent appearance report above; task-supplied actual bone diagnosis and Depot CPU receipt. [Server Midnight investigation](../../../../game-server/docs/wiki/investigations/midnight-economy-content.md#native-acceptance-blocker) owns content/economy provenance, not this renderer root. Later native-window observation is recorded below; independent gate116 gives scoped PASS for the saved flow.

### Animated and nonzero-geoset waist follow-up (2026-10-06)

Live Stockade paladin **Fbpalrun** lost its entire authored visual with the same binding error. Everforged Greatbelt item **222431** resolves to **5646051**, an attachment-local belt with three non-key bones (including its own animated children) and mesh **401**. Cloth belt **4072824** has a single non-key root and mesh **101**. Neither carries body-belt **18xx** geosets; both live under `collections/`, which alone does not imply character binding.

The full-outfit regression also identified the same folder inference on Everforged helm **5645909** (one independent non-key root, mesh201) and shoulders **5646084/5646085** (independent emitter skeletons, mesh0). The classifier recognizes an untransformed non-key root and independent skeleton without the attachment slot's body geosets. Belt uses **53**, helm **11**, shoulders **5/6**, retaining local models and animation. Body-armor slots remain bound, as do collection models carrying their slot's body geosets; empty meshes and transformed/keyed roots do not qualify as attachment-local. No failed-binding fallback or bone guess is added. `character_equipment_fixtures.gd` covers the actual full Everforged paladin outfit and cloth belt with hide/show pixel comparisons, alongside existing TahoModern body-bound equipment. Pre-fix rendered run reproduces both load failures and passes TahoModern; post-fix verification is recorded in the [run2 proof ledger](../../../data/diagnostics/stockaderun-2026-10-06/run2/proof-ledger.txt).

### Native fixture boundary follow-up (evidence date 2026-10-01)

Old proxy-center harness targeted the wrong point; it did not establish model occlusion. Fixture commits **17d77fcb/d2084de7** use a real body point and **600-second cold setup**, based on observed **2134/4298 placements at 240 seconds**. Product **5-second handshake** and **200 ms AH limit** remain unchanged; fixture allowance is not a product timeout change.

Latest supplied native run obtained Buy cursor on actual Zaralda before faction-0 AutoAttack. [Server derived-friendly SSOT](../../../../game-server/docs/wiki/investigations/midnight-economy-content.md#approved-derived-friendly-reaction-evidence-date-2026-10-01) owns damage/despawn evidence, approved content-only faction policy, publication/rebuild proofs and pending independent scope gate. Do not bypass Enemy UI classification. Faction-35 native flow now exits0 in 529.10s: InWorld, actual UnitPick4294916904, Buy cursor, Zaralda title, authored244586 stock1 tooltip, backpack and close flow with two PNGs/evidence JSON. [Server observed-proof ledger](../../../../game-server/docs/wiki/investigations/midnight-economy-content.md#observed-native-success-task-evidence-2026-10-01) links artifacts and independent scoped PASS gate116. This bounded observation does not prove complete authored geosets (two encrypted records unavailable), whole-world rendering or clean pre-existing warnings. Genuine cache gate109 preserves 113,120 old rows and the +19 appearance additions; renderer source/CPU/compile checks remain independent proof.

## Not a bug
Petty Criminal positions equal TDB exactly (18 spawns, `wander_distance` 0, `MovementType` 0). The cluster in the report was criminals that had aggroed and chased the player, drawn standing.

## Evidence
`data/diagnostics/npcaddon-20260927/`: `guards-yaw330.webp` (Ready1H guards with sword and shield, ReadyRifle riflemen), `injured3-yaw90.webp` (sleeping injured guards, sitting injured riflemen), `criminals-yaw180.webp` / `criminals-yaw270.webp` (criminals asleep/sitting), `dump-tree-equip.txt` (12 swords + 12 shields, helms, rifles).

## Gaps
- Stand state SitChair (2) has no established Retail animation (error logged; 29 spawns world-wide).
- Godot client (2026-09-28): renders pose, virtual items and display armor through the shared engine-free `NpcGearData` (see [[godot-conversion]] and [npc-appearance](../../specs/npc-appearance.md)); external `.anim` files load (95b478f6). Live proof `godot/tests/npc_pose_gear.gd`, close-ups `data/diagnostics/npcposes/npc-pose-gear-{guards,criminals}-yaw{0,90,180,270}.png` (Ready1H guards with sword, shield, tabard, gloves, boots; riflemen with rifles; criminals asleep/sitting), matching the Bevy shots above. A live sheath change is not yet exercised.
- HumanFemale HD `.anim` 1000800/1000774/1000773 did not fit the AFSB layout with the pre-2026-09-30 `1000764.skel`; with the re-extracted skeleton, 1000800's tracks all fit (see [m2-format](../formats/m2-format.md)).
