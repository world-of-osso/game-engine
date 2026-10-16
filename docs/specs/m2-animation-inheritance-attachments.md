# M2 authored inheritance and attachment activation

Authored M2 bone-parent selection and attachment participation follow the user-supplied clean-room A6/A7 spec (2026-10-15). Provenance labels in that spec are not implementation sources. [Animation wiki](../wiki/systems/animation.md#authored-parent-contribution-a6--2026-10-15) describes the implementation.

## What it must do

### A6
- [x] Preserve ordinary inheritance for unflagged bones and ignore low inheritance bits on roots.
- [x] With identity model transform and zero pivots, parent uniform scale2 produces child vertex length2 for flags0 and length1 for flags2.
- [x] Bit1 selects model translation; cases2/4/6 respectively retain parent directions/model lengths, model directions/parent lengths, and model basis.
- [x] Without bit1, preserve the parent's transformed child pivot when replacing its basis.
- [ ] Apply low-bit selection before local animation and billboards, without changing crossfade state or losing affine shear in native skinning.
- [x] Render an authentic low-bit model before/after at1920×1080 in m2debug.

### A7
- [ ] Retain MD20 and SKA1 attachment byte tracks, defaulting missing samples to active1.
- [ ] Sample current parent sequence/local or global timeline; preserve attachment lookup identity and ownership.
- [ ] Gate attached-child animation evaluation, callbacks and shadow participation across active0→inactive100→active200ms, sequence switches and always-active/missing controls.
- [ ] Exercise native equipment/spell attachment, not parser retention alone.

## How it works

- [Animation](../wiki/systems/animation.md#authored-parent-contribution-a6--2026-10-15)

## Implementation inventory

- `godot/core/src/m2_inheritance.rs` — pure affine parent selection in converted axes.
- `godot/rust/src/animation/billboard.rs` — parent-first native affine composition and subsequent billboards.
- `godot/rust/src/animation/mod.rs` — ordinary local poses followed by flagged affine overrides.

## Tests asserting this spec

- `godot/core/src/m2_inheritance.rs` — uniform scale, translation, rotation, pivot and nonidentity-model fixtures.
- `godot/rust/src/animation/billboard.rs` — root and ordinary-descendant controls.
- `godot/tests/m2_inheritance_pixels.gd` — authentic125913 bone67 flags4/parent12; same-clock ordinary-hierarchy baseline versus restored affine override, native1920×1080.

## Known gaps (current cycle)

- [ ] A6 runtime-added flags/modern contracts: no allowed-source evidence established. Only authored low bits are implemented.
- [ ] A6 nonuniform/sheared native palette fixture remains unmeasured. Parent-selection CPU3 and native root/descendant1 pass. Authentic125913 same-clock m2debug native run passes; PNGs inspected at `/syncthing/AgentShared/2026-10-15/m2-anim/a6-125913-{before,after}.png`. Baseline clears affine overrides in the same binary; it is not an old-revision executable. Bone67 keeps its transformed pivot while its inherited tilted basis becomes identity.
- [ ] A7 implementation and native participation fixture pending.
- [ ] A7 color-pass behavior: activation alone does not establish hiding; do not change color visibility without evidence.
- [ ] A7 reactivation clock: reference skip alone establishes neither freeze nor catch-up; do not claim either as client parity.

## Out of scope

- Invented flag names/runtime bits, deletion or detachment on inactivity, universal visibility rules, and forbidden source/build-data access.
