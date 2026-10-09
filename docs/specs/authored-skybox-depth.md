# Authored Skybox Depth

> Root `src/` paths below name files deleted with the [retired Bevy client](godot-conversion.md#retired-bevy-client-user-decision-2026-10-02).

Authored M2 skyboxes are backgrounds, regardless of their mesh size or position relative to scene objects. Source: `src/rendering/skybox/skybox_m2_material.rs` and `assets/shaders/m2_skybox.wgsl`. See [skybox rendering](../wiki/systems/skybox.md).

## What it must do

- [x] Scene geometry must occlude the skybox even when the authored sky mesh is physically closer to the camera.
- [x] Unobstructed pixels and transparent foliage cutouts must retain the authored sky color.
- [x] Opaque and blended sky materials must preserve foreground color and coverage.
- [x] Transparent scene objects that do not write depth render over the sky; preserve authored ordering between sky layers.
- [x] Depth correction must not change texture combining, opacity, or sky-layer ordering.

## Native Zephras material correction

Map2991's authored sky7345733 must preserve its M2 shader0x8012 dual crossfade between the three texture stages using animated weights1/2 at the held day fraction. Do not multiply different day-phase skies together. Scope excludes other maps/materials and changes no background-depth or layer ordering. Actual-texture GPU RED/GREEN exists; rebuilt consumer/private after proof remains pending. [Cause and proof](../wiki/investigations/zephras-sky-minimap.md).

## How it works

- [Skybox rendering](../wiki/systems/skybox.md)

## Implementation inventory

- `assets/shaders/m2_skybox.wgsl` — authored sky color and background-depth output.
- `src/rendering/skybox/skybox_m2_material.rs` — material pipeline, blending, and the sky-background sort band added by `b34df4d5`.

## Tests asserting this spec

- `src/rendering/skybox/skybox_depth_gpu_tests.rs` — rendered foreground and foliage-cutout pixels against physically nearer opaque/blended sky geometry.
- `src/rendering/skybox/skybox_m2_material_tests.rs` — material/blending and authored-background-before-transparent-foreground ordering.
- `data/diagnostics/waterfall-missing-20260911/ordering-verification/` — RED/GREEN ordering evidence.

## Known gaps (current cycle)

No open depth-ordering gap. GPU and native acceptance passed. Gray fogged terrain is explained in the linked wiki; its appearance is outside this correction.

## Out of scope

- Fog-distance/color changes or terrain visibility redesign; these are separate scene-presentation decisions.
