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
| `MCNR` | 145 compressed normals (3 bytes each, stored as `x/127, z/127, y/127`) |
| `MCLY` | Up to 4 texture layer definitions (FDID reference, flags, alpha map offset) |
| `MCAL` | Alpha maps for layers 1–3 (layer 0 is fully opaque base) |

MCNK header records the chunk's world-space origin. The 33×33 grid spans one chunk: 16 inner quads × 8 units = 128 units per side, plus a shared border row/column with neighbors.

## Coordinate System

ADT coordinates are WoW world-space (Y-up, right-handed). The engine maps these to Bevy's left-handed Y-up space. Chunk origins from the MCNK header match the derived `(tile_x, tile_y)` math closely — chunk-origin parsing is not a significant source of error.

## Heightmap Topology

The 33×33 grid is a diamond-tessellated mesh: outer ring vertices (17 per row) interleave with inner detail vertices (16 per row). Discontinuities along shared chunk edges cause visible seams when adjacent chunks have large height differences — observed in mountain ridge areas.

## Terrain Normals

MCNR normals are precomputed by the WoW client/editor. In mountain areas they can be inconsistent with the geometric face normals of the reconstructed mesh, indicating either a decoding issue or a mismatch between the stored normals and the actual vertex layout.

## MDDF / MODF Placements

`_obj0` files contain placement records for M2 doodads (MDDF) and WMO world map objects (MODF).

**Rotation mapping** (stored → Bevy): ADT stores `[X, Y, Z]` Euler angles. The engine converts to `[Z, Y-180, -X]` applied in `YZX` order. This was determined empirically against the Adventurer's Rest campsite and cross-referenced with Noggit3's `from_model_rotation` transform.

FDID chunks in `_obj0`:
- `MDID` — diffuse texture FDIDs for doodads
- `MHID` — height texture FDIDs

## Warband Scene Tile Loading

For character-select scenes, the primary tile must be loaded first; sorting the tile list by tile coordinates can accidentally promote a supplemental tile (e.g. a neighbor waterfall tile) ahead of the tile that contains the actual terrain feature. The engine uses a dedicated `ensure_warband_terrain()` call rather than taking the first element of the sorted full tile list.

## Sources

- [docs/adventurers-rest-mountain-brief.md](../adventurers-rest-mountain-brief.md) — tile selection bug, MCNK peak data, normal inconsistency
- [docs/world-object-rotation-investigation-2026-03-22.md](../world-object-rotation-investigation-2026-03-22.md) — MDDF/MODF rotation mapping derivation
- AGENTS.md (ADT Terrain section + `asset/adt_format/`) — split file structure, parser modules

## See Also

- [[casc-format]] — how ADT FDIDs are resolved from CASC archives
- [[wmo-format]] — WMO objects placed via MODF records
- [[m2-format]] — M2 doodads placed via MDDF records
- [[db2-format]] — DB2 tables for light/skybox lookup driven by world position
