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

## Remaining leads

- Draw-call volume: doodads ~4.6k, terrain ~1.9k, units ~1.1k of 7.8k draws (Stormwind, hiding each root).
  Instancing identical static doodads or merging terrain chunks would cut it.
- Skinned M2 culling bounds: WebWowViewerCpp `M2Object::createAABB` culls by the header `bounding_box`; Godot
  recomputes each posed skinned mesh's AABB from its bones every frame (`custom_aabb` would match the reference).
