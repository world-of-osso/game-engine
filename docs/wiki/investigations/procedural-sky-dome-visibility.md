# Procedural Sky Dome Visibility

The ordinary Azeroth InWorld sky remained dark navy after `21feec27` restored the missing procedural-dome lifecycle because the dome geometry was still backface-culled and a dome created after settled game time retained default-white uniforms.

## Reproduction

At the live Azeroth clear-light position, `LightParamsID 12` explicitly selects raw `LightSkyboxID 0`, so InWorld creates `SkyDome`. Native inspection confirmed the named `sky_dome` entity existed while the unobstructed sky remained uniform navy.

## Root Causes

`SkyMaterial` culls `Face::Back` so the camera can render the sphere interior. `build_dome_indices` emitted outward-facing triangles, causing all interior-facing sky geometry to be culled.

`update_sky_colors` skipped work when `GameTime` had not advanced. A dome added after that first settled update therefore retained `SkyUniforms::default()` instead of receiving current `LightData` colors.

## Correction

Commit `58d4b12a` reverses the dome triangle winding for interior rendering. It also treats newly added sky-material handles as update candidates even when game time is unchanged, so late-created domes receive the settled sky colors.

The shared dome correction applies to InWorld and the standalone skybox-debug screen. `83cf11ec` also enables that screen's shared color/environment updates. Authored M2 combiner behavior is unchanged.

## Proof Status

- RED/GREEN and independent reports are retained locally under `data/diagnostics/inworld-sky/`. At `83cf11ec`, 59 distinct relevant tests pass, including seven new regressions; dev build/check pass. Global formatting still reports 104 unchanged vendor files.
- Native `--screen skyboxdebug --light-skybox-id 0` renders visible procedural clouds and a horizon gradient in `standalone-procedural.webp`. ID 0 deliberately selects no authored model in this diagnostic; the tree contains `sky_dome`, the debug camera, and reference ground. It uses the same mesh, material, and LightParams12 color updates as InWorld.
- The Light.csv/LightParams selection and InWorld lifecycle are separately covered by the concrete Azeroth regression. A corrected same-view native InWorld screenshot was not captured: the user requested standalone-screen verification instead of further character/camera changes.
- Default debug mode also loads the unrelated `costalislandskybox.m2`; captures `standalone-screen.webp` and `standalone-authored.webp` retain authored artifacts and are not evidence that all M2 skyboxes are correct.

## Sources

- `src/rendering/skybox/mod.rs` — dome mesh indices, Back culling, and material-update invalidation.
- `src/rendering/skybox/tests/inworld_procedural.rs` — winding and late-dome color regressions.
- [[skybox]] — explicit procedural-vs-authored InWorld selection.

## See Also

- [[authored-skybox-black-output]] — independent authored M2 problem.
- [[procedural-cloud-regeneration]] — procedural cloud texture lifecycle.
