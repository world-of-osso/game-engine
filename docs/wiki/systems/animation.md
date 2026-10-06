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

Current runtime supports sequence-local translation, rotation, and scale tracks with linear vector interpolation and quaternion slerp, weighted loop variations, crossfades, and pivots. M2 global sequences and other parsed interpolation modes are not currently part of this bone runtime; the adapter keeps that existing boundary explicit rather than fabricating support.

Camera-facing spherical billboards remain a frame-driven pass after Bevy animation and before transform propagation. Animated lights remain render-facing. Commit `c3e28125` still evaluates authored model-light tracks every update, but compares its four owned `PointLight` fields and uses conditional visibility assignment before mutating components. Constant tracks no longer emit changes; color, intensity, range, radius, and visibility changes still apply. Two focused regression tests cover both boundaries. Existing joint entities preserve attachment and skinning identity. Replicated gear and face updates retain this animated rig; a mount or race/sex identity change replaces it, and dismount restores the character model before deferred customization.

M2 models store animation sequences inline in the MD20 header (legacy) or in external `.skel` files (HD models, loaded via the SKID chunk). Each sequence has per-bone translation/rotation/scale tracks. Animation data is parsed in `src/asset/m2_format/m2_anim.rs`.

HD models store 422+ sequences in the SKB1/SKS1 chunks of a `.skel` file. The parser (`load_skel_data()`) handles both inline and external paths transparently.

Bone indices in vertex data are global skeleton indices. The skin file's bone lookup table is used only to remap per-submesh local indices to global indices via `remap_bone_indices()`.

## Weighted Loop Variations

`variation_next` links candidates with the same animation ID; it is not a temporal successor. Previously the wolf advanced Stand indices2→9→10→11 and looped the terminal rare variant indefinitely. At each looping boundary the runtime now starts at variation0 and draws from signed authored `frequency` weights. This applies to linked looping families generally, not just Stand. Unlinked single sequences wrap normally; non-looping jump/emote completion does not draw variants. Invalid links, cycles, negative weights, and multi-variant families with no weighted positive-duration candidate are explicit errors—not uniform-weight fallbacks.

A weighted zero-duration variant plays for no time. WebWowViewerCpp `animationManager.cpp:838-921` makes it current at the boundary and re-rolls on the next update because its time already reaches its duration; one roll over the positive-duration candidates is the time-exact equivalent. `pa_redbird_stand.m2` (FDID588287) Stand: variation0 3333ms weight2184, variation1 0ms weight30583, so Stand variation0 loops. Both clients share `asset::m2_format::m2_variation::VariationFamily` (`736ed0f6`); before it Godot logged `M2 variation 1 has zero duration` every frame and froze the bird, and Bevy panicked.

Wolf FDID126487 has46 inline sequences. Stand indices2/9/10/11 carry weights30445/1092/1170/60 (total32767), and all four replay ranges arezero. Exhaustive deterministic rolls reproduce those exact counts. Per-entity SplitMix64 streams with rejection sampling avoid synchronized/global RNG state; tests inject rolls directly. Large elapsed updates consume each crossed boundary and preserve remaining time and crossfade progress; a120-second update matches partitioned updates. Multi-variant catch-up requiring4096or more shortest-duration boundaries is rejected before changing playback, rather than silently discarding time. Single-clip wrapping remains constant-time.

Replay bounds are parsed and retained, but nonzero replay scheduling is not implemented or claimed. Alias behavior is unchanged; alias links are not treated as variation links. This correction does not add NPC locomotion-state generation. A bounded native capture confirms surrounding named-wolf integration only; exact weighted behavior and terminal recovery remain regression-test evidence, not a measured long-run howl distribution. Parser/runtime proof lives under `data/diagnostics/wolf-nameplate-equipment-20260909/`.

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

Running landings (`JumpLandRun`, ID187) remain in the jump state machine until their clip completes, then crossfade into Run. Commit `3f331032` makes the jump-entry guard *exclude* ID187: treating an unfinished running landing as an active jump restarted JumpStart on the next frame. A production-system regression advances Start → Jump → unfinished/completed JumpLandRun → Run, checking that landing never restarts and the movement crossfade remains at least150ms; its final focused run passed 20 tests. Native player-motion verification remains pending.
| Walk ↔ Shuffle | 150ms |
| Walk ↔ WalkBackwards | 200ms |

## WoW Animation IDs

`ANIM_*` constants are defined in `src/rendering/model/animation.rs`. Key IDs:
- `0` — Stand (idle)
- Movement IDs cover Walk, Run, ShuffleLeft, ShuffleRight, WalkBackwards
- Jump IDs: JumpStart, Jump (loop), JumpEnd

Combat and spell IDs come from `AnimationData` (WMVx `animation-names.csv`):

| Group | IDs |
|---|---|
| Attack | 16 Unarmed, 17 1H, 18 2H, 19 2HL, 87 AttackOff |
| Parry | 20 Unarmed, 21 1H, 22 2H, 23 2HL |
| Other melee | 24 ShieldBlock, 30 Dodge |
| Wound | 9 CombatWound, 10 CombatCritical |
| Ready | 25 Unarmed, 26 1H, 27 2H, 28 2HL |
| Spells | 51/52 ReadySpellDirected/Omni, 53/54 SpellCastDirected/Omni, 55 BattleRoar, 124/125 ChannelCastDirected/Omni |

The Bevy constants for these clips (`ANIM_SPELL_CAST_DIRECTED` 51, `ANIM_ATTACK_1H` 46, and so on) do not match that table and were not used.

## Native Godot combat and spell actions

`godot/rust/src/animation/action.rs` layers one action clip over locomotion.

- **Blending.** The clip fades in and out over its M2 `blend_time` (at least 150 ms). Replacing it mid-play crossfades from the pose it reached and keeps the layer weight, so there is no pop.
- **Bones.** Upper-body bones (the subtree of key bone 4 SpineLow, `AnimKitBoneSet` 1) always take the action. Lower-body bones take it only while locomotion is Stand, Ready or SwimIdle, so a swing on the run keeps the run legs.
- **Priority.** Spell kit clips outrank melee swings and hit reactions, so an auto-attack swing arriving mid-cast does not cut Battle Shout's roar.
- **Loops.** Held loops (precast, channel) persist until `stop_action`.
- **Missing clips.** A clip the model lacks follows `AnimationData.Fallback`; a chain that reaches Stand plays nothing.

`world_combat.rs` drives the layer from the server's `CombatEvent`:

- **Swing.** The attacker plays its main-hand weapon class swing. Inventory type 17 is 2H, and 2HL for polearm (6) and staff (10) subclasses; 13/21 is 1H; no weapon is unarmed (the same inventory-type rule as WoWee `resolveMeleeAnimId`).
- **Victim.** The victim reacts with Wound 9, Crit 10, Dodge 30, Parry by its weapon class, or Block 24. A miss plays nothing.
- **Stance.** While `CombatStatus` is set, a standing unit's Stand becomes its Ready stance, or that stance's fallback: training dummies lack 25, which falls back to Stand. Players and creatures both get this.

Spell kit clips come from [[spell-visuals]].

Proof:
- `animation/action_tests.rs` on HumanMale HD:
  - the fade-in and one-shot return to the exact locomotion pose
  - upper-only on the run: legs identical to Run
  - mid-play replacement continuity
  - held precast loop
  - fallbacks (117 → 87, 47 → none)
  - spell-over-swing priority
- `world_combat.rs` tests: weapon class to clip ids, stance, reactions.
- Live `godot/tests/spellcast_anim.gd` (see [[spell-visuals]]): warrior Attack1H swings and the dummy's CombatWound on a private server, recorded to `data/diagnostics/spellcast-anim-2026-09-29/`.

## Native Godot locomotion boundary

`d3593762` shares the original direction selector through `godot/core`; its two tests are RED for the missing export (`/tmp/claude/movement-animation-red-4e896a2.out`) then GREEN 2/2 (`/tmp/claude/movement-animation-green-4e896a2.out`). Land maps None/Forward/Backward/Left/Right to Stand (0), Walk (4) or Run (5) only for running Forward, WalkBackwards (13), ShuffleLeft (11), and ShuffleRight (12). Swimming ignores `running` and maps the same directions to SwimIdle (41), Swim (42), SwimBackwards (45), SwimLeft (43), and SwimRight (44). `5b0bec54` corrects the root library import and constant visibility; its root selector is not yet checked.

`2df84899` adds native authored-ID selection: it selects variation 0, preserves time, active variation, and pose for an unchanged family request, preserves the blended outgoing pose on a loop-mode change or interruption, and leaves playback unchanged for a missing ID. The missing-API RED is `/tmp/claude/native-animation-authored-id-red.log`; four targeted tests are GREEN (`/tmp/claude/native-animation-authored-id-green.log`). It exposes read-only `current_animation_id`, returning `-1` while no model is bound. Existing Death semantics remain separate: first authored Death, non-looping, final-pose hold, once per NPC life; resurrection and replacement do not rearm it.

`bca569a5`'s real-input fixture reads the selected ID and bones but does not select clips. Its Vulkan RED ends `Held W did not select authored Run 5: 0` (`/tmp/claude/native-locomotion-input-red-2df84899.log`). `de3c0835` wires original local `PlayerMovement` direction/running/swimming through the shared selector after prediction and world advance, targeting only the local body. `a9c1bfbb` corrects its status marker; the later authenticated CLI `charselect` and `inworld` fixtures both exit 0 and validate `0 → 5 → 0` with clean shutdown (`/tmp/claude/screen-cli-charselect-runtime-a14c841e.log`, `/tmp/claude/startup-inworld-runtime-668015c5.log`). `4df9b1e2` adds an idle-Space fixture, initially RED because authored JumpStart 37 was absent (`/tmp/claude/jump-input-runtime-red-4df9b1e2.log`). `8bebf2c4` implements local JumpStart 37 → Jump 38 → JumpEnd 39 → movement; `7a6ac0d5` wires it; and `dac0ab3e` makes input release independent of airborne clip duration. `f990867f` adds running-landing fixture coverage and `5a698a23` drains jumping input from the landing boundary. After `23c8f08f` fixes yaw-relative forward validation, the actual root-launcher in-world runtime exits 0 (`/tmp/claude/running-jump-runtime-23c8f08f.log`): W+Space selects 37 → 38 → 187 → 5 → 0, changes body motion, rises, resumes displacement, emits decoded forward-running jumping then nonjump input, and becomes quiet after release. Idle Space and grounded Walk/Backward/Left/Right still pass; production remains unchanged at `dac0ab3e`. `93f38c13` adds actual idle right-drag coverage but its runtime RED observes yaw delta `-0.12`, expected turn 12, and Stand 0 (`/tmp/claude/idle-turn-runtime-red-93f38c13.log`). `167ef65b`/`8ee6c5ea` select idle turns from normalized consecutive facing deltas at an inclusive 0.02-radian threshold; six targeted tests are reported GREEN. The rerun reaches unchanged left orbit and both right drags: 60 samples each, authored 11/12 with changed bones after 150 ms, then Stand 0 (`/tmp/claude/idle-turn-runtime-8ee6c5ea.log`). It exits 101 later at WMO 108238 group 38 material 52 unsupported shader 5; bounded turn assertions are reached, but the whole runtime is not PASS. Verifier649 is asynchronous. No remote protocol was added: original remote entities still lack `MovementState` and remain Stand. The selector intentionally does not infer displacement: a local W held against a wall can remain Run. `d7dae274` is test-only: it types the fixture screenshot image; native production is unchanged. The actual cached `azeroth(32,48)` runtime exits 0 (`/tmp/claude/swimming-runtime-typed-image.log`): X=-8558, dry Z522 → deep Z500 → dry Z522, real W/S/Space, 5 → 42 → 41, changed bones after 150 ms, SwimBackwards 45 (not SwimLeft 43) → WalkBackwards 13 → Stand 0. Wet Space is suppressed stationary and moving; decoded UDP orders swimming false → true → false and becomes quiet after release. Main read the rendered deep-water PNG: it shows the swimming body and terrain, but no water surface; the verifier inspected PNG metadata only. Historical compile/parse failures were fixture/test errors, not production REDs. The verifier reran native `cargo fmt --check` and `cargo check -p game-engine-network --example native_input_fixture`, both exit 0, but did not rerun the default or runtime fixture. Water rendering, lateral swimming, speed, floating physics, real-server behavior, parity, and full conversion remain unproven.

Full-fluid behavior, performance, and full conversion remain open.

## Native Godot replicated NPC locomotion

`90caca32` ports Bevy `sync_npc_motion_animation`. The native bridge's `UnitSnapshot` carries the replicated `CreatureMotion`, the server's `MOVEMENTFLAG_FORWARD`/`MOVEMENTFLAG_WALKING` state for creatures. `WorldUnits` maps it through the shared direction selector: Still → Stand 0, Walk → Walk 4 and Run → Run 5. It selects the clip on the NPC's `NpcModel/M2Animation` only when the ID changes. Snapshots repeating the same motion every 50 ms therefore restart neither the clip nor its crossfade (`max(blend_time, 150 ms)`, starting from the outgoing pose). Death keeps its pose. A replaced visual applies the motion again.

Proof:
- A UDP wire test covers a stop that changes only the motion.
- A test on Riverpaw 3886641 checks Stand → Walk → Run → Stand: crossfades are at least 150 ms, the pose is continuous, and time is not reset on repeated snapshots.
- The live fixture is `godot/tests/npc_locomotion.gd` on the dev server (`NPC_WALK_ACCOUNT`/`NPC_WALK_PASSWORD`/`NPC_WALK_CHARACTER`). It turns the character toward a wandering creature, asserts Walk 4 while it moves and changed bones over 12 frames, then Stand 0 once it stops. The run is `/tmp/claude/npcwalk-live-6.log`, exit 0.

Interpolated feet trail the server. After a stop, one 250 ms sample can still cover up to 1.2 yd while Stand plays, so the fixture tolerates one such sample. Wanderers only walk, so Run 5 has no live coverage yet. Players carry no `CreatureMotion`; other players animate from `PlayerMotion` (next section).

## Native Godot remote player locomotion

The server replicates each player's `PlayerMotion`, Retail `MovementFlags` bit values (TrinityCore `MovementInfo.h`: FORWARD, BACKWARD, STRAFE_LEFT/RIGHT, WALKING, FALLING, SWIMMING), set from its newest applied `PlayerInput` and written only on change (game-server spec `docs/specs/player-motion.md`). Retail clients animate remote units from the same flags (wow_client `unit.c` `update_animation`). `UnitSnapshot.player_motion` carries it. `world.rs` `player_motion_locomotion` turns it into the local player's `update_locomotion` arguments: the direction in the local `compute_movement_input` priority (forward, backward, left, right) through the shared `direction_to_anim_id` (run unless WALKING, swim clips 41-45 while SWIMMING), `jumping` = FALLING, running-forward for the landing choice. `WorldUnits::update_remote_locomotion` runs as its own frame step after the local player's animation and drives every other player's `M2Animation` each frame, so JumpStart 37 → Jump 38 → JumpEnd 39 / JumpLandRun 187 play out; unchanged flags neither restart a clip nor its crossfade (≥150 ms). The local player is skipped (`remote_player_locomotion`): it keeps its predicted movement. A model missing a clip is a `FrameError::Client`. Idle turn clips 11/12 are local only; the flags carry no turn state.

Proof:
- `godot/network` wire test: a strafing jump then its stop reach the host over UDP.
- `animation/remote_player_tests.rs` on HumanMale HD 1011653: flags → IDs for every direction, walk, swim and jump; only remote players follow flags; Walk 4 → Run 5 → 11 → 12 → 13 → Stand 0 with continuous ≥150 ms crossfades and no restart on repeated flags per frame; a running jump 5 → 37 → 38 → 187 → 5 → 0.
- Live, two headless clients on a private server (`godot/tests/remote_player_motion.gd`, roles mover/observer): the mover (Fbfps) runs, backpedals, strafes, walks and jumps with real keys; the observer (Fbworldmap) records Fbfps's model: `run [0, 5]`, `backpedal [0, 13]`, `strafe_left [0, 11]`, `strafe_right [0, 12]`, `walk [0, 4]` with changed bones over 12 frames each, `jump [0, 37, 38, 39]`, `run_jump [0, 5, 37, 38, 187]`, and Stand 0 after every stop (`/tmp/claude/remote-motion-observer.log`, frames `/tmp/claude/remote-motion-*.png`).

## Native locomotion interruption and real-time blending

The native controller in `godot/rust/src/animation/mod.rs` retains the original Bevy jump sequence and running-landing choice (`src/rendering/model/animation/runtime.rs` before retirement): 37 → 38 → 39/187 → movement. Directional swim selection is also the original shared policy. Native water entry now preempts every jump/landing phase; unjumped local airborne movement selects Fall 40 and returns directly to ground movement. Local idle turning uses consecutive normalized facing deltas, selecting ShuffleLeft 11 / ShuffleRight 12 at ±0.02 radians; this is the existing native policy, not newly verified Retail turn behavior.

Every destination uses its M2 `blend_time` with a 150 ms floor, including jump clips. Interrupted blends snapshot the currently evaluated base pose, preserving the outgoing mixture rather than restarting from an unblended clip. Clip playback follows movement speed; blend elapsed time remains wall time. Outgoing sequence clocks retain their own authored movement-speed pacing.

`animation/jump_tests.rs` covers concrete state sequences, half-blend weights, water interruption during each jump phase, fall/swim/turn reversal, and a 2×-paced Run with a 200 ms wall-time fade. `animation/remote_player_tests.rs` covers replicated jump and swim using the same controller. The offline HumanMale HD jump/water capture fixture emits skinned body vertices and sequence/weight records; evidence belongs in canonical `data/diagnostics/locomotion-2026-10-06/`.

Replication boundary remains unchanged: `PlayerMotion::FALLING` represents a jump, with no distinct unjumped-fall signal, and carries no idle-turn state. Remote ledge-fall and idle-turn parity cannot be established from those flags; no wire fields were added. Offline captures do not prove live two-client behavior or rendered scene parity.

## Generated Character Animation

For original (non-WoW) generated characters, the same crossfade logic applies but clips come from Bevy `AnimationClip` assets (glTF) rather than M2 tracks. An additive breathing layer (Chest + Spine2 + Clavicles, ~2° pitch on 3s cycle) runs on top of all base movement animations. See [character-generation.md](../character-generation.md).

Three skeleton templates share animation sets: Humanoid (~25 bones), Digitigrade (~30 bones), Quadruped (~30 bones). Bone scaling at load time allows multiple races to share the same clips via rotation-only retargeting.

## NPC Animation LOD

Replicated NPCs use the full animated M2 attachment path with their exact display skin FDIDs. Server commit `02a4487` loads `creature.orientation` and replicates its authored spawn heading as logical `Rotation.y = orientation + π/2`; `NpcVisualRoot` retains display scale and rotates M2's +X forward axis by -π/2. This covers authored idle orientation only, not arbitrary runtime AI heading. Its `NpcModel` child owns `M2AnimPlayer`/`M2AnimData`; joints and meshes remain under the existing grounded model root. No player marker or default torch is added. Actual sheep (display 503) and HumanMaleHD (display 3167) fixtures advance bone transforms through Bevy playback. HumanMaleHD's local skeleton contains 216 bones and 422 sequences; Stand index 0 lasts 2667 ms and has 127 varying translation/rotation tracks. `sync_npc_motion_animation` feeds the replicated `CreatureMotion` as the model's `MovementState` (Stand/Walk/Run). Bounded native evidence records 422 changed NPC bone transforms; this supports idle playback, not locomotion-state generation or universal visibility/LOD coverage. See [[npc-motion-validation]].

Replicated NPC models carry an `AnimationLod` chosen each frame in `Update` before `sync_m2_animation_players`, from the model's `GlobalTransform` distance to the `WowCamera` and whether any descendant mesh had `ViewVisibility` set in the previous frame: on screen and within 30 yd samples every frame, 30–60 yd samples every other frame (staggered by entity index), beyond 60 yd or off screen is frozen. On a skipped frame the owner's Bevy `AnimationPlayer` stays stopped, so `animate_targets` evaluates no clips and writes no joints, and the subtree is not dirtied for transform propagation. The `M2AnimPlayer` clock and crossfades still advance, so a resumed NPC shows the correct pose immediately. A new binding ignores LOD until Bevy has evaluated its graph once (Bevy threads a graph the frame after it is added): before this, NPCs frozen since spawn — off screen or beyond 60 yd when they arrived — never sampled and stood in the bind pose. A 2026-09-24 native capture showed 39 of 60 replicated NPCs with every bone translation at rest while near NPCs animated. Models parented to an `NpcVisualRoot` and models carrying the `Doodad` marker are affected; the player model and debug scenes always sample. The Godot client applies the same bands: NPCs through `World::apply_animation_lod`, whose in-view test is the doodads' CPU box-vs-frustum test on the model's batch-mesh bounds (a renderer `VisibleOnScreenNotifier3D` never reports on screen under `--headless`, which froze every NPC in headless fixtures), ADT doodads through the in-world doodad cull, which advances a doodad's bones and material animation only on its sampled frames by all the time owed since its last sample (`DeferredClock`), so a resumed doodad is at the current clock time. Spec: [npc-animation-lod](../../specs/npc-animation-lod.md).

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
- `../../data/diagnostics/npc-motion-20260909/{npc-animation-green,npc-lod-green,npc-existing-lod-green,landing-run-green-final}.txt` — current focused NPC idle/LOD and landing proof
- `/tmp/claude/movement-animation-{red,green}-4e896a2.out` and `/tmp/claude/native-animation-authored-id-{red,green}.log` — shared selector and authored-ID unit evidence
- `/tmp/claude/native-locomotion-input-red-2df84899.log` — real-input fixture boundary before local selector wiring
- [character-generation.md](../character-generation.md) — glTF animation pipeline, template skeletons, crossfade table

## See Also

- [[character-rendering]] — HD skeleton loading, bone remapping, jaw bone hack
- [[rendering-pipeline]] — M2 mesh assembly that animation drives
- [[networking]] — separate transport clock versus windowed animation stage
- [[npc-motion-validation]] — bounded native idle, landing, and terrain evidence
- [[spell-visuals]] — spell kit animations, models and missiles over this action layer
