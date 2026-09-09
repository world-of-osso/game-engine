# Animation

The animation system adapts M2 bone tracks to Bevy `AnimationClip`/`AnimationGraph` playback. WoW sequence policy remains in `M2AnimPlayer`; Bevy evaluates and blends joint poses before transform propagation. Transitions always crossfade — never snap.

## M2 Bone Animation

`bevy_curves.rs` builds one Bevy clip per supported M2 sequence. A custom Bevy curve samples the existing translation/rotation/scale track semantics, so Bevy owns curve evaluation and blending instead of a model-level loop writing every joint. Curves blend raw TRS first; their commit step applies the existing local pivot correction:

```text
translation + pivot - rotation * (scale * pivot)
```

This ordering preserves pivoted mid-crossfades. `bevy_player.rs` retains prebuilt clips for every sequence but uses three stable clip nodes: current, outgoing, and snapshot. Sequence changes replace only clip-handle payloads, leaving graph topology, masks, and weights fixed. Current and outgoing nodes retain independent seek times even when they use the same sequence. Unchanged handles do not modify the graph asset. On a re-transition, `PivotEvaluator::commit` has already retained the last blended raw pose before pivot correction and billboards; the player replaces its snapshot clip with that pose, then blends it to the new sequence. It seeks paused clips from `M2AnimPlayer`; the controller remains the single WoW sequence/time clock.

Bevy runs animation in `PostUpdate` before transform propagation. This replaces custom M2 pose evaluation, but it is still part of the windowed render application—not a separate 60 Hz animation worker. Native `--screen m2debug` at `206f844f` rendered `data/models/126487.m2`: its semantic scene reported `is_displayed=true`, a retained screenshot exists, and paired entity trees record 25 changing bone positions. The `--screenshot-regression` route bypasses this custom animation plugin, so it is not animation proof.

### Supported boundary

Current runtime supports sequence-local translation, rotation, and scale tracks with linear vector interpolation and quaternion slerp, loop/next-sequence policy, crossfades, and pivots. M2 global sequences and other parsed interpolation modes are not currently part of this bone runtime; the adapter keeps that existing boundary explicit rather than fabricating support.

Camera-facing spherical billboards remain a frame-driven pass after Bevy animation and before transform propagation. Animated lights remain render-facing. Commit `c3e28125` still evaluates authored model-light tracks every update, but compares its four owned `PointLight` fields and uses conditional visibility assignment before mutating components. Constant tracks no longer emit changes; color, intensity, range, radius, and visibility changes still apply. Two focused regression tests cover both boundaries. Existing joint entities preserve attachment and skinning identity. Replicated gear and face updates retain this animated rig; a mount or race/sex identity change replaces it, and dismount restores the character model before deferred customization.

M2 models store animation sequences inline in the MD20 header (legacy) or in external `.skel` files (HD models, loaded via the SKID chunk). Each sequence has per-bone translation/rotation/scale tracks. Animation data is parsed in `src/asset/m2_format/m2_anim.rs`.

HD models store 422+ sequences in the SKB1/SKS1 chunks of a `.skel` file. The parser (`load_skel_data()`) handles both inline and external paths transparently.

Bone indices in vertex data are global skeleton indices. The skin file's bone lookup table is used only to remap per-submesh local indices to global indices via `remap_bone_indices()`.

## Crossfading Rules

- Transitions must always crossfade. Never snap between poses.
- `blend_time` comes from M2 sequence data with a **minimum of 150ms** for movement transitions.
- A re-transition starts from the last Bevy-evaluated raw pose, not the former source sequence. `aaec3864`/`a495893f` replace the historic `a6a5d917` `x0→4→16→20` jump. `5de8e843` proves zero-elapsed and repeated-interruption continuity for translation, rotation, scale, and a nonzero pivot in actual Bevy evaluation; `4d832318` makes the rotation assertion invariant to equivalent quaternion signs.

| Transition | Blend time |
|------------|------------|
| Stand ↔ Walk/Run | 150ms |
| Walk ↔ Run | 200ms |
| Any → JumpStart | 80ms |
| JumpStart → Jump | automatic (clip end) |
| Jump → JumpEnd | 80ms (on ground contact) |
| JumpEnd → Stand/Walk | 150ms |

Running landings (`JumpLandRun`, ID187) remain in the jump state machine until their clip completes, then crossfade into Run. The jump-entry guard uses the same complete animation-ID set as dispatch; excluding ID187 previously restarted JumpStart on the next landing frame. A production-system regression advances Start → Jump → unfinished/completed JumpLandRun → Run, checking that landing never restarts and the movement crossfade remains at least150ms.
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

## NPC Animation LOD

Replicated NPCs use the full animated M2 attachment path with their exact display skin FDIDs. `NpcVisualRoot` retains display scale and rotates M2's +X forward axis by -π/2 into the network's logical +Z forward axis. Its `NpcModel` child owns `M2AnimPlayer`/`M2AnimData`; joints and meshes remain under the existing grounded model root. No player marker or default torch is added. Actual sheep (display 503) and HumanMaleHD (display 3167) fixtures advance bone transforms through Bevy playback. HumanMaleHD's local skeleton contains 216 bones and 422 sequences; Stand index 0 lasts 2667 ms and has 127 varying translation/rotation tracks. No NPC replicated `MovementState` producer currently exists, so this attachment enables default idle rather than inventing movement animation input.

Replicated NPC models carry an `AnimationLod` chosen each frame in `Update` before `sync_m2_animation_players`, from the model's `GlobalTransform` distance to the `WowCamera` and whether any descendant mesh had `ViewVisibility` set in the previous frame: on screen and within 30 yd samples every frame, 30–60 yd samples every other frame (staggered by entity index), beyond 60 yd or off screen is frozen. On a skipped frame the owner's Bevy `AnimationPlayer` stays stopped, so `animate_targets` evaluates no clips and writes no joints, and the subtree is not dirtied for transform propagation. The `M2AnimPlayer` clock and crossfades still advance, so a resumed NPC shows the correct pose immediately. Only models parented to an `NpcVisualRoot` are affected; the player model, doodads, and debug scenes always sample. Spec: [npc-animation-lod](../../specs/npc-animation-lod.md).

Motivation: in Goldshire the 100 yd server interest sphere holds ~83 creatures (~6,400 bone entities); before this, all of them were sampled and propagated every frame regardless of view. See [[movement-performance]].

## Key Files

- `src/asset/m2_format/m2_anim.rs` — bone track parsing, sequence parsing
- `src/rendering/model/animation.rs` — ANIM_* constants, WoW sequence/crossfade state machine, Bevy registration
- `src/rendering/model/animation/bevy_curves.rs` — exact M2 raw-TRS Bevy curves and pivot commit
- `src/rendering/model/animation/bevy_player.rs` — target/graph binding, paused controller seeks, and deferred binding teardown
- `src/rendering/model/animation/lod.rs` — NPC distance/visibility sampling rate

Billboard targets stage the pivot-corrected Bevy result in `SphericalBillboard.pending_pose` rather than writing an intermediate `Transform`. The existing post-animation billboard system consumes it, applies valid camera-facing rotation, and conditionally writes the final transform once. Unsampled billboards retain their current scale; sampled raw poses still apply when the camera is missing or vertical-degenerate. `RawBonePose` remains the pre-pivot blend snapshot source. This avoids raw-pose→billboard rewrites falsely invalidating a settled transform (`71535eed`; focused real-Bevy RED3→GREEN8). See [CPU investigation](../investigations/movement-performance.md).

Sequence-local constant TRS tracks fold to the existing fixed raw-pose curve when every channel is non-global and has at most one timestamp and one value in the selected sequence. Missing sequences use the existing sampler defaults. Varying/global tracks retain normal sampling; targets, joints and blending remain intact. At `50454f89`, 87 scoped animation tests and fmt/check passed. Catalog construction improved in characterization; the clock-confounded native pair establishes no additional steady-state CPU gain. See [CPU investigation](../investigations/movement-performance.md#constant-curve-follow-up).

## Sources

- AGENTS.md — Animation section, blend_time rules, ANIM_* location
- `src/rendering/model/animation/bevy_curves.rs` and `bevy_player.rs` — runtime implementation and deferred teardown
- `data/diagnostics/event-driven-updates-20260907/bevy-animation/native-offline/explicit-m2debug/` — native model motion evidence
- [character-generation.md](../character-generation.md) — glTF animation pipeline, template skeletons, crossfade table

## See Also

- [[character-rendering]] — HD skeleton loading, bone remapping, jaw bone hack
- [[rendering-pipeline]] — M2 mesh assembly that animation drives
- [[networking]] — separate transport clock versus windowed animation stage
