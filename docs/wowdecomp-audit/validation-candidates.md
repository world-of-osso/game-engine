# Validation candidates — not approved rework findings

[Audit scope](README.md). These differences or omissions need additional evidence before prescribing a change to the project's newer client behavior. `W`/`E` denote the pinned source aliases in the index.

## V1: M2 alpha-key cutoff differs

**Observed sources.** `W/Engine/Source/Model2/CM2SceneRender.cpp:282–305` sets alpha reference to 0 for opaque, `element alpha × 0.87843138` for alpha-key, and `0.0039215689` for other blend modes. `E/godot/shaders/m2.gdshader:237–239` uses `discard_alpha < 0.501960814` for alpha-key only when the selected combiner permits discard.

**Why not a confirmed defect.** Different client generations, combiner alpha expressions, render passes and placement fades can change the applicable comparison. The alpha reference setter alone does not establish the final source fragment comparison. The engine's [M2 format notes](../wiki/formats/m2-format.md) document a Retail-derived cutoff; replacing it globally with the Wrath value is not justified by this audit.

**Validation and conditional change.** Trace the reference shader's final alpha comparison, match a batch/combiner/pass and asset build, then render an alpha ramp with element opacity 1 and 0.5. Compare survivor pixels and depth. If that target actually follows the reference, use its opacity-dependent cutoff and tiny-alpha behavior in the corresponding path; otherwise retain the newer rule and document the version difference.

## V2: Quaternion interpolation curve differs

**Observed sources.** `W/Engine/Source/Model2/M2Model.cpp:2415–2445` uses normalized linear interpolation for interpolated compressed quaternion keys. `E/godot/core/src/asset/m2_format/m2_anim.rs:604–621,636–660` uses spherical interpolation, with a near-equal linear branch. The functions differ for widely separated keys away from the midpoint.

**Why not a confirmed target defect.** This is distinct from A1's ignored step mode. This audit did not establish the intended Retail interpolation curve on a real bone track, nor measure a visible timing discrepancy.

**Validation and conditional change.** Use two rotations 120 degrees apart and sample at 25% and 75%, avoiding the midpoint where the curves coincide. Match decoded source keys and coordinate conventions. If the target uses nlerp, use that within tracks while preserving the independently specified crossfade behavior; do not globally replace every quaternion blend.

## V3: Wrath and Retail fog inputs differ

**Observed sources.** `W/Engine/Source/Model2/CM2SceneRender.cpp:210–248` selects blend-dependent fog colors and submits start/end/density. `E/godot/shaders/m2.gdshader:251–276` calls the shared Retail fog implementation; `E/godot/shaders/retail_fog.gdshaderinc:61–99` includes additional height/artistic/sun terms.

**Why not a confirmed defect.** A richer model is not evidence of incorrect reproduction. The [Retail lighting system](../wiki/systems/retail-lighting.md) deliberately follows newer data. No matched scene/batch input demonstrating wrong fog output was obtained. Blend-identity fog handling exists in the current shader path; it is not generally missing.

**Validation and conditional change.** Compare opaque, alpha, additive and modulating materials at near/mid/far distances in a target-build scene, including unfogged flags. Change only the demonstrated incorrect target behavior; do not remove Retail fog terms merely to resemble the legacy interface.

## V4: Modern particle refraction remains intentionally absent

**Current capability gap.** `E/godot/rust/src/particles.rs:315–326` explicitly filters `PixelShader::Refraction` before native placement. `E/godot/core/src/m2_particles.rs:149–160` identifies that shader from effect flags. The [Avenging Wrath investigation](../wiki/investigations/avenging-wrath-material-time.md) explains why normal-map/refraction parameters must not be drawn as ordinary color particles.

**Reference limitation.** This audit did not establish an equivalent distortion pass in the pinned Wrath reference. Therefore this is a documented current shader-effect omission, not counted among the wowdecomp-demonstrated mismatches.

**Required future behavior.** A version-appropriate refraction pass must distort sampled scene color according to the authored effect rather than painting its parameter texture. Preserve the existing forward exclusion until that behavior exists; removing it alone reintroduces the known visual error.

**Acceptance.** Use the investigation's real Divine Shield model `1127086` against a high-contrast background, compare distortion on/off, and retain Avenging Wrath color-particle controls. Verify normal-map data never appears as visible color and depth/occlusion are correct. This audit ran none of those checks.
