# M2 material animation

M2 colours, opacity, texture weights and UV transforms must use their authored local or global timeline. Source records live in M2 files; [investigation](../wiki/investigations/avenging-wrath-material-time.md) records the Avenging Wrath counterexample.

## What it must do

- [ ] Local tracks sample the owning model's playing sequence and local time, including Stand, Hold and Decay.
- [ ] Global-sequence tracks use timeline zero and the shared global clock, independently of local clip changes.
- [ ] Models without skeletal playback retain their authored Stand-loop material animation.
- [ ] Constant keys on different local sequences still update when the sequence changes.
- [ ] Native source-backed rendering shows Avenging Wrath's held feather sheets, not their initial zero-opacity Stand keys.

## How it works

- [Avenging Wrath material time](../wiki/investigations/avenging-wrath-material-time.md)
- [M2 tracks and batches](../wiki/formats/m2-format.md)

## Implementation inventory

- `godot/core/src/m2_material.rs` — material timeline sampling and variation classification.
- `godot/rust/src/assets/uv_animation.rs` — owning model playback and shared global clock.
- `godot/rust/src/assets/material.rs` — initial material sample and GPU binding.

## Tests asserting this spec

- `godot/core/tests/m2_material.rs` — authentic phase opacity, global UV independence, clip-dependent constant keys and texture resolution.
- `godot/tests/m2_real_pixels.gd`, `m2_wwv_oracle.gd` — independently sampled source records against native rendered pixels.

## Known gaps (current cycle)

- [ ] Reproduce and explain the user's production rainbow quad; do not equate a missing feather-sheet fix with that defect.

## Out of scope

- Positioner59 interpretation: its entry enum semantics have no established reference.
- Texture substitutions, replacement artwork, unrelated material equations or blend-state changes.
