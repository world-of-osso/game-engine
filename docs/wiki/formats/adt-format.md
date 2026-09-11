# ADT Format

ADT files are WoW's terrain tile format. Each map tile is a 533.3×533.3 unit square divided into a 16×16 grid of MCNK chunks. Modern WoW splits the tile into three companion files that must all be loaded together.

## Split Files

| File | Suffix | Content |
|------|--------|---------|
| Root | `.adt` | MCNK heightmaps, vertex normals, hole masks |
| Texture | `_tex0.adt` | Texture layers (MCLY), alpha maps (MCAL), MDID/MHID FDID chunks |
| Object | `_obj0.adt` | MDDF doodad placements, MODF WMO placements |

Companion files share a base FDID; the engine derives `_tex0` and `_obj0` FDIDs from the root FDID via the community listfile (path-based sibling lookup).

## MCNK Chunks

Each MCNK is a 33×33 vertex heightmap (inner 9×9 grid + outer ring, interleaved). Key sub-chunks:

| Sub-chunk | Content |
|-----------|---------|
| `MCVT` | 145 height values (float), relative to chunk base Z |
| `MCNR` | 145 compressed normals (3 signed bytes each) |
| `MCLY` | Up to 4 texture layer definitions (FDID reference, flags, alpha map offset) |
| `MCAL` | Alpha maps for layers 1–3 (layer 0 is fully opaque base) |

MCNK header records the chunk's world-space origin. The 33×33 grid spans one chunk: 16 inner quads × 8 units = 128 units per side, plus a shared border row/column with neighbors.

## Coordinate System

ADT coordinates are WoW world-space (Y-up, right-handed). The engine maps them to Bevy `(X, Y, -Z)`. MCNK stores world X at header offset `0x68`, world Y at `0x6c`, and base height at `0x70`.

MCVT rows advance toward negative Bevy X; columns advance toward positive Bevy Z. The renderer, client heightmap, and [server terrain sampler](../../../game-server/docs/wiki/systems/networking.md#terrain-height-sampling) use this basis. The former row/column transpose reconstructed hills incorrectly while leaving authored doodad transforms unchanged.

## Heightmap Topology

The 145-value grid is a diamond-tessellated 9×9 outer / 8×8 inner layout. Each outer cell is four triangles meeting at its authored inner center vertex. Mesh winding points upward after coordinate conversion. Shared MCNK edges are authored data and are no longer averaged by a parser seam pass; averaging had combined wrong edge samples and altered terrain heights.

## Terrain Normals

MCNR raw bytes `[b0, b1, b2]` map to Bevy `[b0, b2, -b1]`. The current production parser incorrectly emits `[b2, b1, -b0]`; do not treat it as format behavior. A height-derived permutation check over `2703_31_37.adt` aligns the supported mapping at mean dot `0.997198` versus `0.089730` for the current decode. Parser and rendered regressions must land with the correction. See [character-select ground patch](../investigations/charselect-ground-patch-dark-terrain.md).

## MDDF / MODF Placements

`_obj0` files contain placement records for M2 doodads (MDDF) and WMO world map objects (MODF).

**Rotation mapping** (stored → Bevy): ADT stores `[X, Y, Z]` Euler angles. The engine converts to `[Z, Y-180, -X]` applied in `YZX` order. This was determined empirically against the Adventurer's Rest campsite and cross-referenced with Noggit3's `from_model_rotation` transform.

FDID chunks in `_obj0`:
- `MDID` — diffuse texture FDIDs for doodads
- `MHID` — height texture FDIDs

## Warband Scene Tile Loading

For character-select scenes, the primary tile must be loaded first; sorting the tile list by tile coordinates can accidentally promote a supplemental tile (e.g. a neighbor waterfall tile) ahead of the tile that contains the actual terrain feature. The engine uses a dedicated `ensure_warband_terrain()` call rather than taking the first element of the sorted full tile list.

## Sources

- [docs/adventurers-rest-mountain-brief.md](../adventurers-rest-mountain-brief.md) — tile selection bug and mountain visual revalidation context
- `src/asset/adt_format/adt.rs`, `src/asset/adt.rs`, `src/rendering/terrain/terrain_heightmap.rs` — client parser, mesh, and sampler
- `../../../game-server/crates/server/src/terrain_height.rs` — matching server sampler
- [docs/world-object-rotation-investigation-2026-03-22.md](../world-object-rotation-investigation-2026-03-22.md) — MDDF/MODF rotation mapping derivation
- AGENTS.md (ADT Terrain section + `asset/adt_format/`) — split file structure, parser modules

## See Also

- [[casc-format]] — how ADT FDIDs are resolved from CASC archives
- [[wmo-format]] — WMO objects placed via MODF records
- [[m2-format]] — M2 doodads placed via MDDF records
- [[db2-format]] — DB2 tables for light/skybox lookup driven by world position
