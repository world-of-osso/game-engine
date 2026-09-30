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

- **First in-world frame** (163-326 ms of client time): HUD steps set up at once. "Minimap" takes 76-176 ms and loads its tiles and catalogs synchronously (`minimap.rs` `load_tile`). "Terrain materials" and "Entrance bar" add more. This is not in the Account or World objects steps; it is reported, not changed.
- **Frames just before the loading screen hides** (up to 0.3-0.9 s under load), all covered by the loading screen:
  - the `LoadTerrain` event: `read_map_id` and the map request, 52-618 ms;
  - showing the Loading screen: 43-275 ms;
  - "Character preview" at the character-select → loading transition: 0.4-2.2 s.
- **Units of work above the 8 ms budget.** The budgets are checked before each unit of work. The units are now small, but they still exceed 8 ms:
  - one humanoid NPC's `build_model` (126-151 batches): 20-35 ms at low load, 80-170 ms at load 24 (the "Unit visuals" step);
  - one terrain chunk's first-use ground textures (RGBA uploads): "Terrain materials" steps of 22-46 ms;
  - a WMO shader variant's first compile: 35-94 ms.
- **Throughput under load.** Spawning gets at most 8 ms of main-thread time per frame. At 7-10 fps (load 30-44) the ~10,000 placements, WMO doodads included, do not drain in 300 s. An adaptive budget or retail-style distance ordering would help, but neither is implemented.
- **Memory:** the parsed-model cache (`creature::MODELS`) and the doodad/WMO asset caches are kept for the whole process, across world changes. They are not measured yet; the fixture now prints `FIXTURE MEMORY` (VmRSS/VmHWM).
- **Shutdown:** in R2 branch 3, Godot's main thread stayed in `pthread_join` on one of Godot's own threads after the fixture failed. No `unit-visuals`, `world-objects` or `spell-assets` threads were left, and the process was stopped by PID.
- **No retail source:** none was found for the loading-screen wait set, or for how retail streams assets on its threads (see Fix).
- **Tests not run on Depot:** `world_models` and `wmo::scene` unit tests need `data/` files that are not in `godot/depot-test-assets.txt`. They fail there with missing-file errors, as on master. `m2_submesh_arrays` (core) and the `loading` tests pass.

## Sources

- `data/diagnostics/firstload-2026-09-30/ab/` — the original first-load A/B logs with the step timings.
- `/home/osso/.worktrees/.worldentry-artifacts/runs/` — this A/B's client logs (not in the repo).

## See Also

- [[spell-visuals]] — the first-load A/B that found these stalls, and the spell-asset loader this fix reuses.
- [[godot-stormwind-fps]] — steady-state draw and animation cost.
