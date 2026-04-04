# Bevy 0.18: Text + PointLight + SkinnedMesh Black Screen Bug

**Date**: 2026-04-04
**Status**: Workaround applied (Text overlay replaced with console logging)

## Summary

Spawning a Bevy `Text` UI entity in a scene that also contains both a `PointLight` and a `SkinnedMesh` causes the entire 3D framebuffer to go black in Bevy 0.18. Any two of the three work; all three together break. The FPS overlay (from `FpsOverlayPlugin`) does NOT trigger the bug — only manually spawned `Text` entities do.

## Three-Way Interaction

| SkinnedMesh | PointLight | Text | Result |
|-------------|------------|------|--------|
| yes | yes | no | **Renders** |
| yes | no | yes | **Renders** |
| no | yes | yes | **Renders** |
| yes | yes | yes | **Black screen** |

## Workaround

In the particle debug scene, emitter info is logged to console (`info!()`) instead of spawned as a `Text` overlay entity. This avoids the three-way interaction while keeping M2 point lights functional.

## TODO

- Report upstream to Bevy
- Investigate why `FpsOverlayPlugin` text doesn't trigger the bug (different text pipeline?)
- Re-enable Text overlay once fixed
