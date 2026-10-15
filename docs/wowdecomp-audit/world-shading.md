# M2 transparent ordering and WMO emissive lighting

[Scope and pins](README.md). Source differences below have concrete metadata-sensitive counterexamples. They do not establish a wholesale missing M2/WMO shader implementation or Retail pixel parity.

## R1: Authored M2 sort center and radius cannot influence draw order

**Reference.** `W/Engine/Source/Model2/M2Data.h:301–313` defines skin-section sort center/radius. `W/Engine/Source/Model2/M2Scene.cpp:1889–1941` transforms the section center by its center-bone matrix and, under the relevant batch flags, offsets it by the scaled radius to obtain element depth/sort depth. The selected branch also depends on model/render/material conditions; it does not apply the same key to every transparent batch. The transparent comparator at `:1263–1300` orders descending sort depth, split flag, ascending priority plane and descending element depth; sorting is invoked at `:2194–2195`.

**Current.** `E/godot/core/src/asset/m2_format/parser.rs:36–51` reads `_sort_center_position` and `_sort_radius` into the raw record but discards them from the exposed submesh. Batch resolution retains priority plane/material layer and sorts on those fields (`E/godot/core/src/asset/m2_batch_data.rs:230–238`). Native batch loading/mesh construction is at `E/godot/rust/src/assets/mod.rs:460–490,720–746`; actual mesh-instance/material assembly and attachment is at `:534–559`. They cannot react to metadata discarded by exposed-submesh conversion (`E/godot/core/src/asset/m2_format/parser.rs:270–282`). Godot's ordinary geometry-bound sorting is not equivalent to an authored section key.

**Required change.** Preserve and apply the authored section/batch sorting semantics for the applicable transparent draw path, including center-bone transformation, flagged radius adjustment and the existing priority tie-breaks. Do not globally replace every sort key or treat all priority-plane handling as absent.

**Acceptance.** Two overlapping transparent batches have identical geometry/AABBs, split state and priority, but different authored sort centers. Use a center-sensitive reference branch, for example model flag `0x10` clear and `depthSort` false; change only the centers so its sort keys reverse their order. The engine must then produce the corresponding opposite composite, rather than remaining invariant to discarded metadata. Add a radius/flag case where center-only sorting gives the wrong order, a moving center bone, equal-depth priority ties and an opaque control. Real-asset use and native compositing impact remain unmeasured.

## S2: Authored WMO SIDN emissive color is not bound

**Reference.** `W/WoW/Source/WorldClient/MapObj.cpp:604–616` derives the frame SIDN color from a flagged material's authored color and the current day/night curve. `:984–992` combines that value with emissive ambient using the source's saturated channel arithmetic, halves RGB and submits it with `SetEmissiveColor`.

**Current.** `E/godot/core/src/asset/wmo_format/parser.rs:75–89` retains SIDN color and parsed flags; `parser_types.rs:95–120` exposes both. The material binder at `E/godot/rust/src/wmo/scene.rs:550–561` binds `emissive` to zero, with no SIDN-color/curve input. `E/godot/shaders/wmo.gdshader:292–310` uses the bound emissive term and shader-specific texture contributions, not the retained SIDN value. Textured emissive effects therefore exist, but this authored time-dependent contribution is omitted.

**Required change.** For an applicable SIDN material, contribute its authored color using the target-validated time-of-day curve and channel combination. Preserve unflagged materials and existing texture emissive behavior. Wrath's integer color arithmetic is evidence for that build, not automatic proof of modern linear-space coefficients.

**Acceptance.** With direct lighting disabled and other emissive inputs held fixed, use a flagged material with authored SIDN RGB `(128,64,32)` at curve values 0 and 1. Compare the reference-derived contribution and final pixels. Change only SIDN color and verify output responds; an unflagged control must not. Qualify a target-build material before claiming a shipped-asset fix.
