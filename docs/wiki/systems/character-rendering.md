# Character Rendering

Character rendering assembles WoW M2 character models with dynamic geoset visibility and composited textures. The pipeline resolves which submeshes are shown (driven by customization choices and equipment), composites body/face/hair textures into a single atlas, and handles HD skeleton loading for modern character models.

## Replicated equipped item appearances

`equipment_appearance.rs` resolves visible entries with explicit display IDs directly. Item-only entries use `OutfitData::resolve_item_display_id` to join the existing cached item-modified-appearance and item-appearance tables. Hidden entries retain slot ownership without rendering; explicit displays take precedence over item IDs. Missing item mappings emit an error with item and slot context.

`tests/unit/equipment_item_tests.rs` compares textures, geosets, and attached models for actual starter items 25, 38, 39, 40, and 2362 against their catalog displays. This covers client resolution, not native rendering or combat stats.

### Portable outfit catalog

`src/game/equipment/outfit_catalog.rs` is the shared Bevy-free `OutfitData` implementation. `godot/core::outfit_data` exports it; `src/game/equipment/outfit_data.rs` retains the Bevy `Resource` adapter, original race-prefix world-DB lookup, and missing-helmet extraction. Both use the same `src/game/outfit_catalog_db.rs` SQL/import path and model-path decisions. Native callers pass a data root to `OutfitData::load` and use `try_resolve_outfit`, `try_resolve_display_info`, or `try_resolve_runtime_model` to distinguish absent data from cache/query failures; the original Bevy-facing convenience methods retain their return types. The local listfile indexes M2 paths once per data root, rather than rescanning for each item. Cache source-file keys use canonical paths, and first-time imports serialize with SQLite `BEGIN IMMEDIATE` plus freshness recheck: concurrent root/native imports no longer race to rebuild the same cache. Focused real-data proof resolves item 25/38/39/40/2362 displays, clothing FDIDs, and sword/shield model/skin FDIDs; it does not prove native attachment/render parity.

### Replicated player construction boundary

Player appearance waits for `ResolvedModelAssetInfo`, published after the complete M2 mesh, animation, and default equipment command sequence. `Children` first appeared when only the visual root existed; applying then recorded the full snapshot as deduplicated before body meshes existed, and later model initialization overwrote its equipment. Waiting for final model metadata fixes that ordering without periodic reconciliation or an InWorld-state delay. `OutfitData` loads lazily; its log after player spawn does not mean the resource was unavailable.

`tests/unit/player_appearance_spawn_tests.rs` registers real observers before spawning Theron during Loading, delivers five starter items before player identity, then enters InWorld. Desired sword/shield slots survive; body pixels match a fully constructed reapplication, change when clothing is removed, and remain unchanged on duplicate notification. Absolute replicated model paths previously bypassed SKID attachment loading because of a relative `data/models` prefix guard. Removing that guard makes skeleton lookup independent of path spelling; real HumanHD relative/absolute fixtures produce identical 45 attachment records and 75 lookup entries.

### Hand and shield attachment semantics

WoW attachment IDs are semantic mount points, not generic main/off-hand slots: right-palm weapon = 1, left-palm weapon = 2, shield = left wrist 0. HumanMaleHD maps those IDs to bones 206, 211, and 201 respectively. Earlier root-parent assertions for bones 201/206 proved that attachment data loaded, but did not prove correct hand semantics; they are superseded by this mapping. Commit `496a057b` resolves main-hand models through 1, ordinary off-hand models through 2, and canonical `item/objectcomponents/shield/` models through 0. The live HumanHD sword/shield fixture was RED against the old mapping and passes at `5e5b2574`. A one-time approved 59.173-second native run captured Theron after the corrected build; rear/side visual evidence supports the intended hand/wrist placement without arbitrary item-local rotations. Actor yaw changed for captures, not the camera, so filenames do not establish exact camera-facing angles.

Final bounded native capture shows Theron's shirt, pants, and boots. The rear camera does not independently distinguish the sword or shield. Theron's five physical records (GUIDs 10–14) persisted across a server restart. This does not add combat/stat support.

### Native selected-equipment integration

`ef697e34` consumes shared policy `2cd73e88` for clothing/cape sections and exact geosets, attaches authored M2 gear through native attachment points, binds collection meshes by semantic bone names, and reads shared transform configuration `9fc070ab`; `008a6326` fixes the native test-vector call. `d44f3dd4` changes `PlayerInput.elapsed_secs` to the exact prediction delta and passes 13/13 native primitives. Real server `:5000` compatibility remains UNKNOWN: the matching shared-protocol field is uncommitted, and no trial refresh/restart occurred.

At `640e9f30`, `/tmp/claude/native-attachments-rendered-green.log` exits 0 under real offscreen Vulkan. A camera and `frame_post_draw` prove actual HD/boar attachment lookup, authored rest offset, and the combined bone/model rotation, translation, and scale. `e0eba0d7` keeps the attachment-offset child below the bone-binding node. Earlier headless and `skeleton_updated` waits hung; they are not renderer-bug evidence. `/tmp/claude/godot-equipped-runtime-640e9f30.log` exits 0 with compiled Rust `e0eba0d7`: starter clothing, sword, and shield independently change selected-character pixels; scenery, isolated sky/rays, Loading/input gating, UDP movement, camera input, and clean shutdown also occur. Main inspected the output PNG: blue clothing with weapons.

This is bounded equipment/render evidence. Selection replacement, bound-collection actual-pixel proof, real-server compatibility, final native check/readability/independent verification, visual parity, and full conversion remain pending. Running trial `1500671` is a prior background artifact.

## Character Models and HD Skeletons

Legacy models (`humanmale.m2`) store 215 bones inline in the MD20 header. HD models (`humanmale_hd.m2`) store bones externally in a `.skel` file (referenced via the SKID chunk). The `.skel` file contains SKS1 (sequences + global sequences) and SKB1 (216 bones + animation tracks). `load_skel_data()` handles both paths transparently.

**HD skin index overflow**: the skin file's `indexStart` field is u16 but HD models have 147K+ triangle indices. From submesh 47 onward, `indexStart` wraps at 65536. Fix: `M2Submesh.triangle_start` is computed as a cumulative sum of previous `indexCount` values (u32), not the raw u16 field.

**Bone remapping**: vertex bone indices are global skeleton indices. The skin file's bone lookup table maps local (per-submesh) indices to global indices and must be applied via `remap_bone_indices()`.

## Character-creation choice application

Core appearance indexes and ordered additional `(option_id, choice_id)` selections must resolve into one effective set of customization choices before material/geoset application. Core selectors remain canonical for their corresponding option; additional pairs only select other options. This prevents default ears or a second index domain from competing with an authored extra choice.

The local catalog now carries direct/related material and geoset elements, but also marks choices with element effects the renderer does not yet apply. Those choices must remain visibly unsupported rather than be represented as working appearance changes. General requirement/visibility IDs are data, not an implemented unlock evaluator. Renderer integration and end-to-end proof are tracked in [[character-creation]].

## Geoset System

`mesh_part_id = (group * 100) + variant`. Variant 0 = hidden, variant 1+ = visible. Key groups:

| Group | Name | Notes |
|-------|------|-------|
| 0 | Hair/base skin | Mixed; variant 5 = hairstyle mesh |
| 4 | Gloves | Equipment |
| 5 | Boots | Equipment |
| 7 | Ears | 701=hidden, 702=visible |
| 17 | Eye glow | DK only, off by default |
| 21 | Head visibility | Driven by `ItemDisplayInfo.GeosetGroup_1` |
| 27 | Helmet variant | Driven by `ItemDisplayInfo.GeosetGroup_0`; default=2701 |
| 32 | Face (DF+) | Type-1 (body skin) texture |
| 33 | Eyes | |
| 34 | Eyebrows | |

Texture types: 0 = hardcoded TXID, 1 = body skin, 6 = face/hair atlas.

## Texture Compositing

Body skin texture (HD: 1024×512) is composited in `src/asset/char_texture.rs` (`seed_default_body_texture()`). Layers:
- Body base (type 1)
- Underwear overlay at `(256, 192)`
- Face upper + lower textures composited into the FACE_UPPER/FACE_LOWER regions

A second injection path in `m2_texture.rs` was removed — the compositor in `char_texture.rs` is now the single authoritative path.

HD face texture (FDID 1027494, 512×512): face skin + inner mouth + eyeball atlas. Scalp hair overlay (FDID 1043094) composited on top. These form the type-6 replaceable texture assigned to hair geosets.

`55c0167e` moves M2 render-batch CPU pixel algorithms into Bevy-free `src/asset/m2_texture_composite_data.rs`: tiled secondary-texture composition for existing shader IDs, plus positioned alpha overlays with optional nearest-neighbor 2× scaling and clipping. `m2_texture_composite.rs` retains cache keys and BLP/cache I/O; `1d25779c` shares overlay-blit and 2× scale helpers with BLP code. `c3ec6086` separately makes original render-batch decision data portable: `godot/core` exposes `m2::resolve_render_batches` with a caller-provided FDID resolver, preserving the original texture heuristic, and the root `Mesh` wrapper delegates to it. `7cc4dfbd` re-exports the CPU compositor to native code. `5701f466`/`5ba9d672`/`0386e091`/`fe792f20` replace native `StandardMaterial` with resolved authored-batch `ShaderMaterial`: original single/effect routing uses CPU second-texture/overlay composition where required; variants express source blend/cull/depth, UV/transparency, lighting and fog choices; opaque/mask paths omit `ALPHA`; and absent textures remain unbound with their FDIDs reported, not palette-substituted. `e86504d7`/`a6ad7d47` repair the original extraction failures. Native build, real HD asset/BLP, and animation fixtures pass (`/tmp/claude/native-m2-loader-{build,m2_assets,m2_animation}.log`); independent native fmt/check and core compositor 4/4 pass (`/tmp/claude/verify-native-m2-summary.md`). At `87bab0ff`, root library fmt/check pass (`/tmp/claude/native-m2-root-{fmt,lib}-87bab0ff.log`); the binary target remains blocked only by unrelated non-exhaustive `ChatType`. The unchanged `1458ccd8` fixture exits 0 with four corrected generated-M2/skin/BLP pixel assertions passing and its process group gone (`/tmp/claude/native-m2-shutdown-{captured,live}.log`). An earlier shutdown timeout is unexplained; no reliability fix is claimed. A preliminary verifier culling-regression finding was false; no culling fix is recorded. `771c1f5f` adds a native local-CASC acquisition helper for creature models: it caches model/SFID geometry/SKID skeleton companions, then existing batch resolution collects authored and explicit-slot texture FDIDs for cache. Reported focused proof is core 2/2 and native 3/3 after RED/GREEN development; it is not world-unit attachment, runtime/visual proof, or an independent gate. World light/material animation and replacement-texture/geoset APIs remain absent, as do native model/world appearance and visual parity.

## Helmet Geoset Hiding

Two overlapping mechanisms:

1. **HelmetGeosetVis / HelmetGeosetData**: DB2-driven per-race hide rules. For matching race rows, resets the specified character geoset group to default variant 1.
2. **ItemDisplayInfo head slot**: `GeosetGroup_0` → character group 27; `GeosetGroup_1` → character group 21. Group 27 special defaults: no helm → 2701; helm with `GeosetGroup_0 == 0` → 2702; otherwise → `2700 + GeosetGroup_0`.

Some helm models appear with `HelmetGeosetVis = 0,0` (e.g. display 1128), meaning `HelmetGeosetData` doesn't apply — only the GeosetGroup_0/1 path. Scalp hair suppression may require an additional rule path; `CharHairGeosets.Showscalp` is a candidate. See [helmet-hair-hiding-investigation-2026-03-28.md](../helmet-hair-hiding-investigation-2026-03-28.md).

`HelmetGeosetData.Field_10_0_0_46047_003` shows a binary `32`/`-1` pattern across hide groups but its meaning is unknown. See [helmet-geoset-extra-field-investigation-2026-03-28.md](../helmet-geoset-extra-field-investigation-2026-03-28.md).

## Authored NPC Appearance

Replicated humanoid NPCs load display-specific appearance rows from `cache/npc_appearance.sqlite` and queue a one-shot render request on their visual root after M2 creation. Full customization choice IDs select both direct and related materials/geosets; player UI indices are not used. The compositor supplies head/hair and eye textures. An authored baked body texture replaces the body atlas without clothing-erasing customization overlays; only an authored absence of a bake uses the composited body.

Selected customization geosets use the existing character visibility rules, including persistent group-zero body segments. Authored display geoset overrides apply last. Each affected mesh receives a cloned material before binding replacement textures, so NPCs sharing initial M2 materials cannot overwrite one another. A covered ordinary display remains on its existing path. A missing required profile or an out-of-coverage display errors with its ID rather than silently rendering defaults. On September 9, 2026, the local Northshire capture showed clothed, differing background NPCs; the bare foreground Human was exported as local player Theron, whose equipped gear is empty. This is affected-scene evidence, not pixel-perfect or full-catalog support.

### Native Godot wiring

`27cf3c13` adds lazy read-only native handles for imported profile/customization/compositor caches and prepares composed or baked body, type-6/type-19 textures, selected geosets, and authored overrides. `e222933f` (formatted by `1952d1cd`) passes the optional prepared appearance from `assets/mod.rs` through `assets/creature.rs` before M2 allocation; non-effect batch materials receive matching base-texture substitutions in `assets/material.rs`, while `assets/mod.rs` applies selected geosets then authored overrides to batch visibility. `world_models.rs` owns the preparer, calls it by display ID before `load_creature_model`, and preserves the normal path for absent profiles. It consumes imported caches only—no native importer freshness check or rebuild exists. Development RED at `0a53831c` reaches a geoset/body color sentinel failure; the reported RGBA in `/tmp/claude/native-npc-appearance-red-0a53831c.log` is not a measured rendered pixel. `85fa5d8a` then adds a missing-required-type-6 fixture: against unguarded DLL `1952d1cd`, it reaches `BAKED_READY` and `COMPOSED_READY` but exits 101 at phase 22 because the ordinary type-6 batch silently retains its original base texture (`/tmp/claude/native-npc-type6-red-85fa5d8a.md`). `6f00e74c` rejects absent type-1 or type-6 replacements for ordinary prepared-NPC batches; effect routing is unchanged. At `7526c855`, warning-free native build exits 0 and the actual 24-phase UDP fixture exits 0 (`/tmp/claude/native-npc-type6-{build,green}-7526c855.log`): display `910014` explicitly lacks type 6 for batch 0, creates no visual, and reset passes. At test commit `3996c869`, the real-UDP fixture expands to 25 phases and exits 0 against native DLL `7526c855` (`/tmp/claude/native-npc-hair-3996c869.log`): ordinary display `910016` reaches `TYPE6_HAIR_READY` with its declared target-10 type-6 `512×512` hair crop bound. It samples an actual native texture-image pixel distinct from base/body/head, not a rendered pixel. The captured dummy-material-null diagnostic occurs after `BAKED_READY` and before `COMPOSED_READY` during baked-model replacement; it is not proven shutdown-only. Root library fmt/check and core `npcassets` 4/4 evidence remain unchanged at `25d59471` (`/tmp/claude/verify-native-npc-appearance-*`). Verifier376 passes at `fcac4099`: `cargo fmt --check` and `cargo check -p game-engine-godot` exit 0 in `godot/` (`/tmp/claude/final-native-npc-appearance-{native-fmt,native-check}-fcac4099.log`); native sources remain `7526c855`/`0475ef39`. Defer only pre-existing `build_model` assembly length (63 body lines): new batch selection is extracted; cognitive 9/cyclomatic 14; no behavioral or complexity failure authorizes broader refactoring. At `32295725`, unchanged native DLL `7526c855` passes the 27-milestone real-UDP fixture (`/tmp/claude/native-npc-type19-effects-32295725.log`, exit 0). It retains the bound type-6 hair case; ordinary display `910025` binds type-19 RGBA `(185, 45, 215, 255)`; and effect mode 1 retains original base `(102, 76, 51, 255)` and second `(51, 128, 178, 255)` despite an available NPC type-19 replacement. Fixture `6470f4da` initially hid both authored value-101 geosets; `32295725` selects `(1, 1)`, GREEN with no production change. The prior read-only explorer hypothesis was wrong: skin offset 14 is count 2, not lookup 2. Dummy material-null remains unresolved. These are shader-resource image samples, not rendered-pixel proof. Independent gate at `ffa56c3d` inspects unchanged fixture scope at `32295725`, the full 27-milestone exit-0 runtime log, and passes `godot/` format plus `game-engine-network` example compilation (`/tmp/claude/verify-npc-positive-materials-32295725.md`); it does not rerun native, root, or core gates. Harness findings remain deferred, not clean readability: `run_fixture` cognitive 24/cyclomatic 45 and `FixtureProject::create` cyclomatic 22. Production/root/core gates remain unchanged.

## Target Circles

WoW renders selection circles procedurally (ground-projected ring tinted by unit reaction). The engine supports both procedural and BLP-textured styles. DXT1 textures (no alpha) use additive blending; DXT5 textures (real alpha) use alpha blend. Auto-detected via `is_fully_opaque()` after BLP decode. See [target-circle-styles-2026-03-30.md](../target-circle-styles-2026-03-30.md).

## Known Issues

- **HD face dark/transparent**: face geoset (mpid=5) contains both outer face skin and inner mouth cavity. With jaw in bind pose (wide open), dark inner-mouth triangles face the camera and z-occlude face skin. The jaw-close hack (-45° X rotation on key_bone_id=7) partially helps but is insufficient. Real jaw-close data may be in external `.anim` files (AFID chunk).
- **Eye rendering**: eye reflection geoset (5101) uses a specular highlight texture (pure white DXT5) — should be `emissive_texture` with AlphaBlend, not `base_color_texture` with Opaque.
- **Geoset defaults**: groups 7-12 variant 2 means bare skin on HD but equipment on legacy — the default geoset logic needs model-type awareness.

## Sources

- [character-generation.md](../character-generation.md) — original character pipeline, template skeletons, race scaling
- [hd-skeleton-status.md](../hd-skeleton-status.md) — HD model status, known issues, texture FDIDs
- [geosets.md](../geosets.md) — geoset groups, texture types, ItemDisplayInfo slot mappings
- [character-texture-debugging-2026-03-27.md](../character-texture-debugging-2026-03-27.md) — duplicate injection path cleanup
- `src/asset/m2_texture_composite_data.rs` at `55c0167e`, `src/asset/rgba_blit.rs` at `1d25779c`, and portable M2 batch resolution at `c3ec6086` — CPU composition, shared RGBA helpers, and callback-FDID render-batch decisions
- [helmet-geoset-extra-field-investigation-2026-03-28.md](../helmet-geoset-extra-field-investigation-2026-03-28.md) — extra DB2 field observation
- [helmet-hair-hiding-investigation-2026-03-28.md](../helmet-hair-hiding-investigation-2026-03-28.md) — helmet hair hiding mechanisms
- [target-circle-styles-2026-03-30.md](../target-circle-styles-2026-03-30.md) — selection circle styles, BLP blend mode detection
- `../../../data/diagnostics/wolf-nameplate-equipment-20260909/final-proof.md` — bounded physical-equipment persistence and render evidence
- `src/game/equipment/outfit_catalog.rs`, `src/game/outfit_catalog_db.rs`, `godot/core/src/outfit_data_tests.rs` at `e005b96a` — shared catalog boundary and concurrent importer fixture
- `src/game/equipment/equipment.rs` — runtime attachment selection at `496a057b`
- `/tmp/claude/native-attachments-rendered-green.log` — real offscreen-Vulkan attachment lookup/rest-transform GREEN at `640e9f30`
- `/tmp/claude/godot-equipped-runtime-640e9f30.log` — bounded selected-equipment GPU/runtime fixture GREEN with compiled Rust `e0eba0d7`
- `/home/osso/Repos/WMVx/src/core/game/GameConstants.h` and `/home/osso/Repos/WMVx/src/core/modeling/AttachmentCustomization.cpp` — local reference attachment semantics

## See Also

- [[rendering-pipeline]] — M2 material assembly, blend modes
- [[animation]] — bone animation, HD skeleton loading
- [[asset-pipeline]] — CASC extraction for BLP textures, DB2 chains
