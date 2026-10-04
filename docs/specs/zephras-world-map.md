# Zephras world map

WoW Forever map 2991 must load through the Godot world's terrain and object loaders, without invented listfile paths. Sources: `godot/core/src/{map_catalog.rs,asset/wdt.rs}`, `godot/rust/src/terrain/assets.rs`, `scripts/import_forever_zephras.py`. [Terrain wiki](../wiki/systems/terrain.md#forever-zephras-map-2991) describes cache paths and coordinate conventions.

## What it must do

### Map and terrain

- [x] Resolve directory `2991` to map ID 2991 and WDT FileDataID 7198644 from the Forever build-70205 Map export; merge only identities absent in retail, with retail winning conflicts. Missing or malformed required exports fail explicitly.
- [ ] Decode real Zephras and retail WDT MAIN/MAID bytes. MAID addresses active root/tex0/obj0 files by FDID; maps without MAID retain named companion lookup. Declared nonzero files must not silently switch to another source when unavailable.
- [ ] Parse the real Zephras root/tex0/obj0 sample: terrain, water, flight bounds, vertex colours, textures and FDID/legacy placements.
- [ ] Use only a declared map-level WDL FDID for a horizon. Zephras has none in available data, so has no WDL horizon; per-tile MAID LOD/maptexture slots are not a WDL.

### Provisioning and runtime

- [x] Walk deduplicated active terrain, M2 skin/texture/animation/skeleton and WMO group/material/doodad references. Resolve legacy paths through the local listfile. Preserve primary-skin and skeleton model aliases; validate magic before publishing.
- [ ] Provision the whole reachable closure from local `wow_classic_beta` CASC into the exact FDID cache destinations Godot consumes; record file counts, bytes and every failure. Never use CDN.
- [ ] An offline Godot fixture loads a tile-center position, observes terrain/doodad/WMO nodes, and captures rendered pixels. Asset failures must remain reported, not hidden by fixture success.

## How it works

- [Terrain: Forever Zephras](../wiki/systems/terrain.md#forever-zephras-map-2991).
- [World loading readiness](world-loading.md).

## Implementation inventory

- `godot/core/src/map_catalog.rs`: explicit retail/Forever map identities.
- `godot/core/src/asset/wdt.rs`: MAIN and MAID slots.
- `godot/rust/src/terrain/assets.rs`, `account.rs`: FDID terrain acquisition and map identity consumers.
- `scripts/import_forever_zephras.py`: local-CASC recursive provisioning and inventory.
- `godot/rust/src/lib.rs` `preview_world_map`: offline entry into production stream/render paths.

## Tests asserting this spec

- `godot/core/tests/zephras_map.rs`, `zephras_terrain.rs`; WDT tests in `asset/wdt.rs`.
- `godot/rust/src/terrain/assets.rs` `zephras_reads_unnamed_wdt_and_maid_tile_from_fdid_cache`.
- `scripts/tests/test_import_forever_zephras.py`: active MAID selection, model aliases/references, WMO groups/doodads/material slots, wrong-magic rejection.
- `godot/tests/zephras_world.gd`: actual native stream, terrain meshes and authored objects; screenshot `data/diagnostics/zephras-world.png`.

## Known gaps (current cycle)

- [ ] Local installation lacks archive content for part of the closure; see the [dated inventory and proof](../wiki/systems/terrain.md#forever-zephras-map-2991). Dependencies below unavailable models/tiles cannot be enumerated completely.
- [ ] Parser, reader and runtime acceptance evidence pending.

## Out of scope

- Forever lighting/sky tables: separate work; fixture uses an explicit neutral directional/ambient environment.
- Server spawn/gameplay, ground/LOS and playable starting-zone content: not client map-rendering work.
- obj1, per-tile LOD, maptexture/mapnormal/minimap assets: not consumed by these world terrain/object loaders.
