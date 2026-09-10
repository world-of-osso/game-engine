# Procedural Sky Dome Visibility

The ordinary Azeroth InWorld sky remained dark navy after `21feec27` restored the missing procedural-dome lifecycle because the dome geometry was still backface-culled and a dome created after settled game time retained default-white uniforms.

## Reproduction

At the live Azeroth clear-light position, `LightParamsID 12` explicitly selects raw `LightSkyboxID 0`, so InWorld creates `SkyDome`. Native inspection confirmed the named `sky_dome` entity existed while the unobstructed sky remained uniform navy.

## Root Causes

`SkyMaterial` culls `Face::Back` so the camera can render the sphere interior. `build_dome_indices` emitted outward-facing triangles, causing all interior-facing sky geometry to be culled.

`update_sky_colors` skipped work when `GameTime` had not advanced. A dome added after that first settled update therefore retained `SkyUniforms::default()` instead of receiving current `LightData` colors.

The later cloud grid came from two independent procedural-texture defects. Ridge noise converted a high-bit seed into `f32` coordinate offsets around 45–61 million, where ULP 4 quantized neighboring source samples into rectangular blocks. The generated image was nonperiodic despite repeat sampling, so its opposite edges jumped at each wrap. The shader also applied `fract` before its fractional `1.9` secondary longitude scale, creating a spherical seam.

After seam repair, the remaining faint-cloud diagnosis initially assumed `LightData.ron` density 0. That source is not active here: the loaded `LightData` 12/noon CSV cache supplies density `0.5`, which retains the original threshold `0.62`. Periodic cloud shapes rarely exceed that threshold, so `smoothstep(threshold, 1.0, cloud_shape)` made the layer nearly transparent.

## Correction

Commit `58d4b12a` reverses the dome triangle winding for interior rendering. It also treats newly added sky-material handles as update candidates even when game time is unchanged, so late-created domes receive the settled sky colors.

The shared dome correction applies to InWorld and the standalone skybox-debug screen. `83cf11ec` also enables that screen's shared color/environment updates.

`389e0185` replaces the procedural texture generator with periodic integer-hashed gradient fBm. Seed variation stays in integer hashing; unchanged image size, frequencies, density shaping, and repeat sampler now compose without edge jumps. `077599df` retains primary spherical UVs unwrapped until sampling and changes the secondary longitude frequency to integer `2.0`, so repeat sampling has no discontinuity at the longitude wrap.

`a55e0f5b` retains the original density-to-threshold mapping (`0.92` clear through `0.32` full) and centers the existing soft edge on that threshold (`±0.1`). It changes opacity only: no sky colors, exposure, noise generation, or density controls. A wider threshold remap was rejected because its mid-density GPU case lost clear patches. Authored M2 combiner behavior is unchanged.

## Proof Status

- Earlier RED/GREEN and independent reports are retained locally under `data/diagnostics/inworld-sky/`. At `83cf11ec`, 59 distinct relevant tests pass, including seven new regressions; dev build/check pass. Global formatting still reports 104 unchanged vendor files.
- Cloud-tiling verification at `077599df`: seven generator tests, one actual-GPU seam regression, and one preservation test pass; dev build/check pass. The GPU seam difference falls from 28 color levels (`139` versus `111`) to 3 (`139` versus `136`) across a finite angular separation. Logs and independent report: `data/diagnostics/cloud-tiling/`.
- Density GPU RED showed the loaded mid-density case had no bright cloud pixels (maximum 64). At `a55e0f5b`, the generated-texture fixture reports 22.12% bright pixels and 60.16% dark pixels at mid density; full density is 100% bright and clear density 100% dark. `data/diagnostics/cloud-visibility/standalone-after.webp` confirms visible soft cloud patches with the existing sky colors and tonemapping. The seam fixture was recalibrated to unsaturated cloud opacity at `e43bb773`, preserving its finite-angle tolerance; GPU samples are 112 versus 108.
- `standalone-after.webp` shows continuous cloud variation without the conspicuous rectangular blocks from `user-before.png`, using `--screen skyboxdebug --light-skybox-id 0`. Framing differs; this is not a same-view pixel comparison or exhaustive all-direction proof. No user in-world client was moved or restarted for this correction.
- Native `--screen skyboxdebug --light-skybox-id 0` renders visible procedural clouds and a horizon gradient in `standalone-procedural.webp`. ID 0 deliberately selects no authored model in this diagnostic; the tree contains `sky_dome`, the debug camera, and reference ground. It uses the same mesh, material, and LightParams12 color updates as InWorld.
- The Light.csv/LightParams selection and InWorld lifecycle are separately covered by the concrete Azeroth regression. A corrected same-view native InWorld screenshot was not captured: the user requested standalone-screen verification instead of further character/camera changes.
- Default debug mode also loads the unrelated `costalislandskybox.m2`; captures `standalone-screen.webp` and `standalone-authored.webp` retain authored artifacts and are not evidence that all M2 skyboxes are correct.

## Sources

- `src/rendering/skybox/mod.rs` — dome mesh indices, Back culling, and material-update invalidation.
- `src/rendering/skybox/cloud_texture.rs` — periodic seeded cloud generation.
- `assets/shaders/sky.wgsl` — spherical cloud UV sampling.
- `src/rendering/skybox/tests/inworld_procedural.rs` — winding and late-dome color regressions.
- [[skybox]] — explicit procedural-vs-authored InWorld selection.

## See Also

- [[authored-skybox-black-output]] — independent authored M2 problem.
- [[procedural-cloud-regeneration]] — procedural cloud texture lifecycle.
