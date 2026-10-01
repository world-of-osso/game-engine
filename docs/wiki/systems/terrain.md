# Terrain

ADT terrain is loaded from split WoW map tiles. The engine renders heightmap meshes with texture layer compositing, spawns doodads and WMOs from placement data, and uses a custom rotation formula to convert WoW-space placement angles into Bevy-space transforms.

## Local Streaming Cache

Official tile requests resolve their listfile FDID and call the existing `AssetResolver::ensure_cached` at `shared_data_path("terrain/<fdid>.adt")`. Missing disk files trigger local CASC extraction rather than a permanent streaming failure. Declared `_tex0` and `_obj*` companions use the same cache mechanism; an absent optional listfile entry returns `None`, while extraction errors include the WoW path, FDID, and destination. Existing explicitly named local sidecars remain supported. Official streaming no longer prefers a potentially stale named tile over the canonical FDID cache or searches alternate checkout caches.

`terrain_tile::resolver_tests` uses temporary filesystem caches to prove root/companion extraction, byte-preserving reuse, named-sidecar behavior, and lookup/extraction errors. Bounded native cold-edge capture extracted two roots and their declared companions from local CASC; one tile fully spawned before the 30-second cap. A third tile was not observed before the cap, not classified as an extraction failure. See [[npc-motion-validation]].

## Shared terrain surface selection

`70e047ff` makes `src/rendering/terrain/terrain_surface_data.rs` the Bevy-free source of dominant effect ID, dominant texture FDID, and footstep surface selection for the root client and `godot/core`. The first layer weighs 1,000,000; subsequent layers use summed alpha bytes; equal weights favor the later layer. Zero effect IDs are skipped, while an invalid texture index ends FDID selection. Resolved effect surfaces precede texture-path classification; unresolved paths remain Dirt. Bevy retains its existing GroundEffect loading/cache and FDID-path lookup in the heightmap adapter; loaders are unchanged. Native tile ingestion now computes and stores one surface per loaded height-grid chunk with `_tex0` layers. `StreamedTerrain::surface_at(x,z)` uses the same tile/half-open chunk coverage as `area_id_at`; absent tiles, texture companions, or chunk coverage return `None`, and unclassified present chunks use the shared Dirt policy. This shared prerequisite is consumed by the later bounded native local-player footstep path; it does not independently prove that path. Verifier839 remains a separate pending claim.

## ADT Split Files

Each tile is three files:
- Root `.adt` — MCNK chunks with heightmaps and normals
- `_tex0.adt` — texture layer compositing (MDID/MHID for diffuse/height FDIDs); decoding and the shader blend mode follow the map WDT's MPHD flags ([terrain-blend-steps](../investigations/terrain-blend-steps.md))
- `_obj0.adt` — MDDF doodad placements and MODF WMO placements

The engine loads all three. Finding companion files uses the community listfile (path-based sibling lookup).

## Layer textures and specular mask (Godot)

Each MCLY layer samples its MDID texture as authored (WebWowViewerCpp `adtObject.cpp:528`). Modern MDID entries are the `_s.blp` files: the same RGB as the plain diffuse, with the layer's specular mask in alpha (`adtShader.frag.slang`: `specBlend` from the layer alphas). Before `a345a8d8` the Godot client swapped each `_s.blp` for its plain `.blp`, whose alpha is 255 everywhere (Elwynn cobblestone 187099 vs 187100: alpha 255 vs 17..187, mean 48), so every terrain pixel took full power-20 specular: grass and roads glared under the night light. Proof: `godot/rust/src/terrain/textures.rs` tile 32_48 test RED/GREEN (`data/diagnostics/worldvis-2026-10-01/terrain-spec-{red,green}.log` / `godot-lib-green.log`), Northshire night before/after `data/diagnostics/worldvis-2026-10-01/spec-before-after-night.png`. The Bevy client keeps the old swap (`src/rendering/terrain/terrain_material.rs`).

## Ground detail (Godot)

Detail doodads (grass/flower cards) come from the MCLY layer each MCNK cell selects in header 0x40 (2 bits per cell, little-endian rows), unless header 0x50 or a hole excludes it; the layer's GroundEffectTexture row gives density and four weighted GroundEffectDoodad models. `godot/core/src/ground_detail.rs` reproduces the 12340 client's scatter and vertex expansion exactly (solarityclient native fixtures, `godot/core/tests/ground_detail.rs`); the Godot client details chunks within 140 yards of the camera. Gotchas: an effect density of 0 means 8 doodads per cell; retail effects mix models from other zones (Northshire layer 1106 scatters Duskwood `dskgra03.m2`); the chunk seed takes the global row (second ADT file number × 16 + MCNK index y) in the high word. Spec: [ground-detail](../../specs/ground-detail.md).

## Horizon (Godot)

Beyond the streamed tiles the client draws the map's WDL (`world/maps/<map>/<map>.wdl`; MAOF indexed by the second ADT file number × 64 + the first, MARE 17×17 corners then 16×16 centres in whole yards, MAHO face masks) as the 12340 client's horizon fan (solarityclient `terrain/low_detail`), in the scene fog's colour (fully fogged, `retail_fog_color_at`), within `farclip` 777 × `horizonFarclipScale` 4 yards. Depth: reverse-Z `w × 2e-6`, in front of the sky models (`1e-6`) and behind geometry nearer than ~980 yards of the 0.1/1000 camera. Loaded tiles hide their WDL tile. Not drawn: retail's textured `_lod.adt` terrain with `maptextures` (no working reference implementation; WebWowViewerCpp's is a TODO).

## Native MCNK Surface Lookup

`NativeTerrainAssets` reads GroundEffectTexture (FDID 1308499) and TerrainTypeSounds (FDID 1284822) once per reader through the local CASC/cache resolver. Shared parsers map dominant effect → terrain sound name → footstep surface before falling through to dominant texture FDID's listfile path. Invalid/unavailable DB2s report contextual errors once and leave terrain rendering available; texture classification still applies. Chunk alpha weights are calculated at tile ingestion, not per-frame lookup. Metadata lives with each parsed tile and clears on stream reset. This is an actual native `surface_at` metadata lookup, separate from `70e047ff`'s shared prerequisite. `b773ecfd` final bounded footstep verification proves its integrated local-player use, not an isolated terrain-query verifier. WMO material surfaces are now ranked once per root through shared `wmo_surface_data`. The tile reader stores eligible `_obj0` MODF bounds/surfaces beside parsed collision floors, before node/physics spawning; the WDT payload retains its global placement surface. `StreamedTerrain::surface_at_position([x, y, z])` selects the smallest-volume inclusive-containing WMO bounds (global extents use the WDT origin, MODF extents use absolute ADT conversion), then terrain surface, then Dirt. Missing material paths are excluded as in the original renderer. This query alone does not prove trigger/playback; `b773ecfd` final bounded footstep verification proves its integrated local-player use, not an isolated WMO-query verifier.

## Native MCNK Area Lookup

`959112e9` exposes the nonzero `area_id` of the loaded MCNK under the local player as `account_state.area_id`. The native query matches the current player position to the loaded tile and height-grid chunk; unloaded tiles, zero area IDs, invalid coordinates, and cleared terrain generations yield no area.

Targeted `area_query` tests are 2/2, covering chunk/tile bounds and reset/reload clearing. Independent verification remains pending. This is only the terrain-area prerequisite: it does not resolve an `AreaTable` parent/zone, select zone music, start audio playback, or establish native runtime behavior. See [[sound]] for the existing Bevy-only zone-music system.

## ADT MH2O Water

`b412f7e4` restores native rendering for root-ADT MH2O water. The original water geometry builder and 262,144-byte procedural-normal map are shared between the Bevy path and `godot/core`; native creates authored water beneath each tile and uses the original normal/fresnel/specular/depth-alpha shader path. Water samples the shared material clock, and tile reset removes water nodes and materials. The final native fixture exits 0 (`/tmp/claude/adt-water-runtime-clock-typed.log`): authored geometry covers its lake point; 4,608 pixels change and are translucent; paused clock pixels are stable; a 2-second advance changes at least 64 pixels; released nodes are invalid after client cleanup. Main inspected `swimming.png`, `swimming-shoreline.png`, and `swimming-water-isolated.png`, which show blue translucent water over the authored shore. `a160bdbf` changes only test-import order: final root and Godot `fmt --check` pass; earlier root/native checks, core geometry 3/3, normal-byte 2/2, and runtime proof are reused. Native checks retain two known WMO warnings; the narrow normal test retains one unused-import warning. `d44ef597` proves ordinary reconnect water reset/recreation in the owned UDP fixture (`/tmp/claude/adt-water-reconnect-d44ef597.log`, exit 0): 65 seconds of server silence exceeds the 60-second Netcode timeout; `PendingConnect` clears terrain/units and invalidates old Water-node/material weakrefs. Token authentication preserves the selected character through roster reordering, terrain refresh creates distinct water/material IDs, their shared clock advances, input restores, and the headless client exits cleanly. The fixture now spawns at verified actual-ADT Y=112.87991 then Y=117.38283 instead of stale Y=83; production physics is unchanged. `6bd26680` adds bounded same-map transfer resource/state proof in the built native fixture (`/tmp/claude/water-transfer-build-6bd26680.log`; `/tmp/claude/adt-water-transfer-6bd26680.log`, both exit 0): `INITIAL_READY` → `TRANSFER_LOADING` → `TRANSFER_WATER_READY` → `TRANSFER_READY`; `NewWorld` releases old Water-node/material weakrefs, recreates distinct water/material IDs whose shared clock advances, and recreates terrain while retaining the player's identity/model and facing 0.5. The fixture preserves its authored `TransferAborted` error and emits exactly one `WorldPortAck`; its independent verifier passes Godot `cargo fmt --check` and native transfer-fixture `cargo check` (`/tmp/claude/verify-water-transfer-6bd26680.md`). `7059e412` retains the default headless behavior and existing lifecycle/Ack while adding explicit `GODOT_TEST_VISUAL=1` sampling; shader, physics, and protocol are unchanged. Main's example build exits 0 (`/tmp/claude/water-transitions-gpu-build-7059e412.log`). Owned UDP transfer and reconnect runs exit 0 on Vulkan AMD: each recreated destination water sample has 4,608 changed translucent pixels; frozen-clock pixels remain stable, then +2,000 ms animates them (`/tmp/claude/adt-water-transfer-gpu-7059e412.log`; `/tmp/claude/adt-water-reconnect-gpu-7059e412.log`). Main inspected `data/diagnostics/godot-conversion/transfer-water-isolated.png` and `reconnect-water-isolated.png`: 96 px isolated viewport samples of copied production meshes/materials over red backing, not whole-world screenshots. Cage/Xwayland shutdown warnings occur after success and do not denote a Godot runtime failure. This establishes bounded sampled GPU behavior after those transitions, not different-map/destination or real-server coverage, whole-world rendering, image equality, or full visual parity; independent GPU/source audit and targeted compilation pass; `2e8ff63f` fixes import order and final Godot formatting passes (`/tmp/claude/verify-water-transitions-gpu-final.md`), reusing unchanged runtime proof. `3341c9b6`'s missing `WorldTerrain/Tile32_48/Water` is the actual production RED that motivated the restoration; historical fixture type/compile/parse failures are not production defects. The procedural shader was replaced by the retail LiquidType water material; see [[northshire-pale-water]]. WMO water, the non-water LiquidMaterials, buoyancy and performance remain open.

Unresolved char-select missing-row/omitted-mesh evidence is maintained in [Native LiquidObject missing rows](../investigations/northshire-pale-water.md#native-liquidobject-missing-rows--unresolved), separate from the bounded water proofs above.

### Bounded lateral-swimming fixture (2026-09-28)

Test-only `61a8bf98`, fixture-only `6de10e60`/`00a9871e`, and fixture-phase extraction `e883fe1b` leave production unchanged. Main rebuilt `e883fe1b` (`/tmp/claude/swimming-lateral-build-e883fe1b.log`, exit 0) and reran the root-launched Vulkan loopback (`/tmp/claude/swimming-lateral-runtime-e883fe1b.log`, exit 0): W dry→wet, idle Space, wet W+Space, A→SwimLeft 43, D→SwimRight 44, reverse S wet→dry, no jumping, and released UDP quiet. Decoded wet packets remain yaw-relative with `swimming=true`, `jumping=false`; authored sampling remains X=-8562..-8554/Z=480..500 at depth ≥3.761963 (`/tmp/claude/swim-lateral-native-parser-full.log`). The helper remains 4,608 changed / 4,607 translucent pixels; main again inspected `swimming.png` and `swimming-shoreline.png`. Scoped final verification (`/tmp/claude/verify-swimming-lateral-final.md`) confirms source/behavior/runtime evidence plus fresh Godot `fmt --check` and targeted native fixture check. Named phases resolve the prior `run()` cognitive 67/cyclomatic 62 result (`run()` is 3/10), but strict readability remains unmet: `advance_fixture_marker()` is cognitive 10/cyclomatic 28 (>20). Its explicit Stage×marker transition table remains intact; no suppression or threshold-only abstraction is claimed. This is not buoyancy, speed, real-server, whole-world visual, parity, or conversion proof.

`MHID` entries are optional height-texture FDIDs. A zero entry denotes an absent height texture, not BLP FDID 0: terrain material loading and CPU decoding preserve that slot as `None` without resolving a cache path or logging an asset failure. Nonzero missing IDs still use the existing asset-failure diagnostic. `height_texture_zero_slots_stay_absent_even_when_zero_blp_exists` proves the sentinel stays absent even if a readable `0.blp` fixture exists, while preserving a neighboring nonzero slot.

**Critical tile ordering bug (fixed)**: `ensure_warband_terrain_tiles()` was sorting the tile list alphabetically, which reordered `[primary=(31,37), supplemental=(31,36)]` to `(31,36), (31,37)`. The scene loader took `next()` and loaded the supplemental tile as primary, never loading the mountain tile `2703_31_37.adt`. Fix: preserve primary-first ordering and append only distinct supplemental tiles. See [adventurers-rest-mountain-brief.md](../adventurers-rest-mountain-brief.md).

## Streaming Neighborhood

The default load radius is one tile: retain a 3×3 neighborhood around the player's current tile. A zero radius previously unloaded adjacent terrain and its heightmap while approaching a tile edge, leaving visible void beyond the single retained tile. This is separate from WMO placement alignment. The streaming regression uses real loaded entities and heightmaps, keeps other requested tiles pending, and crosses the 32,48→32,49 boundary: both nearby tiles remain while a distant tile is removed. Bootstrap queues all nine tiles; existing pending-load limits and retries are unchanged.

The initial native stream reported a radius-one neighborhood with nine loaded tiles, nine heightmap tiles, and zero failed tiles; the observed `streaming-fixed.webp` view lacks the prior nearby tile-edge void. This does not prove every region or a complete cold ring.

## World Object Placement (MDDF/MODF)

WoW ADT stores object rotations as `[X, Y, Z]` Euler angles. These are converted to Bevy using `placement_rotation()` in `src/terrain_objects.rs`:

```
stored [X, Y, Z] → model rotation [Z, Y - 180, -X] → EulerRot::YZX
```

This was derived by visual validation against Adventurer's Rest campsite props. The key reference was Noggit3's `from_model_rotation()` (`[-Z, Y-90, X]` in YZX), which was the starting point but required further adjustment for this renderer's full transform chain. See [world-object-rotation-investigation-2026-03-22.md](../world-object-rotation-investigation-2026-03-22.md).

**Tests**: `placement_rotation_matches_current_model_rotation_formula` and `placement_rotation_zero_matches_current_yaw_correction` in `terrain_objects.rs` lock in the current formula.

## Maps and WMO-only Maps

`AdtManager.map_name` is the server's `Map.db2` Directory, lowercased (`azeroth`, `kalimdor`, `stormwindjail`); `LoadTerrain` and `NewWorld` set it ([instances spec](../../specs/instances.md)). Setting a map reads its WDT: MPHD flag 0x1 (`wdt_uses_global_map_obj`) makes the map one WMO from the WDT MODF (`asset::wdt::parse_wdt_global_wmo`), held in `AdtManager.global_wmo` (Pending → Spawned/Failed by `spawn_pending_global_wmo`). Such a map streams no tiles. Retail places the global WMO from the world origin, not the map corner an ADT MODF is offset from: the Stockade WDT 791060 places WMO 108631 at raw (0, 0, 0) and its entrance point (56.68, 0.62, -19.27) lies inside the WMO bounds only under `global_wmo_placement_position` (`(-raw.z, raw.y, raw.x)` in Bevy, the ADT rule without the 17066.67 offset). `TerrainHeightmap::set_wmo_only` makes WMO floors the only ground, and the follow camera skips its terrain-floor clamp (the `GROUND_Y` 0 fallback lifted the camera out of the Stockade at floor y -19.3, where portal culling hid the whole WMO).

## Authored Height Grid Axes

MCVT rows advance toward negative Bevy X; columns advance toward positive Bevy Z. Mesh positions and the four-triangle CPU sampler use that same basis, with upward triangle winding. The former transposed basis put part of Northshire's `stormwindgypsywagon01.m2` (FDID198288, placement69265) inside an incorrectly reconstructed hill. Incorrect border averaging was removed because it combined unrelated edge samples and altered authored heights. Asymmetric four-chunk regressions cover positions, height preservation, shared borders, winding, and center-vertex sampling. Water-layer offsets, mesh winding, and water-height queries use the same corrected grid axes. A September 9, 2026 local capture shows the covered wagon clear of the hill. Its authored MDDF coordinates were not patched, but runtime Y changed from about83.8 to81.8 through the existing terrain-origin clamp sampling corrected geometry.

## Shared Material Animation Clock

Terrain layer UV animation reads Bevy's shared virtual shader clock (`globals.time`) in `assets/shaders/terrain.wgsl`; the per-layer velocities remain material data. Advancing only the clock no longer modifies terrain material assets. `config.w` remains unused padding to preserve the existing terrain uniform layout. The clock wraps after one hour, so animated UV phase restarts then; the retired CPU material time was unwrapped. A bounded Vulkan fixture rendered actual terrain and water shaders at shared times 0 and 1, observed changing central pixels for both, and saw no material `Modified` events in 0.71 seconds. This proves shader behavior only, not full-client visual equivalence or a CPU improvement. See [shared material clock spec](../../specs/shared-material-clock.md).

## Doodad Collision

Doodad solidity uses authored M2 collision triangles, not render/visual bounds. The M2 parser reads collision bounds, u16 triangle indices, and vertices; placements share the parsed geometry. A world-space AABB narrows candidates, then a backface-inclusive triangle ray test decides a hit through the placement affine transform. Starting inside the broadphase box is not a collision by itself.

Models with no authored collision indices create no solid doodad collider. There is no visual-bounds fallback. `DoodadVisualBounds` independently retains the visual extent for zone-transition/contact interactions, so non-solid doodads can still publish those bounds.

Terrain and WMO collision behavior is unchanged. [WoWee collision notes](../wowee-collision.md) remain reference material for broader collision design.

## Known Issues

- **Mountain ridge topology**: the earlier slab-like silhouettes and discontinuous chunk edges were observed before the height-grid axis correction above. Adventurer's Rest requires visual revalidation; no mountain-specific completion claim is made.
- **Terrain normals and campsite floor**: `510b44a5` corrects MCNR decoding from `[b2, b1, -b0]` to `[b0, b2, -b1]`. The verified `2703_31_37.adt` geometric alignment is `0.997198` for the corrected mapping versus `0.089730` before it. Parser RED/GREEN is recorded. `a20f6b84` removes the separate character-select `StandardMaterial` grass overlay; its regression confirms ADT terrain and a height at the campsite focus remain while no character-select `StandardMaterial` floor exists. Cross-map and rendered regression proof remains pending. See [character-select ground patch](../investigations/charselect-ground-patch-dark-terrain.md).

## Sources

- [terrain surface selection](../../../src/rendering/terrain/terrain_surface_data.rs) — shared effect, texture, and surface decisions
- [terrain heightmap](../../../src/rendering/terrain/terrain_heightmap.rs) — Bevy GroundEffect adapter and FDID path lookup

- [adventurers-rest-mountain-brief.md](../adventurers-rest-mountain-brief.md) — tile ordering bug, mountain silhouette issues
- [character-select ground patch](../investigations/charselect-ground-patch-dark-terrain.md) — verified MCNR normal-axis failure and bright workaround plane
- [world-object-rotation-investigation-2026-03-22.md](../world-object-rotation-investigation-2026-03-22.md) — placement rotation formula derivation
- [wowee-collision.md](../wowee-collision.md) — broader collision reference
- `../../data/diagnostics/cpu-goal-resumed/doodad-authored-collision/report.md` — authored doodad collision implementation and scoped proof
- AGENTS.md — ADT split files section
- `../../data/diagnostics/npc-motion-20260909/{streaming-fixed-terrain,cold-edge-run}.txt` — bounded native neighborhood and cold extraction evidence
- `/tmp/claude/swimming-lateral-runtime-00a9871e.log` and `/tmp/claude/swim-lateral-native-parser-full.log` — lateral-swimming runtime and authored-depth evidence

## See Also

- [[rendering-pipeline]] — terrain shader, ADT rendering
- [[asset-pipeline]] — CASC extraction for ADT and companion files
- [[character-rendering]] — character models spawned from ADT doodad placement
- [[collision-system]] — broader collision design and remaining layers
- [[npc-motion-validation]] — bounded stream, WMO-basis, and grounded-walk evidence
