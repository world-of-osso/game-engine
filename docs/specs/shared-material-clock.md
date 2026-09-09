# Shared material animation clock

Terrain and water shaders animate UVs from Bevy’s shared shader clock instead of per-material elapsed-time writes. See [terrain](../wiki/systems/terrain.md) and [rendering pipeline](../wiki/systems/rendering-pipeline.md).

## What it must do

- [x] Terrain and water UV animation read the shared shader clock while retaining per-material speed parameters.
- [x] Clock-only progression does not modify terrain or water material assets.
- [x] Terrain environment-map synchronization remains functional; `TerrainMaterialFreezeAfter` no longer stops shader UV animation.
- [x] Remove the water time field and its CPU update system.
- [x] Retain terrain uniform layout; `config.w` is unused.
- [x] Bounded GPU rendering shows terrain and water pixels change from shared time 0 to 1 without material `Modified` events.
- [x] Document the accepted one-hour shared-clock wrap difference from former unwrapped material time.

## How it works

- [Terrain](../wiki/systems/terrain.md)
- [Rendering pipeline](../wiki/systems/rendering-pipeline.md)

## Implementation inventory

- `assets/shaders/terrain.wgsl` — terrain UV clock read.
- `assets/shaders/water.wgsl` — water UV clock read.
- `src/rendering/terrain/terrain_material.rs` — terrain material plugin.
- `src/rendering/terrain/water_material.rs` — water material definition/plugin.

## Tests asserting this spec

- `src/rendering/terrain/terrain_material_systems/tests.rs`
- `src/rendering/terrain/water_material.rs` tests
- `src/rendering/terrain/shared_material_clock_gpu_tests.rs` — ignored GPU fixture; run the built test executable under a ten-second external timeout.

## Proof

`data/diagnostics/shared-material-clock-20260909/gpu/run-assets.log` records the actual terrain and water shaders on Vulkan. The ignored fixture completed in 0.71 seconds, changed both central pixel regions from shared time 0 to 1, and observed no terrain or water material `Modified` events. This is shader-rendering proof, not full-client visual equivalence or a CPU benchmark.

The shared Bevy shader clock wraps after one hour. The retired CPU material time did not wrap, so UV phase restarts at that boundary; speed parameters and equations otherwise remain unchanged.

## Out of scope

- Changing UV equations, speed parameters, wave/foam fields, or the global clock wrap policy.
- Whole-engine CPU gain claims.
