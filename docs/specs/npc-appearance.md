# Authored NPC rendering

> Root `src/` paths below name files deleted with the [retired Bevy client](godot-conversion.md#retired-bevy-client-user-decision-2026-10-02).

Replicated NPCs render the appearance selected by their creature display data. Runtime integration lives in `src/rendering/character/npc_appearance.rs`; [character rendering](../wiki/systems/character-rendering.md#authored-npc-appearance) describes the pipeline. Cache production has a separate [importer contract](npc-appearance-importer.md).

## What it must do

- [x] Retain full customization choice IDs and apply related materials/geosets only when their required choice is selected; unresolved choices produce an explicit error.
- [x] Resolve race/sex to ChrModel from `ChrRaceXChrModel.csv` (Kul Tiran 32, allied races). Apply choices of the displayed model; choices of the race's unaltered form (`ChrRaces.UnalteredVisualRaceID`, Worgen 22 → Human 1) are accepted without being applied.
- [x] Material target 10 declares the type-6 hair texture only where the layout composes target 10 into texture type 6. Dracthyr layout 155 uses target 10 for type 9 and has no type-6 hair.
- [x] Use an authored baked body texture without overwriting its clothing with the composited body. An authored absence of a bake uses composition; an unavailable declared bake is an error.
- [x] Bind distinct body/hair textures to individual NPCs without mutating shared materials or ordinary entities outside the affected visual subtree. Material target10 declares a separate hair texture for M2 type6; failed declared hair composition is an error, not a head-texture substitute. Without target10, type6 uses the composed head atlas.
- [x] Apply selected geosets followed by authored overrides, preserving character group-zero body rules; apply each added request once rather than reallocating materials every update.
- [ ] Waist models with only base mesh part0 and one untransformed non-key root (key -1, parent -1, flags0), including collection-path FDID6378872, use authored attachment53 and retain part0. Skeletal waist collections retain character-joint binding and group18 selection; an unmatched required joint remains an error. Other slots are unchanged. CPU policy tests do not prove native rendering.
- [ ] The real replicated-NPC spawn path must produce visibly correct clothing and distinct authored hairstyles/colors for the affected Northshire scene.
- [x] Apply the display's authored armor (`CreatureDisplayInfoExtra` → `NPCModelItemSlotDisplayInfo`, ItemSlot 0 head … 10 back) through the shared equipment policy: item geosets after the customization and display geosets (tabard 12, gloves 4, boots 5/20, …), helmet-hidden groups, and item models (helm, shoulders, cape) on the model's `Equipment`. Item textures stay in the display's bake.
- [x] Render replicated creature virtual items (`EquipmentAppearance` MainHand, OffHand, Ranged) as item models: main and off hand drawn with `SheathState::Melee` (right palm, left palm, shield on the left wrist), ranged with `SheathState::Ranged` (bow left palm, else right palm); otherwise at the item's `Item.SheatheType` position (WMVx `sheathTypeAttachmentPosition`: 1 → 27, 2 → 30, 3 → 32 main / 33 off hand, 4 → 28). A hand item without a sheath position stays in the hand; a ranged item without one is not shown.
- [x] Hold the replicated `UnitPose` while the creature does not move: stand state Sit 97, Sleep 100, SitLow/Medium/HighChair 102/103/104, Dead 6, Kneel 115, Submerged 202 (AnimationData IDs); standing, the `Emotes.AnimID` of its emote state (333 → Ready1H 26, 214 → ReadyRifle 48). A model without the pose's animation plays the first clip of its `AnimationData.Fallback` chain it has (Dead 6 → Death 1, which plays once and holds its last frame; Stand when the chain ends), as the combat stance does. Stand state SitChair (2) and an emote missing from `Emotes.csv` are errors.

- [x] Compose every selected separate DB2 texture type on its `ChrModelMaterial` canvas, preserving baked body (1) and the established hair/head crop (6). A selected layer without a canvas errors rather than retaining original base colour. `assets::appearance::replacement_tests` covers the declared type union and the historical missing-eye-canvas boundary.

## How it works

- [Authored NPC appearance](../wiki/systems/character-rendering.md#authored-npc-appearance)
- [Appearance importer](npc-appearance-importer.md)

## Implementation inventory

- `godot/rust/src/game/creatures/npc_gear_data.rs` — engine-free `NpcGearData` (Bevy and Godot): pose → animation, `Emotes.csv`, virtual-item attachment policy with `Item.csv` `SheatheType`, display → `CreatureDisplayInfo.csv` Extra → `NPCModelItemSlotDisplayInfo.csv` armor (`data/db2/12.1.0.69933`, exported from local CASC by `scripts/export_db2_csv.py`).
- `src/game/networking/npc_gear.rs` — `NpcGear`, `sync_npc_equipment` (model `Equipment` with `slot_attachments`), `sync_npc_pose_animation` (`IdleAnim`).
- `godot/rust/src/world_models.rs` (Godot) — creature visual keyed by display and virtual items; resolves display armor through `NpcAppearances::prepare` (geosets, hidden groups, item models) and virtual items to models with their sheath attachment; `place_virtual_items` moves them on a sheath change.
- `godot/rust/src/assets/creature.rs`, `assets/equipment.rs` (Godot) — attach armor and virtual item models (`attach_each_equipment`, a failed item is reported, others kept), `place_equipment` (reparent to the attachment, hidden for none).
- `godot/rust/src/world.rs` (Godot) — replicated `UnitPose` → held animation; `creature_animation_id` precedence: death, Walk/Run while moving, else the pose's animation, else Stand; crossfaded via `update_locomotion`.
- `src/rendering/character/npc_appearance.rs` — request processing, full-ID selection, compositing and isolated per-mesh application.
- `src/game/networking/npc.rs` — request creation after M2 spawning and update-system registration.
- `src/rendering/character/character_customization.rs` — shared geoset visibility and override rules.
- `src/game/creatures/npc_appearance.rs` — authored display cache reader, owned by the importer/data integration.
- `godot/rust/src/assets/appearance.rs` — native lazy read-only `npc_appearance.sqlite`/customization/compositor cache handles; prepares baked/composed body, type-6 hair/head, every other selected DB2 layer texture and selected/authored geosets.
- `godot/rust/src/assets/mod.rs` — passes an optional prepared native appearance into M2 batch construction; `assets/creature.rs` prepares it after creature asset caching and before model allocation.
- `godot/rust/src/assets/material.rs` — substitutes prepared replacements in every matching replaceable texture slot; `assets/mod.rs` applies shared selected-then-authored geoset visibility per batch.
- `godot/rust/src/world_models.rs` — owns `NpcAppearances`, prepares by display ID before `load_creature_model`, and retains ordinary displays on the no-appearance path.

## Tests asserting this spec

- `assets::appearance::replacement_tests` — every selected DB2 separate texture type, body/hair preservation, and explicit missing-canvas errors. Two regression tests RED at `fd8d2101`; four appearance tests (including mipmaps) GREEN with production `2be90ae0`.
- `native_npc_visual_fixture authored-textures` / `world_npc_authored_textures.gd` — independently compose canonical DB2/BLP textures, replicate real displays 825/1322/1285/90209/110154, compare complete mip-0 native material images for every authored batch/slot, and save captures. Authored display replication, not acceptance of those NPCs at their original world locations. Successful authored runs preserve oracle/capture files under canonical `data/diagnostics/npc-authored-textures/<pid>/` before disposable-project cleanup; per-type acceptance recorded below.

- `equipment_appearance_data::tests::{rigid_waist_base_mesh_uses_authored_attachment_53,skeletal_waist_collections_keep_binding_and_group_18,rigid_root_does_not_change_other_slots_collection_binding}`; native `assets::equipment::tests::{native_waist_base_mesh_and_skeletal_collection_keep_their_parts,skeletal_waist_missing_root_joint_remains_an_error}` — bounded waist policy and required-joint errors; later genuine Zaralda native-friendly exit0 is [observed scoped visual proof](../wiki/investigations/npc-stance-gear.md#native-fixture-boundary-follow-up-evidence-date-2026-10-01), independent artifact gate116 scoped PASS; encrypted-geoset completeness unproven.

- `godot/rust/src/game/creatures/npc_gear_data_tests.rs` — Stockade poses (guard emote 333, criminal Sleep/Sit, rifleman 214), guard sword/shield and rifleman rifle attachments per sheath state, guard display 2989 armor rows.
- `src/game/networking/npc_animation_tests.rs::stockade_guard_and_criminals_hold_their_authored_poses`, `::stockade_guard_draws_and_sheathes_sword_and_shield` — real models 2989/35069: played sequence IDs and item parent bones.
- `src/rendering/character/npc_appearance.rs::tests::npc_armor_geosets_switch_body_groups_but_not_equipment_models`; `equipment_appearance_data::tests::stockade_guard_armor_switches_glove_boot_and_tabard_geosets`.
- `src/rendering/character/npc_appearance.rs::tests` — body pixels/error semantics, full-ID related selections, two-NPC material/geoset isolation and once-only updates; real-data Kul Tiran 140376, Worgen 31054 mixed forms, Dracthyr 110154 without hair; ignored `sweep_all_spawned_profiles` (`SWEEP_CACHE`, `SWEEP_IDS`) prepares every listed profile and reports failures.
- `tests/unit/character_customization_tests.rs` — shared group-zero and exact geoset override semantics.
- Godot: `godot/rust/src/animation/npc_pose_tests.rs` (HumanMale HD guard Ready1H → Walk → Ready1H continuous crossfades; criminal Sleep/Sit leave Stand), `world_models::tests` (display 2989 armor geosets; sword/shield/rifle placements per sheath), `godot/network/src/wire_tests.rs::native_bridge_receives_unit_pose_changes`, live `godot/tests/npc_pose_gear.gd` (Stockade map 34).

## Known gaps (current cycle)

- [ ] Godot: a live sheath change (combat draw/sheathe) is covered by placement unit tests only; `npc_pose_gear.gd` observes spawn-time placement. Displays without a bake (1 of 7,825 in `npc_appearance.sqlite`) do not composite armor item textures in either client.

- [ ] Full declared-type real-spawn acceptance remains open: types 5/11/12/13 have no selected imported NPC profile; 21/22 have no bound slots in the sampled Kul Tiran base M2; 23 has no layer, 24 no canvas/selected profile, and sampled type 26 is hidden. See the per-type table below. Ordinary creature skin slots (M2 2/11, 12, 13) are a distinct path, not proven by these compositor cases.

- [ ] Parent integration must import current display data and visually validate the actual replicated Northshire NPCs; synthetic material tests are not visual acceptance. The native path reads imported caches only: it neither checks importer freshness nor rebuilds them.

- [ ] The isolated native fixture bootstrap is not a passing regression gate. `0f83d8f9` through `40fc60a4` repair missing UI/Warband CSV, cached model/terrain, player hair defaults without overriding explicit NPC negatives, private SQLite/local aliases/community CSV, and recursive skybox staging. The successive logs exit 101 for missing fixture hair, map alias, nested skybox model, then an actual logger regression: the emitted missing-required-type-6 error loses display `910014` context after all fixture milestones reach `RESET_READY` (`/tmp/claude/npc-fixture-bootstrap-runtime-{54f6bf69,36449a60,09c5f94d,40fc60a4}.log`). `ca0c9204` restores NPC server-ID/display-ID and player-name context. Its native build exits 0 with two existing unused-WMO-field warnings; runtime exits 0 across NPC, lighting, death, visibility, missing-type-6, bound-hair, type-19, effect, and reset, and observes the corrected `NPC … display 910014` error (`/tmp/claude/npc-diagnostic-{build,runtime}-ca0c9204.log`). The headless dummy-renderer `Parameter "material" is null` diagnostic remains nonfatal; this is not error-free evidence. Independent verifier audit remains pending.

- [ ] `85fa5d8a` adds a native missing-required-type-6 fixture. Against unguarded DLL `1952d1cd`, it reaches `BAKED_READY` and `COMPOSED_READY`, then exits 101 at phase 22 because the ordinary type-6 batch silently retains its original base texture (`/tmp/claude/native-npc-type6-red-85fa5d8a.md`). `6f00e74c` rejects an absent type-1 or type-6 replacement for an ordinary prepared-NPC batch; effect routing is unchanged. At `7526c855`, warning-free native build exits 0 and the actual 24-phase UDP fixture exits 0 (`/tmp/claude/native-npc-type6-{build,green}-7526c855.log`): display `910014` reports its missing type-6 texture for batch 0 without creating a visual, then reset passes. At test commit `3996c869`, the same real-UDP fixture runs 25 phases against native DLL `7526c855` and exits 0 (`/tmp/claude/native-npc-hair-3996c869.log`): ordinary display `910016` reaches `TYPE6_HAIR_READY` with its declared target-10 type-6 `512×512` hair crop bound. The fixture samples the actual native texture-image pixel and proves it differs from base/body/head; it does not sample rendered pixels. Its dummy-material-null diagnostic is captured after `BAKED_READY` and before `COMPOSED_READY` during baked-model replacement, not proven shutdown-only. Prior root library fmt/check and core `npcassets` 4/4 proof remain unchanged at `25d59471` (`/tmp/claude/verify-native-npc-appearance-*`). Verifier376 passes at `fcac4099`: `cargo fmt --check` and `cargo check -p game-engine-godot` exit 0 in `godot/` (`/tmp/claude/final-native-npc-appearance-{native-fmt,native-check}-fcac4099.log`); native sources remain `7526c855`/`0475ef39`. Defer only pre-existing `build_model` assembly length (63 body lines): new batch selection is extracted; cognitive 9/cyclomatic 14; no behavioral or complexity failure authorizes broader refactoring. At `32295725`, unchanged native DLL `7526c855` passes the 27-milestone real-UDP fixture (`/tmp/claude/native-npc-type19-effects-32295725.log`, exit 0). It retains the bound type-6 hair case; ordinary display `910025` binds type-19 RGBA `(185, 45, 215, 255)`; and effect mode 1 retains original base `(102, 76, 51, 255)` and second `(51, 128, 178, 255)` despite an available NPC type-19 replacement. Fixture `6470f4da` initially hid both authored value-101 geosets; `32295725` selects `(1, 1)`, GREEN with no production change. The prior read-only explorer hypothesis was wrong: skin offset 14 is count 2, not lookup 2. The historical dummy material-null diagnostic is covered by the later [M2 material-free ordering fix](../wiki/investigations/godot-material-null-free.md) (`eb619da4`, with reader follow-ups `c7ed0120`/`bfb73227`). These are shader-resource image samples, not rendered-pixel proof. Independent gate at `ffa56c3d` inspects unchanged fixture scope at `32295725`, the full 27-milestone exit-0 runtime log, and passes `godot/` format plus `game-engine-network` example compilation (`/tmp/claude/verify-npc-positive-materials-32295725.md`); it does not rerun native, root, or core gates. Harness findings remain deferred, not clean readability: `run_fixture` cognitive 24/cyclomatic 45 and `FixtureProject::create` cyclomatic 22. Production/root/core gates remain unchanged.

### Material-null follow-up (2026-10-05)

At `dd6c0453`, the unchanged native fixture crosses `BAKED_READY` → `COMPOSED_READY` with zero material-null errors under headless `--verbose --fixed-fps 60`. Its full run exits 101 at phase 24: ordinary type-19 expects `(185,45,215,255)` but receives the original base texture. Two real-time runs fail earlier at the death-motion assertion. These failures do not authorize changing appearance or death behavior for the material-null task. Evidence: `data/diagnostics/materialnull-2026-10-05/baseline-runtime-{sqlite,retry,fixed}.log`.

The network example now rejects `Parameter "material" is null` from either Godot output stream, including output drained after child exit. The intentional missing-type-6 diagnostic remains allowed. Full native-fixture acceptance remains blocked by the type-19 failure; zero material-null lines alone are not an exit-0 claim.

### Type-19 fixture canvas (2026-10-05)

`d2a82a32` intentionally switched NPC eyes from one replacement FDID to all selected layers composited on the `ChrModelMaterial` canvas (`assets/appearance.rs::compose_replacement_textures`, using `CharTextureData::composite_texture_type`). The synthetic cache declared `model_materials` but omitted its type-19 canvas; composition therefore returned `None`, retaining ordinary base RGBA `(102,76,51,255)` instead of `(185,45,215,255)`. Stage `(layout_id=910041, texture_type=19, width=2, height=2)`. Keep the existing ordinary and two-texture pixel assertions unchanged; this repairs fixture data, not production behavior. The historical effect-mode expectation predates the retail shader batch binding rewrite; current fixture asserts replacement type-19 in slot 0 and original slot 1.

### Texture-type acceptance (2026-10-07)

Verified: 2026-10-07. Historical phase-24 failure was fixture data, not a current type-19 binding failure: `d2a82a32` moved eyes to layer composition, but the fixture lacked its `ChrModelMaterial` canvas. `6f7a9df4` supplied `(910041,19,2,2)` before this task. Both baseline `ca32f892` and final production `2be90ae0` reach `TYPE19_READY`, `EFFECT_ISOLATED_READY`, and `RESET_READY` in the full native UDP fixture. The current two-texture assertion expects the type-19 replacement in slot 0 and the original slot 1, not the historical pre-retail-shader effect routing.

Remaining production gap was the hardcoded 19-only separate-canvas branch. `2be90ae0` makes it data-driven and explicitly errors on selected missing canvases. Synthetic CPU assertions cover the union of `ChrModelTextureLayer` and `ChrModelMaterial` types: **1,5,6,7,8,9,10,11,12,13,19,20,21,22,23,24,25,26**. Layer types omit 23; canvas types omit 24. This does not confer real-spawn acceptance on absent/unconsumed types.

`native_npc_visual_fixture authored-textures` replicates five genuine catalog template/display pairs as fresh NPC spawns, relocated to the fixture map with a stable node-name alias: Dark Strand Adept 3728/825, Kaja 3322/1322, Conservator Ilthalaine 2079/1285, Apprentice Mage 149131/90209, Krenzen 198506/110154. The independent oracle reads full-ID selections and local BLP pixels, retains the DB2 bake, and composites other layers with the shared compositor rather than the native preparation helper. Source CSVs independently confirm all five race/sex/bake mappings through `CreatureDisplayInfoExtra` and `TextureFileData`; armor rows come from `NPCModelItemSlotDisplayInfo`.

**Material PASS** means complete native mip-0 RGBA bytes and dimensions match the DB2/BLP oracle in every sampled M2 batch/slot. **Visible PASS** additionally requires at least one enabled batch on that genuine spawn. This is material-image proof, not rendered-scene pixel parity or acceptance at original world locations. FAIL below means the corresponding acceptance gate is unproven, not necessarily a client defect.

| Type | Material gate | Visible spawn gate | Concrete evidence |
|---|---|---|---|
| 1 | PASS | PASS | Dark Strand Adept 825: 65 slots, 11 visible; body bake FDID919252. Kaja 1322 bake1012307 and Krenzen 110154 bake4736911 also match. |
| 5 | FAIL | FAIL | No selected imported NPC profile; generic CPU behavior only. Wider catalog profiles require separate input/import coverage. |
| 6 | PASS | PASS | Dark Strand Adept 825: 43 slots, 4 visible; authored hair/head image. |
| 7 | PASS | PASS | Apprentice Mage 90209: 20 slots, 3 visible. |
| 8 | PASS | PASS | Kaja 1322: 16 slots, 3 visible. |
| 9 | PASS | PASS | Krenzen 110154: 26 slots, 1 visible. |
| 10 | PASS | PASS | Krenzen 110154: 1 slot, 1 visible. Ilthalaine 1285 matches 19 slots but all are hidden. |
| 11 | FAIL | FAIL | No selected imported NPC compositor profile; ordinary creature skin-slot behavior is separate. |
| 12 | FAIL | FAIL | No selected imported NPC compositor profile; ordinary creature skin-slot behavior is separate. |
| 13 | FAIL | FAIL | No selected imported NPC compositor profile; ordinary creature skin-slot behavior is separate. |
| 19 | PASS | PASS | Dark Strand Adept 825: 2 slots, 2 visible; plus ordinary/two-texture synthetic native boundary. |
| 20 | PASS | PASS | Kaja 1322: 26 slots, 1 visible. |
| 21 | FAIL | FAIL | Apprentice Mage 90209 selects it, but M2 FDID1734034 primary skin has zero bound slots of this type. No native material assertion. |
| 22 | FAIL | FAIL | Same selected profile/model: zero bound slots. No native material assertion. |
| 23 | FAIL | FAIL | Declared canvases, no texture layers/selected imported NPC profile. |
| 24 | FAIL | FAIL | Layout135/136 layers target0; no matching canvas or selected imported NPC profile. Selected missing-canvas error is CPU-tested. |
| 25 | PASS | PASS | Krenzen 110154: 1 slot, 1 visible. |
| 26 | PASS | FAIL | Krenzen 110154: all 8 slots match, but 0 visible. Do not infer visible correctness from hidden materials. |

The expected `missing NPC replacement texture type 6` line is an intentional synthetic negative, not missing real CASC data: display910014's selected materials omit target10 and its layout has no head section9. The separately authored player-default hair choice is deliberately not injected into explicit NPC selections. Its type-6 M2 batch must fail without creating a visual; the positive synthetic hair case and all three genuine type-6 profiles pass.

Evidence:

- Production `2be90ae0`; oracle skeleton/catalog/real-template correction `b3808351`; artifact-retention correction `1212db4b`. Baseline helper build `/tmp/claude/npctextypes-build-baseline.out`; type-19 native baseline `/tmp/claude/npctextypes-baseline-runtime.out`.
- RED `/tmp/claude/npctextypes-red.out`: two tests fail (absent type5; missing type19 canvas returns empty), exit101. GREEN `/tmp/claude/npctextypes-green.out`: appearance4/4, exit0. `/tmp/claude/npctextypes-core.out`: asset/catalog4/4, exit0. Final workspace format `/tmp/claude/npctextypes-fmt.out`: exit0; local locked helper build `/tmp/claude/npctextypes-build-artifacts.out` succeeds without warnings. No separate current `cargo check` gate was run; native build and focused tests compile the affected code.
- Final ordinary/type6/type19/effect/reset fixture `/tmp/claude/npctextypes-final-native.out` passes all 27 milestones. Authored bounded material runs `/tmp/claude/npctextypes-authored-{green,artifacts}.out` complete with 555 matching batch/slot images and five fresh `existing_appearance=false` spawns. Production DLL SHA256 `2c5fff42797aa9b65431f850ba0c65eca64d2bdd87f8d5afc783d38bf28b7de4`.
- Retained native captures/oracle: canonical `data/diagnostics/npc-authored-textures/593095/{captures,oracle}/`. Independent Pillow decoding of **555/555** saved PNGs matches the full RGBA oracle with zero differences (`/tmp/claude/npctextypes-capture-inspection.out`, `data/diagnostics/npctextypes-2026-10-07/capture-validation.json`). Visually inspected ten-type contact sheet `material-contact-sheet.png`, Kaja `1322-type1-Batch1-slot0.png`, Krenzen `110154-type1-Batch10-slot0.png`; exact representative paths in `inspected-capture-paths.json`.
- Retained setup failures: oracle initially parsed SKID models without skeletons; fixed at `b3808351`. GDScript bool inference fixed at `2be90ae0`. Original fixture cleanup removed first-run captures; `1212db4b` preserves proof before cleanup. These failed probes are not behavioral PASS/RED evidence. Wider raw import inputs `1264997.db2`, `3692043.db2`, `1720141.db2` are absent from current local data tree; current imported-profile coverage is not the entire CSV catalog.
- Native runs retain pre-existing shutdown warnings: 81 ObjectDB instances and dummy-renderer texture/font RIDs leaked. Zero material-null diagnostics is not clean teardown, full visible NPC parity, or all-declared-type acceptance. New test harness orchestration length remains readability debt; no clean-readability claim.

## Out of scope

- Player equipment replication and creature model redesign; authored NPC data is applied through the existing M2 and character paths.
