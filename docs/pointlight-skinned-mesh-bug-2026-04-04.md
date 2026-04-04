# Bevy 0.18: PointLight + SkinnedMesh Black Screen Bug

**Date**: 2026-04-04
**Status**: Workaround applied (M2 point lights disabled)

## Summary

Adding a `PointLight` entity to a scene that also contains a `SkinnedMesh` causes the entire 3D framebuffer to go black in Bevy 0.18. The clear color, ground plane, and all 3D meshes become invisible. Only 2D/UI elements (text overlays, FPS counter) continue rendering.

## Affected Scenes

- **Particle debug**: Torch M2 model (skinned) + M2 point light → black
- **Character select**: Character models (skinned) + scene point lights → black 3D, UI renders
- Any scene combining `SkinnedMesh` + `PointLight`

## Unaffected Scenes

- **Skybox debug**: Skybox M2 model + `DirectionalLight` only → renders fine
- Scenes with only `DirectionalLight` + `SkinnedMesh` → fine
- Scenes with only `PointLight` + non-skinned `Mesh3d` → fine

## Reproduction

Minimal reproduction in particle debug scene:

1. Spawn a `Camera3d` + `DirectionalLight` + ground plane → renders ✓
2. Add a skinned M2 model (torch) → renders ✓
3. Add ANY `PointLight` entity (even default values, top-level, no parent) → entire 3D scene goes black ✗

The PointLight values don't matter — tested with:
- Default PointLight (intensity=1000, range=5)
- M2-authored values (intensity=1027, range=2.2)
- Zero radius, different parents, different positions

All produce the same black screen.

## Root Cause

Likely a Bevy 0.18 rendering pipeline bug where the point light clustering/shadow pass interacts badly with the skinned mesh GPU skinning pass, producing NaN or corrupted values in the framebuffer that the tonemapping shader (TonyMcMapface) propagates to solid black.

## Workaround

`spawn_model_point_lights()` in `src/rendering/model/m2_spawn.rs` is disabled (function body is empty). M2 models that define point lights will not have them rendered.

## TODO

- Report upstream to Bevy if not already known
- Re-enable `spawn_model_point_lights()` when Bevy fixes the PointLight + SkinnedMesh interaction
- Test with Bevy 0.19 when available
