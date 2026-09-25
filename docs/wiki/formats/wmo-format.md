# WMO Format

WMO (World Map Object) is Blizzard's format for large static scene geometry: buildings, dungeons, caves, interiors, and set-piece exterior structures. Unlike M2 (animated characters/doodads), WMOs are static multi-group meshes with their own lighting, portal culling, and doodad sets.

## File Structure

A WMO consists of a root file and one or more group files:

| File | Purpose |
|------|---------|
| `<name>.wmo` | Root: group count, doodad sets, material list, portal planes, GFID/MODI chunks |
| `<name>_000.wmo` … `<name>_NNN.wmo` | Groups: geometry, vertex data, batch definitions, per-group lighting |

Modern WMOs carry a `GFID` chunk in the root with FDIDs for all group files, and a `MODI` chunk with FDIDs for embedded doodad M2 models. The engine currently resolves group FDIDs via a listfile path-pattern roundtrip rather than reading GFID directly (known improvement opportunity).

## World Placement

WMOs are placed in the world via MODF records in ADT `_obj0` files. Each MODF record contains position, rotation, and a reference FDID. The same rotation mapping as MDDF doodads applies: stored `[X, Y, Z]` → engine `[Z, Y-180, -X]` in YZX order.

Commit `965f6f9e` makes WMO-local vertices, bounds, portals, lights, liquids, and embedded doodad positions use `[x, z, -y]` before placement. The former `[-x, z, y]` conversion added an extra180° rotation. All seven Northshire MOHD→MODF bounds match within0.002units after correction. Northshire Abbey is FDID107074 (not bridge FDID108121); its former maximum mismatch was21.75units. The regression checks actual Abbey header/placement coordinates, not a per-building offset. This proves placement basis only: WMO floor support, the reported free-fall, and native hole/visibility verification remain open.

## Vertex Color Lighting

Retail lights every WMO the same way; the source here is WebWowViewerCpp's Retail shaders (`calcLight`, `caclWMOFragMat`, `fixColorVertexAlpha`). MOCV is light, not albedo. The engine implementation is `WmoLitMaterial` (`terrain_objects_wmo_lighting.rs`, `wmo_lighting.wgsl`). See [[wmo-retail-lighting]] for the evidence and tests.

- **Light.** `texture * (ambient + 2 * fixed MOCV + sun)`.
  - Exterior light is the scene daylight.
  - Interior light is the WMO ambient without a sun. It comes from MAVG for the active doodad set, else the first MAVD, else the MOHD ambient.
  - The fixed MOCV alpha blends interior and exterior light per vertex, in gamma space.
  - Exterior-lit groups take full exterior light. A group is exterior-lit if it has EXTERIOR (`0x08`) or EXTERIOR_LIT (`0x40`), or lacks INTERIOR (`0x2000`).
  - `F_UNLIT` materials show the texture alone.
- **Fixup.**
  - MOHD `0x08` ("lighten interiors") keeps MOCV color raw.
  - Otherwise the MOHD ambient is subtracted, unless MOHD `0x02` (skip base color) is set.
  - Transition-batch vertices become `(c - amb) * (1 - a) / 2` and keep their alpha, which cross-fades interior and exterior light.
  - Later vertices become `(c * a / 64 + c - amb) / 2`, with alpha 255 in exterior groups and 0 in interior groups.
- **Two-layer.**
  - MOMT 13 is `mix(tex2, tex1, MOCV2.a)`.
  - MOMT 6 is `mix(mix(tex1, tex2, tex2.a), tex1, MOCV2.a)`.
  - Both read MOTV2 and MOCV2 whatever the MOMT flags.
  - MOMT 21 (MapObjLod) is texture 1 alone.
- **Alpha test.** None for blend 0 (Opaque). AlphaKey (blend 1) discards texture alpha below 128/255.
- **Placement.** A WMO listed under the same MODF uniqueId by several tiles is spawned once and lives while any of those tiles is loaded.

## Parser Location

Pure parser (no Bevy dependencies): `src/asset/wmo_format/`

The WMO parser is less complete than the M2 or ADT parsers. Rendering of WMO interiors and full material handling is ongoing work.

## Sources

- AGENTS.md (`asset/wmo_format/` entry) — module structure
- [docs/world-object-rotation-investigation-2026-03-22.md](../world-object-rotation-investigation-2026-03-22.md) — MODF rotation mapping, verified against campsite WMOs
- [docs/casc-architecture.md](../casc-architecture.md) — GFID/MODI FDID chunk description
- `../../data/diagnostics/npc-motion-20260909/wmo-basis-proof.json` — seven corrected MOHD→MODF extent comparisons

## See Also

- [[stormwind-dark-render]] — unified MapObj MOCV lighting
- [[wmo-retail-lighting]] — Retail WMO lighting, two-layer, alpha test and placement dedup

- [[adt-format]] — MODF records that place WMOs in the world
- [[m2-format]] — M2 doodads embedded inside WMO doodad sets
- [[casc-format]] — FDID resolution for WMO root and group files
