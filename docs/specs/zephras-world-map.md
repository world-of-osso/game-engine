# Zephras world map

WoW Forever map 2991 must load through the Godot world's terrain and object loaders, without invented listfile paths. Sources: `godot/core/src/{map_catalog.rs,asset/wdt.rs}`, `godot/rust/src/terrain/assets.rs`, `scripts/import_forever_zephras.py`. [Terrain wiki](../wiki/systems/terrain.md#forever-zephras-map-2991) describes cache paths and coordinate conventions.

## What it must do

### Map and terrain

- [x] Resolve directory `2991` to map ID 2991 and WDT FileDataID 7198644 from the Forever build-70205 Map export; merge only identities absent in retail, with retail winning conflicts. Missing or malformed required exports fail explicitly.
- [x] Decode real Zephras and retail WDT MAIN/MAID bytes. MAID addresses active root/tex0/obj0 files by FDID; maps without MAID retain named companion lookup. Declared nonzero files must not silently switch to another source when unavailable.
- [x] Parse the real Zephras root/tex0/obj0 sample: terrain, water, flight bounds, vertex colours, textures and FDID/legacy placements.
- [x] Use only a declared map-level WDL FDID for a horizon. Zephras has none in available data, so has no WDL horizon; per-tile MAID LOD/maptexture slots are not a WDL.

### Provisioning and runtime

- [x] Walk deduplicated active terrain, M2 skin/texture/animation/skeleton and WMO group/material/doodad references. Resolve legacy paths through the local listfile. Preserve primary-skin and skeleton model aliases; validate magic before publishing.
- [ ] Provision the whole reachable closure from local `wow_classic_beta` CASC into the exact FDID cache destinations Godot consumes; record file counts, bytes and every failure. Never use CDN.
- [x] An offline Godot fixture loads a tile-center position, observes terrain/doodad/WMO nodes, and captures rendered pixels. Asset failures must remain reported, not hidden by fixture success.

### Lighting

- [ ] Import Light, LightData, LightParams, LightSkybox, ZoneLight and ZoneLightPoint from build 1.60.1.70205 using hash-matched DBD layouts; preserve retail CSV headers and report encrypted drops and local archive failures.
- [x] Add authored sky models reachable from Forever-only maps' LightParams slots to the importer's recursive skin/texture asset closure.
- [ ] Select Forever lighting only for maps absent from retail, without allowing colliding LightParams IDs to modify retail samples. Missing Forever inputs must fail explicitly rather than sample retail defaults.
- [ ] Sample actual Zephras position/time through native production lighting and capture inspected rendered pixels without fixture-neutral lighting.

### Liquids

- [x] Import LiquidType, LiquidMaterial, LiquidObject and LiquidTypeXTexture using build-70205 hash-matched layouts, exact Retail headers and CASC provenance.
- [x] Select a separate Forever liquid catalog only for maps absent by ID and directory in Retail. Colliding rows must not alter Retail liquid materials. Missing Forever tables, objects, types, materials or sampled textures fail explicitly.
- [x] Extend local-CASC closure with textures referenced by authored MH2O instances and their LiquidObject type mappings.
- [ ] Zephras native world fixture reports zero missing-liquid-type diagnostics and captures inspected coastal water.

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
- `godot/tests/zephras_world.gd`: actual native stream, terrain meshes, authored objects and production lighting; intended screenshot `data/diagnostics/zephras-world-production-lighting.png`. Capture is blocked by unavailable Forever lighting/sky assets. Earlier neutral-lighting proof remains `data/diagnostics/zephras-world.png`, not production-lighting acceptance.

## Known gaps (current cycle)

- [ ] Local installation lacks archive content for part of the closure; see the [dated inventory and proof](../wiki/systems/terrain.md#forever-zephras-map-2991). Dependencies below unavailable models/tiles cannot be enumerated completely.
- [ ] Runtime fixture reported Texture/ObjectDB RID leaks on shutdown while object work remained queued; no clean-resource-lifetime claim.

## Out of scope

- Server spawn/gameplay, ground/LOS and playable starting-zone content: not client map-rendering work.
- obj1, per-tile LOD, maptexture/mapnormal/minimap assets: not consumed by these world terrain/object loaders.
