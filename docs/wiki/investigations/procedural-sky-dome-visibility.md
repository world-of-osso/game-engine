# Procedural Sky Dome Visibility

The ordinary Azeroth InWorld sky remained dark navy after `21feec27` restored the missing procedural-dome lifecycle because the dome geometry was still backface-culled and a dome created after settled game time retained default-white uniforms.

## Reproduction

At the live Azeroth clear-light position, `LightParamsID 12` explicitly selects raw `LightSkyboxID 0`, so InWorld creates `SkyDome`. Native inspection confirmed the named `sky_dome` entity existed while the unobstructed sky remained uniform navy.

## Root Causes

`SkyMaterial` culls `Face::Back` so the camera can render the sphere interior. `build_dome_indices` emitted outward-facing triangles, causing all interior-facing sky geometry to be culled.

`update_sky_colors` skipped work when `GameTime` had not advanced. A dome added after that first settled update therefore retained `SkyUniforms::default()` instead of receiving current `LightData` colors.

## Correction

Commit `58d4b12a` reverses the dome triangle winding for interior rendering. It also treats newly added sky-material handles as update candidates even when game time is unchanged, so late-created domes receive the settled sky colors.

This correction is limited to the procedural raw-zero InWorld path. It does not change authored M2 selection or the unresolved authored-cloud combiner behavior.

## Proof Status

- RED: `/tmp/inworld-sky-winding-red.log` records the outward-winding and late-uniform failures.
- GREEN, build, and native visual proof remain pending at this documentation checkpoint.

## Sources

- `src/rendering/skybox/mod.rs` — dome mesh indices, Back culling, and material-update invalidation.
- `src/rendering/skybox/tests/inworld_procedural.rs` — winding and late-dome color regressions.
- [[skybox]] — explicit procedural-vs-authored InWorld selection.

## See Also

- [[authored-skybox-black-output]] — independent authored M2 problem.
- [[procedural-cloud-regeneration]] — procedural cloud texture lifecycle.
