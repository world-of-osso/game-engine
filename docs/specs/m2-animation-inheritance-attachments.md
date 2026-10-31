# M2 authored inheritance and attachment activation

Authored M2 bone-parent selection and attachment participation follow the user-supplied clean-room A6/A7 spec (2026-10-15). Provenance labels in that spec are not implementation sources. [Animation wiki](../wiki/systems/animation.md#authored-parent-contribution-a6--2026-10-15) describes the implementation.

## What it must do

### Parent skeleton animations
- [ ] Resolve base animations in the child skeleton first, then its SKPD parent unless PABC excludes the ID. Sample the selected bank's sequences, bone tracks, global timelines, hierarchy, pivots and flags.
- [ ] Preserve smooth transitions and attachment/bone correspondence when the two skeleton hierarchies differ. WebWowViewerCpp `animationManager.cpp:101–127,392–405,410–413,472–490` selects the parent's complete bone definitions for inherited clips.
- [ ] Verify Kokia's inherited Death with real assets and a native corpse capture, plus a child-only control.

### A6
- [x] Preserve ordinary inheritance for unflagged bones and ignore low inheritance bits on roots.
- [x] With identity model transform and zero pivots, parent uniform scale2 produces child vertex length2 for flags0 and length1 for flags2.
- [x] Bit1 selects model translation; cases2/4/6 respectively retain parent directions/model lengths, model directions/parent lengths, and model basis.
- [x] Without bit1, preserve the parent's transformed child pivot when replacing its basis.
- [ ] Apply low-bit selection before local animation and billboards, without changing crossfade state or losing affine shear in native skinning.
- [x] Render an authentic low-bit model before/after at1920×1080 in m2debug.

### Camera-less billboard poses
- [x] Without an active camera-facing operation, billboard-only bones retain their sampled position, quaternion and scale, including authored zero scale, without Basis quaternion errors. Authentic3049899 CPU-engine regression passes1/1 after the preserved six-error RED.
- [ ] Parent-first inheritance, ordinary descendants and the A6 affine/shear contract remain unchanged; no scale/epsilon predicate or skipped authored bone.

### A7
- [x] Retain MD20 and SKA1 attachment byte tracks, defaulting missing samples to active1.
- [x] Sample current parent sequence/local or global timeline; preserve attachment lookup identity and ownership.
- [x] Gate attached-child animation evaluation, callbacks and shadow participation across active0→inactive100→active200ms, sequence switches and always-active/missing controls.
- [x] Exercise native equipment/spell attachment, not parser retention alone.

### M2-MAT (approved 2026-10-17)

Clean-room user-supplied M2-MAT requirements; W/C labels are provenance only.

- [x] Per-track out-of-range selection uses timeline zero; an in-range empty timeline uses the channel default. Existing master behavior retained, including classic wings165644 and creatures356426/125854 tint controls.
- [x] Material modes2/3 use ordinary non-step endpoint interpolation, not held keys. Scalar/vector0→1 at0–1000ms yields0.25 at250ms within fixed16 quantization.
- [x] Runtime integration: active base transitions blend eligible local non-step RGB, alpha, texture weights and UV translation/scale/quaternion rotation, using both sequence clocks and the bone weight.
- [x] Pure material sampler: empty transition sides contribute defaults; global tracks use their global clock without local blending; step tracks retain their current held value.
- [x] Interrupted transitions retain raw eligible material channels in load-time buffers, using the same base-state bone weight. Runtime85 filtered animation tests and native interrupted-weight controls pass.
- [x] Native both-skin1920×1080 real-asset before/after and165644 tint control; both Hold images are pixel-identical across binaries. [Proof](../wiki/systems/animation.md#local-material-transitions-m2-mat--2026-10-17).
- [ ] M2-MAT-CUBIC: target-qualified material spline/tangent semantics remain unproven; camera-style Hermite/Bezier sampling is explicitly excluded.

## How it works

- [Animation](../wiki/systems/animation.md#authored-parent-contribution-a6--2026-10-15)

## Implementation inventory

- `godot/core/src/m2_inheritance.rs` — pure affine parent selection in converted axes.
- `godot/rust/src/animation/billboard.rs` — parent-first native affine composition and subsequent billboards.
- `godot/rust/src/animation/mod.rs` — ordinary local poses followed by flagged affine overrides; child sampling/callback gates.
- `godot/core/src/asset/m2_format/m2_attach.rs` — MD20/SKA1 activation retention and timeline sampling.
- `godot/rust/src/animation/attachment.rs` — ancestor eligibility and reversible native shadow settings.
- `godot/rust/src/assets/equipment.rs` — refresh eligibility-owned shadow settings after equipment relocation.

## Tests asserting this spec

- `godot/core/src/m2_inheritance.rs` — uniform scale, translation, rotation, pivot and nonidentity-model fixtures.
- `godot/rust/src/animation/billboard.rs` — root and ordinary-descendant controls.
- `godot/core/tests/unit/asset/m2_attach_tests.rs` — MD20/SKA1 keys, global/local controls, malformed records, real1081566/971117 zero keys.
- `godot/rust/src/animation/action_tests.rs` — actual HumanMaleHD `$SCD` marker delivery/consumption while active/inactive.
- `godot/tests/m2_attachment_activation.gd`, `scripts/agent/m2-anim-fixtures.py` — private solar-spell track fixture with actual equipped item25, child pose writes, callback eligibility, shadows, identity, sequence/control tracks and existing native clock/color-state measurements.
- `godot/tests/m2_inheritance_pixels.gd` — authentic125913 bone67 flags4/parent12; same-clock ordinary-hierarchy baseline versus restored affine override, native1920×1080.
- `godot/network/tests/journal_chooser_zero_scale.rs`, `godot/tests/journal_chooser_zero_scale.gd` — ignored CPU-only real Skeleton3D boundary, authentic3049899 idle0/t0; source quaternion/zero-scale retention and zero basis errors.

M2-MAT fixtures:
- `godot/core/tests/m2_material.rs`, `m2_material_transition.rs` — authentic tint/clock/selection controls, ordinary nonzero interpolation, all local channels/defaults, quaternion composition, interrupted snapshots and thread-scoped allocation counts.
- `godot/rust/src/animation/material_tests.rs` — authentic165644 base-state two-clock and repeated zero-elapsed retransition behavior.
- `godot/tests/m2_material_transition.gd` — both skins, same-camera/fixed-clock full-HD before/after and165644 Hold tint control.

## Known gaps (current cycle)

- [ ] `BILLBOARD_CAMERA_PRESENT_ZERO_SCALE_OVERRIDE_DECOMPOSES`: in-world camera-present billboards with authored zero scale can still reach Godot's decomposing interpolate; excluded from the bounded camera-less correction.

- [ ] A6 runtime-added flags/modern contracts: no allowed-source evidence established. Only authored low bits are implemented.
- [ ] A6 nonuniform/sheared native palette fixture remains unmeasured. Parent-selection CPU3 and native root/descendant1 pass. Authentic125913 same-clock m2debug native run passes; PNGs inspected at `/syncthing/AgentShared/2026-10-15/m2-anim/a6-125913-{before,after}.png`. Baseline clears affine overrides in the same binary; it is not an old-revision executable. Bone67 keeps its transformed pivot while its inherited tilted basis becomes identity.
- Native/CPU acceptance at8a1afc3b0: filtered native animation83, attachment parser7, inheritance3, format check pass; no Rust compile warnings. Native solar-spell→HumanMaleHD→original authored item25 hand point passes at0/99/100/199/200ms: eligibility/pose-write/shadow follows1/1/0/0/1; missing/constant controls remain active, item identity/owner/visibility stay unchanged, exact original shadow mode2 restores. Sequence switch first asserts inactive100ms, then active sequence1. Actual `$SCD` dispatch suppression/no replay is CPU evidence; native fixture observes callback eligibility, not audible voice dispatch.
- [ ] External attachment-key storage/chunk association is not established here; parser currently reads activation elements from their containing MD20/SKA1 blob. No external attachment-chunk tag semantics are invented.
- [ ] Native child material/particle/ribbon clocks remain their existing separate systems; no additional evaluation/clock parity claim. The established native gate covers skeletal pose evaluation, reported action callbacks and recursive geometry shadow participation.
- [ ] A7 reference-client color-pass behavior: activation alone does not establish hiding. Native visibility-state measurements stay true; no reference-client raster measurement or universal hiding rule is claimed.
- [ ] A7 reactivation clock: reference skip alone establishes neither freeze nor catch-up. Native pre-change measurement advances child20/40/60/80/100ms across parent0/99/100/199/200ms; this existing clock policy is preserved while suppressing pose writes/callbacks. Neither clock policy is claimed as reference-client parity.

## Out of scope

- Invented flag names/runtime bits, deletion or detachment on inactivity, universal visibility rules, and forbidden source/build-data access.
