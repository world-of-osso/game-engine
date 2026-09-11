# Terrain

ADT terrain is loaded from split WoW map tiles. The engine renders heightmap meshes with texture layer compositing, spawns doodads and WMOs from placement data, and uses a custom rotation formula to convert WoW-space placement angles into Bevy-space transforms.

## Local Streaming Cache

Official tile requests resolve their listfile FDID and call the existing `AssetResolver::ensure_cached` at `shared_data_path("terrain/<fdid>.adt")`. Missing disk files trigger local CASC extraction rather than a permanent streaming failure. Declared `_tex0` and `_obj*` companions use the same cache mechanism; an absent optional listfile entry returns `None`, while extraction errors include the WoW path, FDID, and destination. Existing explicitly named local sidecars remain supported. Official streaming no longer prefers a potentially stale named tile over the canonical FDID cache or searches alternate checkout caches.

`terrain_tile::resolver_tests` uses temporary filesystem caches to prove root/companion extraction, byte-preserving reuse, named-sidecar behavior, and lookup/extraction errors. Bounded native cold-edge capture extracted two roots and their declared companions from local CASC; one tile fully spawned before the 30-second cap. A third tile was not observed before the cap, not classified as an extraction failure. See [[npc-motion-validation]].

## ADT Split Files

Each tile is three files:
- Root `.adt` — MCNK chunks with heightmaps and normals
- `_tex0.adt` — texture layer compositing (MDID/MHID for diffuse/height FDIDs)
- `_obj0.adt` — MDDF doodad placements and MODF WMO placements

The engine loads all three. Finding companion files uses the community listfile (path-based sibling lookup).

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
- **Terrain normals**: `510b44a5` corrects MCNR decoding from `[b2, b1, -b0]` to `[b0, b2, -b1]`. The verified `2703_31_37.adt` geometric alignment is `0.997198` for the corrected mapping versus `0.089730` before it. Parser RED/GREEN is recorded; cross-map and rendered regression proof remains pending. See [character-select ground patch](../investigations/charselect-ground-patch-dark-terrain.md).

## Sources

- [adventurers-rest-mountain-brief.md](../adventurers-rest-mountain-brief.md) — tile ordering bug, mountain silhouette issues
- [character-select ground patch](../investigations/charselect-ground-patch-dark-terrain.md) — verified MCNR normal-axis failure and bright workaround plane
- [world-object-rotation-investigation-2026-03-22.md](../world-object-rotation-investigation-2026-03-22.md) — placement rotation formula derivation
- [wowee-collision.md](../wowee-collision.md) — broader collision reference
- `../../data/diagnostics/cpu-goal-resumed/doodad-authored-collision/report.md` — authored doodad collision implementation and scoped proof
- AGENTS.md — ADT split files section
- `../../data/diagnostics/npc-motion-20260909/{streaming-fixed-terrain,cold-edge-run}.txt` — bounded native neighborhood and cold extraction evidence

## See Also

- [[rendering-pipeline]] — terrain shader, ADT rendering
- [[asset-pipeline]] — CASC extraction for ADT and companion files
- [[character-rendering]] — character models spawned from ADT doodad placement
- [[collision-system]] — broader collision design and remaining layers
- [[npc-motion-validation]] — bounded stream, WMO-basis, and grounded-walk evidence
