# M2 weighted loop variations

M2 looping playback selects authored alternatives rather than following a temporal chain. Source lives in `src/rendering/model/animation/variants.rs` and `runtime.rs`; [animation](../wiki/systems/animation.md#weighted-loop-variations) describes the implementation.

## What it must do

- [x] Parse signed frequency, replay bounds, and variation-list links without conflating the alias field.
- [x] Start selection at variation0 for the active animation ID on every loop boundary, including after a terminal rare variation.
- [x] Select by authored weights with deterministic injectable rolls; all32767 wolf rolls yield30445/1092/1170/60 selections.
- [x] Reject invalid linked-family metadata rather than synthesize uniform weights or follow another animation ID.
- [x] Keep separate reproducible entity random streams; non-looping completion does not select alternatives.
- [x] Preserve overflow across multiple cycle boundaries and existing transition behavior. A120-second update matches partitioned elapsed time; pathological catch-up errors instead of silently dropping time.

## How it works

- [Weighted loop playback and limitations](../wiki/systems/animation.md#weighted-loop-variations)
- [Sequence fields](../wiki/formats/m2-format.md#animation-sequences)

## Implementation inventory

- `src/asset/m2_format/m2_anim.rs` — sequence record parsing.
- `src/rendering/model/animation/variants.rs` — validated family selection and entity random streams.
- `src/rendering/model/animation/runtime.rs` — cycle boundaries and crossfade clock advancement.
- `src/rendering/model/animation.rs` — required per-player random state.

## Tests asserting this spec

- `variants.rs::tests` — actual wolf weights, parser fields, deterministic ranges, invalid data, entity independence, non-looping and large elapsed behavior.
- `tests/unit/animation_tests/core.rs` — terminal wolf recovery and overflow/crossfade selection.
- Existing animation tests retain jump, emote, interrupted blending and actual Bevy pose evaluation.

## Known gaps (current cycle)

- [ ] Parent validation of visible wolf idle behavior; no native run is part of this implementation slice.

## Out of scope

Nonzero replay scheduling, new alias interpretation, locomotion-state generation, nameplates, and character equipment. Replay bounds are retained for a future explicit replay-policy implementation; the affected wolf's bounds are allzero.
