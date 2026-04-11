# Bevy 0.18: Bloom + PointLight Black Screen Bug

**Date**: 2026-04-04
**Status**: Root cause identified

## Summary

The black-screen bug was misattributed earlier. The actual trigger is enabling Bevy bloom in a scene that also contains a `PointLight`. Text and skinned meshes were correlated with the original reproduction, but they are not required for the failure.

## Root Cause

`Bloom` + `PointLight` is the breaking combination. Once bloom is active, adding a point light can black out the 3D framebuffer. The earlier `Text + PointLight + SkinnedMesh` matrix described a misleading reproduction path rather than the real dependency.

## Workaround

Avoid using bloom in scenes that rely on point lights until the Bevy-side issue is understood or fixed.

## TODO

- Report upstream to Bevy
- Identify the exact render-graph or post-process interaction between bloom and point lights
- Re-enable the desired bloom/point-light combination once Bevy has a fix
