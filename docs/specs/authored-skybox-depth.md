# Authored Skybox Depth

Authored M2 skyboxes are backgrounds, regardless of their mesh size or position relative to scene objects. Source: `src/rendering/skybox/skybox_m2_material.rs` and `assets/shaders/m2_skybox.wgsl`. See [skybox rendering](../wiki/systems/skybox.md).

## What it must do

- [ ] Scene geometry must occlude the skybox even when the authored sky mesh is physically closer to the camera.
- [ ] Unobstructed pixels and transparent foliage cutouts must retain the authored sky color.
- [ ] Opaque and blended sky materials must preserve foreground color and coverage.
- [ ] Depth correction must not change texture combining, opacity, or sky-layer ordering.

## How it works

- [Skybox rendering](../wiki/systems/skybox.md)

## Implementation inventory

- `assets/shaders/m2_skybox.wgsl` — authored sky color and background-depth output.
- `src/rendering/skybox/skybox_m2_material.rs` — material pipeline, blending, and ordering.

## Tests asserting this spec

- `src/rendering/skybox/skybox_depth_gpu_tests.rs` — rendered foreground and foliage-cutout pixels against physically nearer opaque/blended sky geometry.
- `src/rendering/skybox/skybox_m2_material_tests.rs` — existing material/blending behavior.

## Known gaps (current cycle)

- [ ] Run corrected GPU regression and native character-selection acceptance.
- [ ] Explain the gray backdrop in the prior far-depth diagnostic without hiding missing sky output.

## Out of scope

- Fog-distance/color changes or terrain visibility redesign; these are separate scene-presentation decisions.
