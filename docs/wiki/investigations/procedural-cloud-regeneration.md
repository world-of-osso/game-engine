# Procedural Cloud Regeneration

Before commit `b2b07e5b`, in-world sky updates synchronously regenerated one procedural cloud texture every five seconds. The work consumed most sampled CPU during regeneration even though the shader already animates cloud UVs over time.

## Finding

`src/rendering/skybox/cloud_texture.rs` defines 512×1024 cloud images with six simplex-noise octaves. Each image contains 524,288 pixels; each pixel evaluates two six-octave fractal-noise fields. `create_procedural_cloud_maps()` generates three textures at startup.

Before `b2b07e5b`, `update_procedural_cloud_maps()` regenerated the next texture synchronously from `Update` while `InWorld` or `CharSelect` was active. The profiler's self samples placed approximately 60% of sampled CPU in `simplex2`, `simplex_corner`, `fbm_simplex`, `gradient`, and related cloud functions.

## Resolution

Commit `b2b07e5b` removed runtime regeneration and its `Update` registration. The three startup textures, texture dimensions, six octaves, cloud parameters, and visual settings remain unchanged. `assets/shaders/sky.wgsl` continues to animate clouds by offsetting sampled UVs with time-derived cloud parameters, so runtime pixel regeneration was not required for cloud motion.

## Final Post-Fix Performance Evidence

Clean user-provided in-world baselines:

- **Focused:** 27.98 FPS
- **Visible-unfocused:** 27.89 FPS

The dedicated read-only `game-engine-cli performance` command was invoked exactly once post-fix while the window was fully visible. It returned:

```text
fps=28.14 frame_time_ms=35.54 focused=false
```

Compared with the matching visible-unfocused baseline, the delta is **+0.25 FPS**, which is not a meaningful FPS improvement. No screenshot was used.

A five-second read-only sample recorded:

- Process CPU: **214.96% of one core**
- Process AMD gfx-engine busy: **53.28%**
- System GPU busy samples: **[67, 58, 61, 64, 63, 59]**
- Average system GPU busy: **62.00%**

A ten-second `perf record` collected **1K samples** with **zero lost samples**. It contained no `simplex`, `fbm`, `cloud_density`, or `generate_cloud` rows. The prior approximately 60% procedural simplex hotspot is removed, but frame rate is effectively unchanged; the cloud fix must not be credited with raising FPS.

The engine journal separately recorded **5,229** repeated `bevy_pbr::ssao` errors from 18:46:15 through 19:14:41 local time: SSAO requires `Msaa::Off`, while the engine uses `Msaa::Sample4`. This confirms a live render-configuration error and substantial log spam. Its contribution to the unchanged frame rate is not established, and it was not changed in this work.

IPC/network evidence remained healthy: `pong`; `InWorld`; `connected=true`; `connected_links=1`; `remote_entities=78`; `local_players=1`. The entity-tree dump succeeded with **15,431 lines**. The same journal window contained only the expected initial connecting-state disconnect and reconnect, with no reconnect loop, GPU out-of-memory error, or general out-of-memory error.

IPC screenshots are visual captures, not frame-timing measurements. The screenshot `FPS: 1.00` artifact remains not fully isolated; no stronger cause is established here.

## Sources

- [rendering-pipeline](../systems/rendering-pipeline.md) — pipeline summary and known performance history
- [cloud_texture.rs](../../src/rendering/skybox/cloud_texture.rs) — texture dimensions, startup generation, and simplex implementation
- [skybox/mod.rs](../../src/rendering/skybox/mod.rs) — runtime regeneration removal and sky update registration
- [sky.wgsl](../../assets/shaders/sky.wgsl) — time/cloud-parameter UV scrolling
- [game-engine-cli command dispatch](../../src/bin/game-engine-cli/command_dispatch.rs) — IPC screenshot capture behavior

## See Also

- [[rendering-pipeline]] — skybox and known rendering bottlenecks
- [[skybox]] — authored and procedural sky composition
