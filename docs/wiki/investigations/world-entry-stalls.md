# World-entry stalls (Godot client)

At world entry the Godot client blocked one frame for 17-87 s in the "Account" step. Afterwards the "World objects" step ran well past its 8 ms budget and never drained its queue. The cause was synchronous file, parse and texture work on the main thread, done once for every unit and placement. That work now runs on `AssetLoader` workers, and the main thread builds nodes within per-frame budgets.

## Symptoms (base, before the fix)

- **"Account" frame.** At world entry the server replicates every unit in range at once. Each `UnitUpdated` built its visual synchronously in `WorldUnits::upsert` → `WorldModels::load_visual`. A profile of the 14.9 s frame (`GAME_PROFILE_MS=5`, load average about 15) showed 75 unit events taking 14.7 s: 73 NPC visuals took 13.3 s, of which `build_model` was 6.4 s, NPC texture composition 3.0 s and M2 parsing 2.2 s. The first NPC also loaded the NPC gear DB2 rows on the main thread (1.0-1.4 s).
- **`build_model` per humanoid NPC** (113-151 batches) took 150-350 ms. Mesh streams were assembled vertex by vertex through Godot FFI in the unoptimized `game-engine-godot` crate (about 1 ms per batch). The M2 shader variant was rebuilt from the shader source for every batch (about 0.6 ms). `WowAnimationPlayer::from_model` deep-cloned every bone track (15-145 ms).
- **"World objects" frames.** The budget was checked before each unit of work, but one unit could be:
  - a local-CASC extraction plus M2 parse and texture decode;
  - a whole WMO: all its groups' meshes and materials, 426-641 batches, 87-320 ms.

  With about 6,500 placements near Northshire, `world_objects.pending` did not reach 0 within 300 s.
- **Repeat stalls.** A unit leaving and re-entering range rebuilt its visual from scratch: 250-330 ms every ~10 s for a patrolling NPC.
- **"Terrain materials."** Every parsed tile was built in the frame it arrived: 256 chunks, each with meshes, collision and materials. That took 225-980 ms per tile.

## Fix

- **Unit visuals** (`world_models.rs`, `world.rs`, `assets/{creature,player,appearance}.rs`). `WorldModels` owns an `AssetLoader` with two `unit-visuals` workers. `upsert` requests the visual of a changed appearance, and the old visual stays until the new one arrives. The worker does everything that needs no engine calls:
  - the display query and gear/outfit rows;
  - NPC and player texture composition, kept as pixels (`AppearanceParts`);
  - extraction and parsing of the body, armor and item models;
  - BLP decode of their textures.

  The new "Unit visuals" frame step attaches arrived visuals within 8 ms (at least one per frame), then brings each up to its snapshot: sheath placement, death and animation. The gear and outfit catalogs are warmed on a worker when the client starts.
- **Shared parse and GPU resources.**
  - Parsed models are cached per `.m2` path (`creature::load_model_files`).
  - Batch meshes are cached per model and submesh.
  - M2 shader variants are cached per (blend mode, effect, two-sided); WMO shader variants per (two-sided, blended, clamp S/T).
  - M2 bone tracks are `Arc`-shared.
  - `equipment_transforms.ron` is read once.
  - Submesh vertex streams are built in `game-engine-core` (`m2::submesh_arrays`, opt-level 2) and copied into Godot arrays in bulk.
- **ADT objects** (`terrain/objects.rs`). Placements whose model or WMO is not loaded are handed to two `world-objects` workers and wait by asset. The workers extract and parse (`load_model_files`, `wmo::assets::read_wmo`) and decode textures. Within the 8 ms budget, the main thread inserts arrived textures, spawns ready placements, and builds WMOs a slice of batches at a time (`wmo::scene::WmoBuild`). `pending_count` counts waiting and in-progress placements too.
- **Terrain** (`terrain/material.rs`). Tile chunks are built within an 8 ms budget per frame, the map request's initial tiles first. A tile attaches once all its chunks are built. Mesh and collision arrays are converted in bulk.
- **Loading screen** (`loading.rs`). It also waits for the local player's model to be attached, or to have failed and been reported ("Initializing character..."). No retail source was found for what the loading screen waits on. Solarityclient's own readiness chain (not reverse-engineered) also waits for the controlled player model (`crates/runtime/src/loading/readiness.rs:5-16`). The only retail-derived condition found is that the screen waits for a transport the player stands on (`readiness.rs:71-73`). Other placements stream in after the loading screen hides, and none of them blocks a frame.

## Measurements

The fixture is `godot/tests/world_entry_frames.gd`. It logs in, enters the world and records the wall-clock time of each frame: through loading, then until `world_objects.pending` is 0, plus 120 frames. It fails when:
- a loading frame exceeds `WORLD_ENTRY_LOADING_FRAME_MS` (1000);
- a frame after the loading screen hides exceeds `WORLD_ENTRY_FRAME_MS` (100);
- the queue does not drain within `WORLD_ENTRY_SETTLE_S`.

Set `GAME_PROFILE_MS=<ms>` to print each frame step and account event slower than that as `PROFILE`.

Back-to-back A/B, 2026-09-30:
- Setup: private game-server `a6a704f`, shared-protocol `2acb8a9`, UDP 5102, fresh redb per run, level-10 Human paladin `fb_worldentry`/Fbworldent in Northshire, debug builds.
- Base is master `0bc505b6`; branch is `worldentry`.

The fixture's bounds were a 100 ms frame limit and 300 s to settle. "Settled" means `world_objects.pending` reached 0.

- **Round 1:** base `0bc505b6`, branch `d75ef727`; game-server `a6a704f`, shared-protocol `2acb8a9`.
- **Round 2:** base `c521ec6c`, branch `92098f39`; shared-protocol `88bc3fe`. Players now load ahead of creatures, and object textures upload one per budget step.

| Run | Load | Loading screen | Longest loading frame | World median / p99 | Longest world frame | Frames over 100 ms | Settled (pending at 300 s) |
|---|---|---|---|---|---|---|---|
| R1 base 1 | 14 | 31.2 s | 21.5 s | 84 / 703 ms | 5.05 s | 945 | no (5491) |
| R1 branch 1 | 19 | 4.9 s | 0.34 s | 67 / 132 ms | 0.45 s | 226 | 113 s |
| R1 base 2 | 14 | 20.7 s | 17.6 s | 43 / 156 ms | 0.62 s | 55 | 111 s |
| R1 branch 2 | 15 | 18.1 s | 0.60 s | 77 / 255 ms | 1.11 s | 360 | 91 s |
| R1 base 3 | 19 | 35.6 s | 27.6 s | 122 / 1662 ms | 7.05 s | 1282 | no (5953) |
| R1 branch 3 | 18 | 7.2 s | 0.31 s | 94 / 290 ms | 0.72 s | 678 | 152 s |
| R2 base 1 | 17 | 37.2 s | 33.5 s | 91 / 1598 ms | 4.86 s | 792 | no (6349) |
| R2 branch 1 | 22 | 5.5 s | 0.22 s | 186 / 683 ms | 1.25 s | 1269 | no (3831) |
| R2 base 2 | 44 | 61.5 s | 47.7 s | 154 / 3696 ms | 7.36 s | 892 | no (7425) |
| R2 branch 2 | 44 | 13.6 s | 0.91 s | 131 / 627 ms | 0.91 s | 1156 | no (2135) |
| R2 base 3 | 31 | 29.3 s | 24.7 s | 116 / 1736 ms | 3.48 s | 1231 | no (6556) |
| R2 branch 3 | 30 | 46.7 s | 0.80 s | 143 / 450 ms | 29.4 s | 1299 | no (1111) |
| R3 base 1 | 18 | 25.6 s | 21.3 s | 96 / 808 ms | 3.01 s | 1144 | no (4851) |
| R3 branch 1 | 13 | 4.6 s | 0.27 s | 81 / 335 ms | 0.53 s | 461 | 119 s |
| R3 base 2 | 21 | 17.3 s | 15.1 s | 47 / 155 ms | 0.66 s | 59 | 137 s |
| R3 branch 2 | 7-14 | 4.7 s | 0.20 s | 49 / 91 ms | 0.43 s | 4 | 30 s |

**Round 3** compared base `4b20e16e` with branch `e0728e69`. Both include the resolver-cache change (`237d3c27`), so both reuse `~/.cache/asset-resolver`, and neither rebuilt the CASC table. Base still blocked one frame for 15-21 s, so that stall is the synchronous unit visuals, not the CASC cache. VmHWM at the end was 3.66 and 4.09 GB for base, 2.73 and 2.80 GB for the branch.

- **Longest loading frame:** 17.6-47.7 s in base, 0.22-0.91 s on the branch.
- **Longest world frame:** 0.62-7.4 s in base. On the branch it was 0.45-1.25 s, except one 29.4 s frame in R2 branch 3.
  - During that run the machine's 27.9 GB of swap was full and load reached 65. The client reported 28.5 s of its own process time for the frame, which step is not recorded (profiling was off).
- **Settling:**
  - Base settled once in six runs.
  - The branch settled in all three R1 runs (91-152 s). In R2, at load 22-44, it did not settle in 300 s, but it had 1,111-3,831 placements left against base's 6,349-7,425.
- **Remaining over-100 ms frames:** most of them are the steady-state frame. With every object spawned, the branch's median is 67-186 ms at load 15-44, and its per-frame client time is 25-60 ms plus 25-50 ms of viewport render CPU. Base spawns less, so it draws less; its low median in R1 base 2 comes from a world that was mostly empty.

Single runs at low load (GAME_PROFILE_MS on, branch before the merge):
- Load 3.7: 3.0 s loading (longest frame 170 ms), median 42.7 ms, 18 frames over 100 ms, settled in 16.5 s.
- Load 8, after the WMO slicing: 2.8 s loading (longest 116 ms), median 42 ms, p99 81 ms, two frames over 100 ms (212 and 122 ms), settled in 21.2 s.

## Remaining gaps

- **First in-world frame** (163-326 ms of client time): HUD steps set up at once. "Minimap" takes 76-176 ms (133-602 ms at load 20-35) decoding its tiles synchronously (`minimap.rs` `load_tile`); its catalogs and the entrance bar's now load in the background ([catalog waits](#catalog-waits--2026-10-01)). "Terrain materials", "Damage meter" and "Nameplates" add more. Reported, not changed.
- **Frames just before the loading screen hides** (up to 0.3-0.9 s under load), all covered by the loading screen:
  - the `LoadTerrain` event, 52-2713 ms: under load 24 the `terrain.enter_loading` span (showing the Loading screen and resetting the world) took 507 ms, `terrain.map_id` and `terrain.request_map` under 50 ms;
  - showing the Loading screen: 43-275 ms;
  - "Character preview" at the character-select → loading transition: 0.4-2.2 s.
- **Units of work above the 8 ms budget.** The budgets are checked before each unit of work. The units are now small, but they still exceed 8 ms:
  - one humanoid NPC's `build_model` (126-151 batches): 20-35 ms at low load, 80-170 ms at load 24 (the "Unit visuals" step);
  - one terrain chunk's first-use ground textures (RGBA uploads): "Terrain materials" steps of 22-46 ms;
  - a WMO shader variant's first compile: 35-94 ms.
- **Throughput under load.** Spawning gets at most 8 ms of main-thread time per frame. At 7-10 fps (load 30-44) the ~10,000 placements, WMO doodads included, do not drain in 300 s. An adaptive budget or retail-style distance ordering would help, but neither is implemented.
- **Memory:** the parsed-model cache (`creature::MODELS`) and the doodad/WMO asset caches are kept for the whole process, across world changes. In round 3 the branch peaked at 2.73-2.80 GB VmHWM and base at 3.66-4.09 GB, with every object spawned in R3 base 2 and both branch runs. The caches' size across world changes is not measured.
- **Shutdown:** in R2 branch 3, Godot's main thread stayed in `pthread_join` on one of Godot's own threads after the fixture failed. No `unit-visuals`, `world-objects` or `spell-assets` threads were left, and the process was stopped by PID.
- **No retail source:** none was found for the loading-screen wait set, or for how retail streams assets on its threads (see Fix).
- **Tests not run on Depot:** `world_models` and `wmo::scene` unit tests need `data/` files that are not in `godot/depot-test-assets.txt`. They fail there with missing-file errors, as on master. `m2_submesh_arrays` (core) and the `loading` tests pass.

## Retained integrated performance — 2026-10-01

These are actual failed workloads, not a retained-original baseline or accepted settled performance. Historical A/B results above retain their original object-only definition of “settled”; do not apply that definition to the integrated measurements.

| Case | Actual observations | Proof boundary |
| --- | --- | --- |
| Run1, exit1 | Loading 22.068802 s, maximum loading frame 8053.257 ms. Object pending briefly 0, then final 5803. Seven memory phases; nominal window 60.172530 s. | Duration adequate, readiness unstable: invalid settled window. Raw distributions/logs remain useful; loading and post-hide fixture limits failed. |
| Readiness test | Pure behavioral RED at `f52e58c9`: 2 fail/2 pass; `bfc4e373` GREEN 4/4. Predicate requires terrain pending 0, object pending 0 and unit-visual pending 0; observation also monitors unchanged parsed tiles. | Fixes diagnostic readiness, not a production streaming-terminal signal or successful runtime performance case. |
| Run2, exit1 | Loading 172.615 s, maximum loading frame 2429.502 ms; drain 251.532 s. Nominal window 60.077 s; readiness changed even though object pending ended at 0. | Invalid settled window again. Final object zero does not override changed readiness or fixture-limit failures. |
| Memory/environment | Seven phase samples preserved; run2 RSS around 3 GB, HWM 3189628 KiB. Shared-host contention, degraded FIFO pacing and unsupported disabled-VSync request retained. | Lifetime process HWM, not per-phase allocation/VRAM/leak proof. No isolated performance baseline, product-budget violation or shutdown acceptance. |

Run1's root measurement error was treating one empty object queue as terminal readiness: terrain polling can expose another parsed tile before the later object-sync step queues its placements. Loading hidden intentionally precedes full surrounding streaming. The corrected fixture monitors all three pending counts and parsed-tile stability; run2 still rejects its window. Do not substitute a longer sleep or relax fixture thresholds (1000 ms loading / 100 ms after hide). These policies are not supplied product budgets.

`scripts/performance/measure.py` retains wall-clock phase arrays, nearest-rank distributions, seven `/proc` RSS/HWM samples, combined logs, exit status and before/after input snapshots; `test_measure.py` has retained targeted GREEN9/9 at `2c92ff3f`, not native performance acceptance. No baseline supplied: comparison remains a gap across workload/assets/options/cache/hardware/resolution/renderer/server/camera/sampling/resource limits. Checkout HEAD observations are not native binary provenance; run records separately identify the supplied `f23343bb` native artifact with retained build-ID/tool-return limits.

Only new owned dev account `fb_perf_01a0dea2` / character `Fbretainperf` was used; no server restart or existing-account changes were authorized. Account data remains retained; no supported deletion was observed. Pre-existing spell-attachment errors belong to another owner: report, do not repair. Water, appearance and remaining tooling handoffs remain outside this scope. [[authored-skybox-black-output#Retained original/native evidence — 2026-10-01|Skybox evidence]] has its own dark-phase parity limits; neither investigation closes the retained goal or full conversion.

## Catalog waits — 2026-10-01

The longest loading frame at world entry was the first `NpcMessage::Inventory`: 9.2, 9.7 and 54.9 s (`PROFILE account.event Npc(Inventory)`, load 8-15). `item_catalog::catalog()` was a `OnceLock::get_or_init` that `warm_item_catalog` started on a spawned thread, so `InventoryState::apply_snapshot` → `stack_slot` on the main thread waited for the whole `ItemSparse.csv` parse (49 MB, 213,415 items; 12.6-77 s on the shared host). Retail never waits: `C_Item.GetItemInfo` returns nil and the item updates on `GET_ITEM_INFO_RECEIVED`.

The same wait or a synchronous first use on the main thread, all fixed on branch `catalogwait`:

| Table | Was | Now |
|---|---|---|
| Item catalog + item icons (`src/game/item_catalog.rs`, `item_icons.rs`) | main thread waited in `get_or_init`; icons parsed synchronously in `stack_slot` | one background load holds both; readers get `None`. Items received meanwhile show `INV_Misc_QuestionMark` and no name, their tooltip the red `RETRIEVING_ITEM_INFO`; the "Item data" frame step calls `InventoryState::refresh_item_data` in the first frame the catalog is loaded |
| NPC gear rows (`world_models.rs` `VisualCatalogs.gear`) | `WorldUnits::upsert` read them with `get_or_init` while the unit-visuals worker loaded them | `loaded_gear()` never waits; weapon class, NPC pose and player stand state resolve when the unit's visual arrives (every visual load reads the rows first) |
| Spell visual catalog (`spell_effects.rs`) | the first cast `join`ed the worker | `BackgroundLoad` poll; casts and swings meanwhile show no kits, replicated casts and auras start theirs once it loads |
| Entrance difficulty catalog (`entrance_bar.rs`) | loaded in the first in-world frame (273-372 ms "Entrance bar") | loads from client start; the bar stays hidden until it is |
| Minimap `AreaTable`/`ChrRaces` (`minimap.rs`) | loaded in the first in-world frame | loads from client start; no zone text, and quest headers unnamed, until it is |

Measurements (`godot/tests/world_entry_frames.gd`, `GAME_PROFILE_MS=50`, private server, character with 13 bag and equipment items, debug builds; base `7e666dfe` = master `b3e546e2` + event labels):

| Run | Load | Longest loading frame | Its cause | Item catalog loaded after |
|---|---|---|---|---|
| base 2 | 8.5-11.2 | 9222 ms | Npc(Inventory) 9188 ms | — |
| base probe | 8.4-15.1 | 55038 ms | Npc(Inventory) 54912 ms | — |
| base 3 | 6.6-14.3 | 9731 ms | Npc(Inventory) 9697 ms | — |
| fix 2 (`7b31ceb5`) | 4.6-6.4 | 243 ms | — | — |
| fix 3 (`1417f847`) | 6.8-29.7 | 1612 ms | Replication 596 + LoadTerrain 608 + Screen(Loading) 372 | 77.2 s |
| fix 4 (`81e0db3f`) | 20-28 | 4298 ms | LoadTerrain 2713 ms (before its spans) | 36.5 s |
| fix 5 (LoadTerrain spans) | 24-27 | 1038 ms | LoadTerrain → `terrain.enter_loading` 507 ms | 29.4 s |

In no fix run does an inventory event or the "Item data" step reach 50 ms; "Entrance bar" took 273 ms in fix 3 and stays under 50 ms from `81e0db3f`. The first in-world frame was 385-1595 ms in base and 401-1865 ms in the fix runs, dominated by "Minimap" tile decoding (see Remaining gaps) at very different host loads; it is not comparable across these runs.

Proof: `godot/ui-model/tests/item_catalog_pending.rs` was RED on master (`apply_snapshot waited 7.64 s`) and is GREEN; it asserts the pending slots and retrieving tooltip, then Linen Cloth, Ruined Pelt and the equipped Worn Shortsword after `refresh_item_data`. Live: `godot/tests/world_entry_item_data.gd` requires the inventory to arrive before the catalog, then the catalog's names, icons and quality in bags and equipment and the white Linen Cloth tooltip (PASS at `bae583ab`, capture `item-data-linen-tooltip.png`). `background_load` and `world_models` unit tests cover the non-blocking reads. The preserved Bevy client shares the item catalog change (its inventory gets the same refresh system) but was not compiled.

Fixed later on branch `stalls` ([below](#character-select-and-enter-world-loads--2026-10-01)): the Character preview step at character select takes 1.0-16.4 s; the character model build there loads `CustomizationDb`, `OutfitData`, the 143 MB community listfile and the player model tables synchronously. That is before Enter World.

## Character select and Enter World loads — 2026-10-01

Branch `stalls`. Each was a synchronous load or wait on the main thread; each now runs on a thread from client start or from the request, and its result applies when it arrives.

| Stall | Root cause | Now |
|---|---|---|
| Character preview 1.0-16.4 s per selected character | `load_character` built the model in one frame: `CustomizationDb`, outfit tables, model extraction, texture composition; `try_resolve_runtime_model` read and indexed the 143 MB community listfile only to discard the path | customization catalog loads from client start (`BackgroundLoad`); `prepare_player_parts` runs on a `character-preview` thread; the camera frames the shot once the presentation is known, the model appears when loaded; runtime item models resolve through `ModelFileData` by FDID, no listfile |
| 327 ms in the frame CASC startup ends (shown at character select) | `initialize_sound` read `AreaTable`, the zone music catalogs and the footstep files; footsteps scan the community listfile (no `FootstepTerrainLookup`/`SoundKitEntry` DB2 is extracted, so the scan stays) | a `sound-data` thread reads them from client start; the "Sound data" step hands them to `NativeSound`; until then no zone music, ambience or footstep plays |
| Minimap tile decode, first in-world frame | every tile under the view extracted and decoded synchronously | `minimap-tiles` `AssetLoader`; the composite redraws as tiles arrive |
| LoadTerrain → `terrain.enter_loading` (catalogwait's 507 ms) | (1) dropping the campsite preview joined its terrain worker and world-objects `AssetLoader` workers, waiting for each worker's read or parse in hand (`preview_reset.drop` 58 ms idle, 1.2-2.1 s at load 20-36); (2) the loading screen was built twice | `AssetLoader` and `StreamedTerrain` no longer join on drop: the worker finishes its task and exits, the result is discarded (CASC cache writes are temp-file + rename, so exit mid-task leaves no partial file); the loading screen already up is kept |
| `player.appearance_textures` 21-225 ms when a model shows | `Image::generate_mipmaps` on the main thread per composed texture | the worker builds the mip chain with Godot's RGBA8 algorithm (`_generate_po2_mipmap`, `average_4_uint8`, godot `core/io/image.cpp`); the main thread only creates the image |

Measurements (`GAME_PROFILE_MS=50`, private server, debug builds, host load from other agents 3-36 throughout; every main-thread cost scales 3-10x with load, so compare within a row):

| Phase | Before (`50971f03` spans build) | After |
|---|---|---|
| Character select, longest frame | 3663 ms (client 3491), load 4.5 | 227 ms at load 19 (`4dbb590e`); 439 ms at load 2 before the sound fix, all from `startup.sound` |
| Preview drop on Enter World | `screen.preview_reset` 124 ms (load 5), 1252-2114 ms (load 20-36) | under 50 ms at load 13-22 |
| LoadTerrain | 289 ms (load 5) | 72.9 ms (load 13, `4dbb590e`) |
| Longest loading frame | 594 ms (load 5) | 309.5 ms (load 13) |
| "Minimap" step | 133-602 ms (catalogwait runs) | at most 54 ms (load 22, `95788a28`); under 50 ms in the other runs |

Tests: `asset_loader::tests::dropping_the_loader_does_not_wait_for_the_load_in_hand` and `terrain::streaming::tests::dropping_the_stream_does_not_wait_for_the_read_in_hand` were RED (drop waited 1.00 s and 1.10 s for the held task) and are GREEN; `mip_chain_tests` pin the averaging; `outfit_data_tests` covers model resolution without the listfile. Live fixture `godot/tests/charselect_preview_frames.gd` selects two characters in turn and fails on any frame over 100 ms; it still fails on the loaded host.

Remaining main-thread work over 100 ms on the loaded host, none of it these loads:
- Authored UI textures are read and decoded on the main thread at every use, uncached (`ui/assets.rs` `load_file`): `screen.attach CharacterSelect` 133-690 ms, the loading screen 95-357 ms, and the first in-world frames (`4719247.blp` 255 ms in Targeting, `135891.blp` 148 ms in Spells).
- Godot node construction: `player.build_body` 27-45 ms idle, 112-429 ms loaded; campsite objects and materials 12-38 ms idle, 90-190 ms loaded.

Evidence: `data/diagnostics/stalls/` (`ab1-*`, `ab2-*`, `ab3-*` A/B logs, `red-drop-join*.log`, `green-drop-join.log`, `charselect-fix-*.log`).

## Shader compilation ahead of need — 2026-10-02

Branch `stalls`. Godot creates a `Shader`'s rendering-server shader, and compiles it, the first time something asks for its RID (`Shader::get_rid` → `_check_shader_rid` → `shader_create_from_code`, godot 4.7.2 `scene/resources/shader.cpp`); `ShaderMaterial::set_shader` asks. Every first use of an M2 pipeline variant, WMO variant, the terrain shader or a liquid shader therefore cost 10-50 ms of main-thread CPU in the frame that built the first material, 2-3 per character model. Retail ships its shaders precompiled.

`shader_warmup.rs` records each shader the scene uses in `user://used_shaders.txt` (`m2 <gx_blend two_sided depth_test depth_write>`, `wmo <two_sided blended clamp_s clamp_t>`, `resource <res:// path>`). Later runs compile the recorded shaders one per frame (an opaque M2 pipeline's scenery-fade variant with it) during asset startup and on the login and loading screens, so character select and the world take compiled shaders. Compiled resource shaders stay loaded so `ResourceLoader` hands their users the same instances. The first run on a machine still compiles on first use; it records what later runs compile ahead.

The authored campsite catalog (`WarbandScene*` and `UiTextureAtlas*` CSVs, 31-44 ms) was also read on the main thread when character select attached; a `campsites` `BackgroundLoad` reads it from client start.

Measurements (`GAME_PROFILE_MS=5`, private server, debug build `37e1a3b9`, idle host, load 2.4-4.8; on-CPU ms from `profile.rs` spans; cold = no record, warm = the record a previous run left):

| Phase | Cold | Warm |
|---|---|---|
| Character select, longest frame (fixture wall) | 136.4 ms, 1 frame over 100 | 53.9, 60.9, 78.5, 60.2 ms in 4 runs, none over 100 |
| Character select, longest `client.frame` CPU | 122.2 ms | 50.3-66.6 ms |
| Character select shader compiles on first use | 6 M2 (133 ms), 3 WMO (81 ms), terrain 16 ms, water 10 ms | none in 3 of 4 runs; one WMO variant (24 ms) not yet compiled when startup ended in 1 |
| `player.build_body` max | 70.3 ms | 26.7-33.5 ms |
| `screen.attach CharacterSelect` | 45-61 ms before the campsite move (`a70f1adb`) | 13-16 ms |
| World entry shader compiles on first use | 9 M2 (185 ms), 5 WMO (125 ms), terrain 19, water 25 | none |
| Warmup frames (asset startup, login, loading) | none | 11-17 compiles, 25-53 ms each |

World entry still fails its 100 ms fixture policy on both: the warm run's slow frames came 10-23 s into the world with render CPU 40-96 ms (steady-state draw, not loads).

Tests: `godot/tests/used_shaders.gd` (headless) was RED before each step (no compile entry point; then 0 of 2 and 2 of 3 recorded shaders compiled) and is GREEN; `background_load::tests::wait_takes_the_result_once_the_thread_is_done`. `water/terrain/liquid/wmo/m2_material` and `m2_scenery_fade` pixel fixtures pass on the final build.

Remaining character select frames, all under 80 ms idle: campsite terrain tiles (`preview.background.materials` up to 39 ms), campsite objects (up to 33 ms), `sky.prepare_batches` 17-22 ms (2 sky variants, 6-9 ms each, are not recorded), character body build 27-34 ms.

Evidence: `data/diagnostics/stalls/` (`w6-*`, `w7-*` final cold/warm logs, `w1`-`w5` earlier variants, `build-warm*.log`, `test-warm5.log`).

## Scene light rebound every frame — 2026-10-02

Branch `settlegate`. Stormwind perf runs reported "World did not settle" (3 of 5 at load 7-20): the readiness predicate (`world_entry_readiness.gd`: terrain, object and unit-visual pending all 0) never held within 900 s. Nothing was stuck; the object queue drained at its 8 ms per frame, but frames took ~1 s. With `GAME_PROFILE_MS`, `step=World lighting` cost 400-710 ms of every frame (growing with spawned objects), because:

- the server clock advances `world_minutes` continuously, so `WorldLighting::sync`'s sample differs every frame (sun direction and LightParams interpolation; solarityclient samples band colours at whole half-minutes but the sun direction at the continuous day fraction, so the light changes per frame there too);
- every change rebound ~27 light and fog uniforms on every material of every spawned model: 21,147 meshes took 490 ms (65 ms of it `find_children`), unit visuals 130-160 ms, terrain 45 ms (temporary spans, load ~14).

`frame_benchmark.gd` fixes the time only after the world settles, so the steady-state benchmark never saw this; real play with the server clock did (Stormwind ~1-2 fps).

Fix: the scene light and fog are Godot global shader uniforms (`project.godot` `[shader_globals]`, `shaders/scene_light.gdshaderinc`, `shaders/retail_fog.gdshaderinc`), written once per change by `TerrainLight::bind_scene`, as WebWowViewerCpp fills its per-scene `SceneWideParams` (`commonLightFunctions.slang:26-52`) and the Bevy client shared one `RETAIL_SCENE_LIGHT_BUFFER` ([[retail-lighting]]). `bind_model` only marks a material `scene_light` with `fog_mode` 1, so models are rebound only when the light arrives or is cleared. Character previews, character creation and quest markers keep their own light uniforms; terrain still rebinds its environment cubemap per change, and water its colours.

| Run (Stormwind trade, private server) | After InWorld | Frames during settle |
|---|---|---|
| base `59e9791f`, load ~5 | settled 488 s; 2,836 pending at 300 s | ~1 fps |
| fix, load ~7.4 (`GAME_PROFILE_MS=100`) | settled 31.9 s | 9-10 fps, no lighting span over 100 ms |
| fix, 10 consecutive (`LOOP_SETTLE_S=300`), load 6.7-16.8 | 10/10 settled in 26-162 s (median 49 s) | 3-10 fps |

Evidence: `data/diagnostics/settlegate-2026-10-02/` (`base-prof-1`, `diag-1`, `fix-prof-1`, `green-*`). Loading itself (~127 s at load 7) is the separate Loading gate ([world-loading](../../specs/world-loading.md)).

## Sources

- `data/diagnostics/retained-performance-20261001/run1/` and `run2/` — actual `result.json` and `stdout.log`; manifests/config retained alongside them.
- `/tmp/claude/retained-conversion-20/{performance-run1-verification.md,perf-settled-boundary.md,readiness-red.log,readiness-green.log,performance-runner1.log,performance-runner2.log,C.md}` — arithmetic/source-boundary audit, test proof and runner status; run2 rounded summary is not a separate settled distribution.
- [measure.py](../../../scripts/performance/measure.py), [test_measure.py](../../../scripts/performance/test_measure.py), [world_entry_frames.gd](../../../godot/tests/world_entry_frames.gd), [world_entry_readiness.gd](../../../godot/tests/world_entry_readiness.gd) — runner and diagnostic readiness contract.
- `data/diagnostics/stalls/` — character select and Enter World A/B runs (`ab1`-`ab3`), drop-join RED/GREEN test logs.
- `data/diagnostics/catalogwait-2026-10-01/` — catalog-wait runs (`world-entry-{base,fix}-*.log`, `item-data-fix-*.log`, `red-test.log`, `uimodel-test-2.log`, `godot-lib-test-3.log`).
- `data/diagnostics/firstload-2026-09-30/ab/` — the original first-load A/B logs with the step timings.
- `/home/osso/.worktrees/.worldentry-artifacts/runs/` — this A/B's client logs (not in the repo).

## See Also

- [[spell-visuals]] — the first-load A/B that found these stalls, and the spell-asset loader this fix reuses.
- [[godot-stormwind-fps]] — steady-state draw and animation cost.
