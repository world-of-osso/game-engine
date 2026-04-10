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

## Parser Location

Pure parser (no Bevy dependencies): `src/asset/wmo_format/`

The WMO parser is less complete than the M2 or ADT parsers. Rendering of WMO interiors and full material handling is ongoing work.

## Sources

- AGENTS.md (`asset/wmo_format/` entry) — module structure
- [docs/world-object-rotation-investigation-2026-03-22.md](../world-object-rotation-investigation-2026-03-22.md) — MODF rotation mapping, verified against campsite WMOs
- [docs/casc-architecture.md](../casc-architecture.md) — GFID/MODI FDID chunk description

## See Also

- [[adt-format]] — MODF records that place WMOs in the world
- [[m2-format]] — M2 doodads embedded inside WMO doodad sets
- [[casc-format]] — FDID resolution for WMO root and group files
