# Procedural Cloud Regeneration

Before commit `b2b07e5b`, in-world sky updates synchronously regenerated one procedural cloud texture every five seconds. The work consumed most sampled CPU during regeneration even though the shader already animates cloud UVs over time.

## Finding

`src/rendering/skybox/cloud_texture.rs` defines 512×1024 cloud images with six simplex-noise octaves. Each image contains 524,288 pixels; each pixel evaluates two six-octave fractal-noise fields. `create_procedural_cloud_maps()` generates three textures at startup.

Before `b2b07e5b`, `update_procedural_cloud_maps()` regenerated the next texture synchronously from `Update` while `InWorld` or `CharSelect` was active. The profiler's self samples placed approximately 60% of sampled CPU in `simplex2`, `simplex_corner`, `fbm_simplex`, `gradient`, and related cloud functions.

## Resolution

Commit `b2b07e5b` removed runtime regeneration and its `Update` registration. The three startup textures, texture dimensions, six octaves, cloud parameters, and visual settings remain unchanged. `assets/shaders/sky.wgsl` continues to animate clouds by offsetting sampled UVs with time-derived cloud parameters, so runtime pixel regeneration was not required for cloud motion.

## Performance Evidence

Clean user-provided in-world baselines:

- **Focused:** 27.98 FPS
- **Unfocused:** 27.89 FPS

These are pre/post-comparison baselines only. No post-fix FPS improvement is claimed yet.

IPC screenshots are visual captures, not frame-timing measurements. They can display a transient `FPS: 1.00` overlay even though a separate unfocused capture remains at 27.89 FPS, so the `1.00` value is a capture-frame artifact rather than normal unfocused behavior. The exact diagnostics-sampling cause is not yet isolated; code and scene inspection show no Winit-setting change or persistent screenshot entity. `game-engine-cli screenshot` writes the captured frame as WebP.

## Sources

- [rendering-pipeline](../systems/rendering-pipeline.md) — pipeline summary and known performance history
- [cloud_texture.rs](../../src/rendering/skybox/cloud_texture.rs) — texture dimensions, startup generation, and simplex implementation
- [skybox/mod.rs](../../src/rendering/skybox/mod.rs) — runtime regeneration removal and sky update registration
- [sky.wgsl](../../assets/shaders/sky.wgsl) — time/cloud-parameter UV scrolling
- [game-engine-cli command dispatch](../../src/bin/game-engine-cli/command_dispatch.rs) — IPC screenshot capture behavior

## See Also

- [[rendering-pipeline]] — skybox and known rendering bottlenecks
- [[skybox]] — authored and procedural sky composition
