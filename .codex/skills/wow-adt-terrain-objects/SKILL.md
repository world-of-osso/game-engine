---
name: wow-adt-terrain-objects
description: Use when working on WoW ADT terrain, `_tex0`/`_obj*` companion files, doodad or WMO spawning, MDDF/MODF placement, terrain-object LOD, WMO portal culling, or placement debugging in the game-engine repo.
---

# WoW ADT Terrain Objects

Use this skill for terrain-object work that is too detailed for `AGENTS.md`: ADT split-file parsing, doodad placement, WMO placement, terrain object LOD, and validation against known tiles.

## When to use

- User mentions ADT, `_tex0`, `_obj0`, `_obj1`, `_obj2`
- User mentions doodads, MDDF, MODF, WMOs, or WMO groups/materials/portals
- A bug involves terrain object rotation, coordinate conversion, scale, spawning radius, culling, or missing world props
- You need to inspect or modify the terrain-object pipeline rather than only the terrain mesh

## Primary code

- `src/asset/adt.rs`: root ADT parsing, MCNK/MCVT terrain mesh data
- `src/asset/adt_tex.rs`: `_tex0` texture layer parsing and compositing inputs
- `src/asset/adt_obj.rs`: `_obj*` parsing, MDDF doodads, MODF WMOs
- `src/asset/wmo.rs`: WMO root/group parsing, materials, group meshes, portals
- `src/terrain.rs`: tile loading, terrain-object integration, spawn/despawn lifecycle
- `src/terrain_objects.rs`: doodad/WMO spawning, transforms, filtering, materials
- `src/terrain_lod.rs`: object LOD swapping
- `src/culling.rs`: doodad distance culling and WMO portal/group culling

## Repo facts

- ADT terrain is not just height data here. The runtime already supports:
  - root terrain mesh
  - `_tex0` ground texture compositing
  - `_obj*` doodad and WMO loading
  - doodad LOD swaps
  - WMO distance and portal culling
- Do not describe doodads/WMOs as future work unless you verified a missing path in code.

## Format facts to remember

- ADT top-level chunks use reversed 4CC tags in this parser path, for example `REVM`, `RDHM`, `KNCM`, `TVCM`, `RNCM`
- MCNK position in root ADT is stored as `[Y, X, Z]`, not `[X, Y, Z]`
- MCVT contains 145 floats: 9x9 outer plus 8x8 inner grid heights relative to chunk `Z`
- Terrain expands in negative `X/Y` from the chunk corner in this coordinate mapping
- Split files are:
  - root `.adt`: heights/normals/basic terrain info
  - `_tex0.adt`: texture layers and alpha maps
  - `_obj0.adt`: full-detail doodads and WMOs
  - `_obj1.adt`, `_obj2.adt`: lower-detail object companions when present
- `_obj0` uses:
  - `MDDF`/`FDDM`: M2 doodad placements
  - `MODF`/`FDOM`: WMO placements
  - `MMDX`/`MMID`: doodad path table + offsets
  - `MWMO`/`MWID`: WMO path table + offsets

## Placement facts

- MDDF/MODF stored world position is treated as `[X, Z, Y]` in this repo’s parsing layer
- Doodads and WMOs do not share exactly the same transform path; check the dedicated conversion helpers before “simplifying” them
- WMO local coordinates are converted with the repo’s `wmo_local_to_bevy()` mapping; keep that aligned with placement tests
- Before changing rotation/swizzle logic, read `docs/world-object-rotation-investigation-2026-03-22.md`

## Working method

1. Identify which layer is wrong: root terrain, `_tex0`, `_obj*`, placement transform, material load, LOD, or culling.
2. Read the narrow file set first:
   - parsing bug: `src/asset/adt*.rs` or `src/asset/wmo.rs`
   - spawn/transform bug: `src/terrain_objects.rs`
   - streaming/LOD bug: `src/terrain.rs` and `src/terrain_lod.rs`
   - visibility bug: `src/culling.rs`
3. Prefer validating with an existing known asset or tile before changing math broadly.
4. If changing transforms, look for existing tests in `src/terrain_objects.rs`, `src/asset/wmo.rs`, and terrain-related test modules.
5. When possible, add or update a focused test for the exact coordinate/placement assumption being changed.

## Validation assets and docs

- `data/terrain/azeroth_32_48.adt`: baseline terrain tile
- `data/terrain/2703_31_37.adt`: useful char-select/campsite area in existing investigations
- `docs/world-object-rotation-investigation-2026-03-22.md`: placement and rotation debugging history
- `docs/adventurers-rest-mountain-brief.md`: char-select terrain/background context
- `docs/wowee-collision.md`: collision behavior notes that interact with WMO/world-object assumptions
- `doc/casc-extraction.md`: split-file extraction details and FDID relationships

## Guardrails

- Do not overwrite terrain-object transform logic in multiple places without tracing all call sites first
- Do not assume a visual bug is a shader issue before ruling out tile selection, placement math, and culling
- Do not remove `_obj1`/`_obj2` fallback behavior unless you verified the streaming path still works across tiles
- Keep repo-specific facts here; keep only short project-level summaries in `AGENTS.md`
