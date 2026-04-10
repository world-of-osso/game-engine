# Animation

The animation system drives bone-based M2 model animation using data from the M2 format's bone tracks. Transitions between animations always crossfade — never snap — to maintain visual continuity.

## M2 Bone Animation

M2 models store animation sequences inline in the MD20 header (legacy) or in external `.skel` files (HD models, loaded via the SKID chunk). Each sequence has per-bone translation/rotation/scale tracks. Animation data is parsed in `src/asset/m2_format/m2_anim.rs`.

HD models store 422+ sequences in the SKB1/SKS1 chunks of a `.skel` file. The parser (`load_skel_data()`) handles both inline and external paths transparently.

Bone indices in vertex data are global skeleton indices. The skin file's bone lookup table is used only to remap per-submesh local indices to global indices via `remap_bone_indices()`.

## Crossfading Rules

- Transitions must always crossfade. Never snap between poses.
- `blend_time` comes from M2 sequence data with a **minimum of 150ms** for movement transitions.
- When re-transitioning mid-blend (e.g. quick direction changes), preserve the outgoing pose's blend weight — resetting to 0 causes visible pops.

| Transition | Blend time |
|------------|------------|
| Stand ↔ Walk/Run | 150ms |
| Walk ↔ Run | 200ms |
| Any → JumpStart | 80ms |
| JumpStart → Jump | automatic (clip end) |
| Jump → JumpEnd | 80ms (on ground contact) |
| JumpEnd → Stand/Walk | 150ms |
| Walk ↔ Shuffle | 150ms |
| Walk ↔ WalkBackwards | 200ms |

## WoW Animation IDs

`ANIM_*` constants are defined in `src/rendering/model/animation.rs`. Key IDs:
- `0` — Stand (idle)
- Movement IDs cover Walk, Run, ShuffleLeft, ShuffleRight, WalkBackwards
- Jump IDs: JumpStart, Jump (loop), JumpEnd

## Generated Character Animation

For original (non-WoW) generated characters, the same crossfade logic applies but clips come from Bevy `AnimationClip` assets (glTF) rather than M2 tracks. An additive breathing layer (Chest + Spine2 + Clavicles, ~2° pitch on 3s cycle) runs on top of all base movement animations. See [character-generation.md](../character-generation.md).

Three skeleton templates share animation sets: Humanoid (~25 bones), Digitigrade (~30 bones), Quadruped (~30 bones). Bone scaling at load time allows multiple races to share the same clips via rotation-only retargeting.

## Key Files

- `src/asset/m2_format/m2_anim.rs` — bone track parsing, sequence parsing
- `src/rendering/model/animation.rs` — ANIM_* constants, crossfade state machine, apply_animation

## Sources

- AGENTS.md — Animation section, blend_time rules, ANIM_* location
- [character-generation.md](../character-generation.md) — glTF animation pipeline, template skeletons, crossfade table

## See Also

- [[character-rendering]] — HD skeleton loading, bone remapping, jaw bone hack
- [[rendering-pipeline]] — M2 mesh assembly that animation drives
