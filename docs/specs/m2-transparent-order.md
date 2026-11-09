# M2 transparent batch order

M2 batches must retain the authored priority plane when the native renderer draws overlapping transparent surfaces. The Stormwind Trade District fountain (366804) is the regression fixture. [Investigation and evidence](../wiki/investigations/fountain-camera-flash.md).

## What it must do

- [x] Preserve each batch's signed authored priority plane in its rendered composition, including negative and positive values.
- [x] Keep fountain water composition equal to the source-ordered reference across camera yaw and pitch sweeps, even when unrelated transparent draws change the sort list.
- [x] Leave texture animation, shader equations, opacity and depth/cull state unchanged.

## How it works

- [Fountain camera-flash investigation](../wiki/investigations/fountain-camera-flash.md).
- [M2 materials](../wiki/formats/m2-format.md#materials-and-render-flags).

## Implementation inventory

- `godot/rust/src/assets/material.rs` — creates native materials from resolved authored batches.
- `godot/tests/m2_wwv_oracle.gd` — independent raw skin reader.

## Tests asserting this spec

- `godot/tests/m2_transparent_priority.gd` — actual fountain pixels versus a raw-source priority reference at 120 camera poses, with zero-alpha transparent draws that affect sorting but not colour.

## Known gaps (current cycle)

None.

## Out of scope

- Same-plane sort-center/radius parity, ordering between distinct M2 objects, shader/lighting changes, and Windows or current-retail certification.
