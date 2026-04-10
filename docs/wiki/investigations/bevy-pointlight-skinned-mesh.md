# Bevy PointLight + SkinnedMesh Black Screen Bug

In Bevy 0.18, spawning a `Text` UI entity in a scene that also contains both a `PointLight` and a `SkinnedMesh` causes the entire 3D framebuffer to go black. Any two of the three components work; all three together trigger the bug.

## Finding

Interaction matrix:

| SkinnedMesh | PointLight | Text | Result |
|-------------|------------|------|--------|
| yes | yes | no | Renders |
| yes | no | yes | Renders |
| no | yes | yes | Renders |
| yes | yes | yes | Black screen |

The `FpsOverlayPlugin` text entity does **not** trigger the bug, suggesting it goes through a different text rendering pipeline than manually spawned `Text` entities.

## Root Cause

Bevy 0.18 internal bug — exact interaction between the shadow/lighting pass for `PointLight`, the skinning compute pass for `SkinnedMesh`, and the UI text render graph node. Not yet diagnosed at the Bevy level.

## Resolution / Workaround

In the particle debug scene, emitter info is logged to console via `info!()` instead of spawning a `Text` overlay entity. M2 point lights remain functional.

**TODO:**
- Report upstream to Bevy.
- Investigate why `FpsOverlayPlugin` text avoids the bug.
- Re-enable `Text` overlay after a Bevy fix.

## Sources

- [pointlight-skinned-mesh-bug-2026-04-04.md](../../pointlight-skinned-mesh-bug-2026-04-04.md) — reproduction matrix and workaround

## See Also

- [[m2-rendering]] — M2 point lights and particle system (if page exists)
