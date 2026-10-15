# M2 animation sampling

[Scope, revisions and evidence levels](README.md). `W` = pinned wowdecomp; `E` = pinned game-engine. These are source-demonstrated conditional mismatches, not asset-backed visual reproductions. No blanket claim that M2 animations, attachments or skinning are absent.

## A1: Step tracks are interpolated

**Reference.** `W/Engine/Source/Model2/M2Model.cpp:3343–3362` (`M2AnimateTrack<C3Vector,C3Vector>`, original address `0x0082B0A0`) explicitly selects the current key when `!track.m_trackType`. The float specialization at `:3433–3454` has the same distinction. Compressed quaternion sampling at `:2415–2445` also distinguishes step from interpolated keys.

**Current.** `E/godot/core/src/asset/m2_format/m2_anim.rs:577–593,604–621,662–672` interpolates vec3, rotation and float tracks between adjacent keys without consulting `interpolation_type`. This is reachable for native bones through `E/godot/rust/src/animation/mod.rs:323–340`; ribbon color/width sampling calls the same generic evaluators at `E/godot/core/src/m2_ribbons.rs:45–55`.

**Counterexample.** A type-0 translation track has keys `(0 ms,[0,0,0])` and `(1000 ms,[10,0,0])`. At 500 ms the reference holds `[0,0,0]`; the generic engine evaluator returns `[5,0,0]`. This is unintended motion for that input, independent of coordinate permutation.

**Required change.** Honor the authored hold-versus-interpolate mode in each affected generic evaluator. Keep continuous playback for authored interpolated tracks and inter-animation crossfades; a step within a track is not an instruction to snap animation transitions. Do not replace material sampling, which already has separate interpolation-aware handling (`E/godot/core/src/m2_material.rs:299–326`).

**Acceptance.** At 0, 499, 999 and 1000 ms, the type-0 fixture holds the first key until 1000, then switches. The equivalent type-1 fixture yields 4.99 and 9.99 between keys. Exercise a step quaternion and float width too; finally inspect native bone/ribbon output from a hashed relevant asset. Affected real assets were not enumerated.

## A2: Ribbon alpha is always stepped

**Reference.** `W/Engine/Source/Model2/M2Model.cpp:3297–3315` (`M2AnimateTrack<fixed16,float>`, `0x0082AF40`) holds type-0 values but linearly interpolates nonzero-type fixed16 values in floating point. `:5651–5656` routes ribbon alpha through that specialization.

**Current.** `E/godot/core/src/asset/m2_format/m2_anim.rs:595–602` (`evaluate_i16_track`) always returns the previous key. `E/godot/core/src/m2_ribbons.rs:49–51` converts that held key to opacity; native sampling/drawing is wired at `E/godot/rust/src/ribbons.rs:87–104`.

**Counterexample.** A type-1 alpha track from 0 to 32767 over 1000 ms should be approximately 0.5 at 500 ms. The ribbon sampler produces 0 until the next key. Result: stepped color capture on newly generated ribbon edges rather than the authored smooth fade.

**Required change.** Respect interpolation when sampling ribbon fixed16 alpha, retaining enough precision through normalization. Preserve type-0 holds and the existing rule that an emitted edge retains its birth color. Do not claim all material opacity is broken: `E/godot/core/src/m2_material.rs:329–339` already has a distinct interpolation-aware fixed16 sampler.

**Acceptance.** Sample the fixture at 250/500/750 ms and obtain approximately 0.25/0.5/0.75; type 0 remains 0. Generate moving ribbon edges at those times and verify their captured alpha and native blended pixels. Existing edges must not acquire a new birth color on later updates. Real-asset impact remains unmeasured.

## A3: Short track tables do not select timeline zero

**Reference.** The vec3, fixed16 and float specializations select `animIndex < ...m_count ? animIndex : 0` before reading keys (`W/Engine/Source/Model2/M2Model.cpp:3298–3300,3344–3346,3434–3436`). This is an out-of-range animation-index rule, not a rule to replace an explicitly empty valid timeline.

**Current.** The generic evaluators directly call `track.sequences.get(seq_idx)?` (`E/godot/core/src/asset/m2_format/m2_anim.rs:582,596,609,663`). Native `sample_sequence` then substitutes zero translation, identity rotation or unit scale when evaluation returns `None` (`E/godot/rust/src/animation/mod.rs:323–340`). The separate material sampler already selects timeline zero when the requested index exceeds the table (`E/godot/core/src/m2_material.rs:308–318`).

**Counterexample.** Play sequence index 1 while a non-global translation track contains only timeline 0 with one key `[3,0,0]`. The reference selects that authored value. The native bone sampler gets `None` and substitutes zero. Changing animations can therefore reset that transform for this valid short-table input.

**Required change.** Apply the format's authored timeline-zero selection to out-of-range indices in affected generic samplers, consistently with the existing material path. Do not substitute timeline zero for an in-range empty timeline; preserve its existing no-value/default meaning. This is an authored indexing rule, not a request for a compatibility fallback or error suppression.

**Acceptance.** Use the exact one-timeline/sequence-1 fixture and assert the nonzero transform survives in the native bone pose. Separately test a two-timeline track whose timeline 1 is explicitly empty, an actually absent track, and a global-sequence track; those cases must not be conflated. No asset census or visual reproduction was performed.
