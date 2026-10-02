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

## Remaining leads

- Draw-call volume: doodads ~4.6k, terrain ~1.9k, units ~1.1k of 7.8k draws (Stormwind, hiding each root).
  Instancing identical static doodads or merging terrain chunks would cut it.
- Skinned M2 culling bounds: WebWowViewerCpp `M2Object::createAABB` culls by the header `bounding_box`; Godot
  recomputes each posed skinned mesh's AABB from its bones every frame (`custom_aabb` would match the reference).
