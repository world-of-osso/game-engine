# Bevy Bloom + PointLight Black Screen Bug

In Bevy 0.18, the black-screen issue is caused by combining bloom with a `PointLight`. The older `Text + PointLight + SkinnedMesh` explanation was a false lead from the original reproduction scene.

## Finding

The breaking combination is:

| Bloom | PointLight | Result |
|-------|------------|--------|
| no | yes | Renders |
| yes | no | Renders |
| yes | yes | Black screen |

## Root Cause

Bevy 0.18 internal bug in the interaction between the bloom post-process path and point-light rendering. Text and skinned meshes are not part of the actual trigger.

## Resolution / Workaround

Disable bloom in scenes that need point lights, or avoid point lights in bloom-enabled scenes, until the Bevy-side bug is fixed.

**TODO:**
- Report upstream to Bevy.
- Identify the exact bloom/point-light failure path in Bevy's render graph.
- Re-enable the intended bloom + point-light setup after a Bevy fix.

## Sources

- [pointlight-skinned-mesh-bug-2026-04-04.md](../../pointlight-skinned-mesh-bug-2026-04-04.md) — updated root cause and workaround

## See Also

- [[m2-rendering]] — M2 point lights and particle system (if page exists)
