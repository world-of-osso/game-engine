# Character-Selection Visibility

Character-selection fog preserves nearby campsite scenery while fading distant terrain. Fog ownership and camera setup live in `src/scenes/char_select/scene/camera.rs`; see [skybox rendering](../wiki/systems/skybox.md).

## What it must do

- [ ] Nearby trees 45 world units from the camera remain unobscured by distance fog.
- [ ] Terrain at 150 units fades partially; terrain at 400 units is fully fogged.
- [ ] Fog range uses scene distance rather than the camera-to-character framing distance.
- [ ] Scene fog remains owned by character selection and is not overwritten by procedural sky updates.
- [ ] Preserve skybox-behind-geometry ordering; do not reveal clouds by painting over opaque terrain.

## How it works

- [Skybox rendering](../wiki/systems/skybox.md)
- [Authored sky depth](authored-skybox-depth.md)

## Implementation inventory

- `src/scenes/char_select/scene/camera.rs` — character camera and world-distance fog.
- `src/rendering/skybox/mod.rs` — excludes character-select cameras from global fog updates.

## Tests asserting this spec

- `src/scenes/char_select/scene/tests/camera_tests.rs`
- `src/scenes/char_select/scene/tests/render_path_tests.rs`
- `src/rendering/skybox/tests.rs`

## Known gaps (current cycle)

- [ ] Corrected regression and native view verification pending.

## Out of scope

- Camera reorientation, terrain removal, or drawing clouds through hills to recreate the former incorrect sky overlay.
