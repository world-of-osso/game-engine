# Godot In-World Frame Time

Steady-state in-world frame time of the Godot client (October 2, 2026, branch `frameperf`).

## Benchmark

`scripts/performance/frame-benchmark.py` runs `godot/tests/frame_benchmark.gd` headless in cage against a private
server: idle, camera orbit and real-key walk segments (20 s each) after every load queue drains. Each segment reports
p50/p95/p99 frame interval, the client's own step time, render CPU/GPU ms, draw calls and the main thread's on-CPU ms
(`/proc/thread-self/schedstat`). `--gpu-profile` adds the CPU ms of each renderer timestamp area; `--perf` samples the
main thread per segment, and `--godot` runs a Godot build with symbols so engine functions resolve
(`scons production=yes lto=none debug_symbols=yes`; perf call chains still break at Rust frames). `GAME_PROFILE_MS=0`
prints every client span with wall and on-CPU ms (`scripts/performance/frame-spans.py`).

Shipped build: the pinned editor Godot (`production=yes`) with the dev-profile extension (opt-level 2, gdext strict
safeguards).

## Findings

- The main thread is CPU-bound: on-CPU ms per frame equals the frame interval; GPU time is 12-34 ms.
- The client's own steps are 5-15 ms per frame in Stormwind and Northshire; Godot's renderer is the rest.
- **Material uniform rebuilds (fixed, `9cf5cc0b`).** `WowMaterialAnimation` resampled ~9k animated batch materials per
  frame in Stormwind and set all five inputs on each, though ~380 changed. Every `set_shader_parameter` makes Godot
  rebuild that material's uniform buffer (`MaterialStorage::update_uniform_buffer`, about 30% of the main thread).
  Setting only changed inputs:

  | scene (segment) | before p50 | after p50 |
  |---|---|---|
  | Stormwind idle | 77-110 ms | 37-52 ms |
  | Stormwind walk | 84-110 ms | 41-60 ms |
  | Northshire idle | 66-82 ms | 43-45 ms |
  | Northshire walk | 80-111 ms | 37-51 ms |

  Ranges span runs at host load 3-11; pairs were run back to back.
- **After the fix (Stormwind idle, symbolized perf):** renderer draw submission is about 60% of the main thread
  (7.8k draw calls: `_render_list`, `RenderingDeviceGraph`, pipeline lookups), scene update about 17%
  (skinned-mesh AABBs from skeleton bones 4%, BVH moves 4%, remaining material updates 7%), Rust about 10%.
- Skeletons already run in `MANUAL` modifier mode, so the 17k `Skeleton3D` nodes cost nothing per frame unposed.
- World lighting rebinds every model's light parameters only when the sampled light changes; in the benchmark that
  is rare (0.2 ms/frame mean), so it is not a steady-state cost.

- **Shadow and depth passes (branch `shadowpass`).** The benchmark now reports `shadow_draw_calls`
  (`Viewport.RENDER_INFO_TYPE_SHADOW`; the depth pre-pass is not counted separately). Stormwind idle: 2.46k of
  7.84k draws are shadow-cascade draws, Northshire 1.12k of 5.29k; the rest is the colour pass (whose depth
  pre-pass resubmits every opaque draw). Three changes, each matching Retail where Retail defines it:
  1. Interior WMO groups and doodads referenced only by interior groups cast no directional shadow: Retail
     collects only EXTERIOR/EXTERIOR_LIT (MOGP `0x48`) groups and their MODR doodads (solarityclient
     `terrain_frame/world_model/shadow.rs` `prepare_shadow_draws`). Scenery shadows already stop at the
     fade-start radius (`SceneryDistance::admits_shadow`). Retail's per-cascade minimum unit radius (0.25/2/10,
     `terrain_frame/shadow.rs` `model_maps`) has no Godot equivalent (no per-cascade caster mask). The Godot
     client draws no ground clutter, so there is nothing to exclude there.
  2. Two directional cascades (`PARALLEL_2_SPLITS`): the retail client writes `shadowNumCascades 2` for shadow
     quality Medium (`_retail_/WTF/Config.wtf`, `graphicsShadowQuality 2`; `Wow.exe`: "Number of shadow
     cascades (1-4)"). Retail split distances and maximum shadow distance were not found locally; Godot's
     defaults (100 yd, split 0.1) are kept.
  3. `rendering/driver/depth_prepass/enable=false`. Godot ties SSAO/SSIL/SDFGI to the pre-pass; the client
     enables none of them (the `ssao_enabled` option is not applied). Water's depth texture and TAA motion
     vectors come from the colour pass.

  Medians of 4 interleaved rounds (`data/diagnostics/shadowpass/r{1..4}-*`, host load 5-14; each variant
  includes the ones above it):

  | scene (idle) | variant | p50 | p95 | render CPU | GPU | draws | shadow draws |
  |---|---|---|---|---|---|---|---|
  | Stormwind | base | 45.9 | 61.6 | 24.1 | 17.8 | 7840 | 2462 |
  | Stormwind | + interior WMO casters off | 43.6 | 54.3 | 22.5 | 17.4 | 7383 | 2027 |
  | Stormwind | + 2 cascades | 40.9 | 53.5 | 21.7 | 17.2 | 6807 | 1426 |
  | Stormwind | + no depth pre-pass | 34.9 | 42.1 | 18.3 | 18.0 | 6746 | 1396 |
  | Northshire | base | 32.2 | 40.4 | 18.0 | 19.6 | 5285 | 1123 |
  | Northshire | + 2 cascades | 31.0 | 43.6 | 18.1 | 18.5 | 5091 | 883 |
  | Northshire | + no depth pre-pass | 28.8 | 37.2 | 14.9 | 14.0 | 5105 | 888 |

  Stormwind walk p50 47.3 -> 40.3 ms, Northshire walk 38.6 -> 31.8 ms. Northshire has few interior WMOs; its
  draw count varies by ~250 with NPC positions (a run-order swap moved the difference between variants), so
  its interior-caster row is omitted. Fixed-camera screenshots (`*.png`, diffs `diff-*.png`): the pre-pass
  change differs from the same build with the pre-pass by 0.2% of pixels, less than two runs of one build
  (0.4-1.2%, moving NPCs); two cascades soften distant leaf shadows (1-3% of pixels). `m2_real_pixels` and
  `character_real_pixels` pass.

- **Where the renderer CPU goes (branch `godotprof`).** DWARF call-graph perf (`cycles:u`, 199 Hz, symbolized
  Godot) of the Stormwind idle segment after `shadowpass`, 42.5 ms main-thread CPU per frame (inclusive ms/frame):

  | function | ms | cause in our scene |
  |---|---|---|
  | `RenderingServerDefault::_draw` | 26.7 | whole renderer |
  | `RenderForwardClustered::_render_list` | 7.3 | per draw: 6.9k colour + shadow draws |
  | `RenderingDeviceGraph::end` (command replay) | 6.5 | per draw; plus `libvulkan_radeon` 5.2 ms self |
  | `RendererSceneCull::update` | 6.6 | dirty instances + material queue |
  | `update_dirty_instances` | 4.3 | instances re-dirtied every frame |
  | `_fill_render_list` | 2.5 | per visible surface |
  | `MeshStorage::mesh_get_aabb` | 2.1 | skinned batch meshes re-derive their AABB from bones after each pose write |
  | `_update_queued_materials` | 2.0 | animated M2 material inputs (changed only) |
  | `_update_dirty_geometry_instances` | 1.7 | water: see below |
  | `_update_dirty_geometry_pipelines` | 0.9 | water: see below |

  Self time by object: Godot 62%, the extension 19% (8 ms), `libvulkan_radeon` 12% (5 ms). Draw submission
  (`_render_list` + graph replay + driver) is about 19 ms for 6.9k draws, about 2.8 us per draw, so it is
  proportional to draws; the 8% and 12% draw cuts (`doodadinst-unmerged-ref`, `terrainmerge-unmerged-ref`)
  predict 1.5-2.5 ms, below the run-to-run spread of p50 (35-46 ms).
- **Liquid texture rewrites (fixed, `godotprof`).** `LiquidSurface::set_time` wrote every texture slot each
  frame. In Godot a texture parameter write marks the material's textures dirty
  (`MaterialStorage::material_set_param`), so `update_parameters_uniform_set` rebuilds the uniform set and
  `_update_queued_materials` sends `DEPENDENCY_CHANGED_MATERIAL`, which re-runs `_geometry_instance_update`
  and pipeline lookup for every water chunk instance using that material. The flipbook advances one frame
  per second (`LiquidMaterialManager.cpp` `updateLiquidDataAnimatedTextures`), so a slot is now written only
  when its frame changes. Stormwind idle, same profile: geometry instance updates 1.67 -> 0.17 ms, pipeline
  updates 0.89 -> 0.11 ms, `_fill_render_list` 2.52 -> 1.38 ms, `_draw` 26.7 -> 24.9 ms.
  Stormwind idle without perf, interleaved (`data/diagnostics/godotprof/r{2,3}-*`, load 4.7-5.2, same draws):
  base p50 40.7 / 39.3 ms, render CPU 21.4 / 20.3; fix p50 35.2 / 35.1 ms, render CPU 16.9 / 16.7. A first
  round under 997 Hz perf (`r1-*`) showed no difference (38.0 vs 38.6), so the size of the gain is not settled.

## Remaining leads

- Draw-call volume: doodads ~4.6k, terrain ~1.9k, units ~1.1k of 7.8k draws (Stormwind, hiding each root).
  Instancing identical static doodads or merging terrain chunks would cut it.
- Skinned M2 culling bounds: WebWowViewerCpp `M2Object::createAABB` culls by the header `bounding_box`; Godot
  recomputes each posed skinned batch mesh's AABB from its bones every frame (`mesh_get_aabb`, 2.1 ms in
  Stormwind idle; `custom_aabb` would match the reference).
- `Node3D::_propagate_transform_changed` 1.4 ms self, reached from extension `set_quaternion`/`set_position`
  calls during idle (callers lost at Rust frames).
