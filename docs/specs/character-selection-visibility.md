# Character-Selection Visibility

Character-selection fog preserves nearby campsite scenery while fading distant terrain. Fog ownership and camera setup live in `src/scenes/char_select/scene/camera.rs`; see [skybox rendering](../wiki/systems/skybox.md).

## What it must do

- [x] Nearby trees 45 world units from the camera remain unobscured by distance fog.
- [x] Terrain at 150 units fades partially; even beyond the fade range, at least half of its unfogged contribution remains visible, matching the reference's readable distant scenery.
- [x] Fog range uses scene distance rather than the camera-to-character framing distance.
- [x] Scene fog remains owned by character selection and is not overwritten by procedural sky updates.
- [x] Preserve skybox-behind-geometry ordering; do not reveal clouds by painting over opaque terrain.
- [x] Character selection uses one sky-owned environmental directional light and generated camera IBL; it does not manufacture directional campfire or terrain-fill lights.
- [x] Sky color/time updates affect only the explicitly sky-owned environmental sun, never unrelated directional or M2 point lights.
- [x] M2 attachments retain every authored type-1 point light; animated tracks use the model-local or global duration declared by the model, and joint-bound item lights despawn with their item root.

## How it works

- [Skybox rendering](../wiki/systems/skybox.md)
- [Authored sky depth](authored-skybox-depth.md)

## Implementation inventory

- `src/scenes/char_select/scene/camera.rs` — character camera and world-distance fog.
- `src/rendering/skybox/mod.rs` — excludes character-select cameras from global fog updates; environmental-sun and camera-IBL ownership.
- `src/scenes/char_select/scene/lighting.rs` — character-selection environmental-light setup.
- `src/rendering/model/m2_spawn.rs` and `src/rendering/model/animation.rs` — M2 point-light attachment and animation.

## Tests asserting this spec

- `src/scenes/char_select/scene/tests/camera_tests.rs`
- `src/scenes/char_select/scene/tests/render_path_tests.rs`
- `src/rendering/skybox/tests.rs`
- `src/scenes/char_select/scene/tests/m2_lighting_tests.rs`
- `tests/unit/equipment_event_tests.rs`

## Known gaps (current cycle)

Bounded native reference inspection confirms better retained distant detail and blue-green haze. This is color fog over retained geometry, not terrain transparency, terrain deletion, or a camera change. At `4fcc7437`, focused ownership/attachment/clock/lifetime regressions pass; a bounded native character-select capture at `698ea9e5` reports one environment sun, ambient brightness 0, and a non-black terrain/character frame. Its unrelated controller, malformed-MH2O, and missing-texture warnings remain recorded. Independent final fmt/check/readability verification remains pending. Exact WoW lighting/assets, photometric conversion, and brightness equivalence are not claimed.

## Out of scope

- Camera reorientation, terrain removal, or drawing clouds through hills to recreate the former incorrect sky overlay.
