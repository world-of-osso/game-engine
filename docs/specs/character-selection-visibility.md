# Character-Selection Visibility

Character-selection fog preserves nearby campsite scenery while fading distant terrain. Fog ownership and camera setup live in `src/scenes/char_select/scene/camera.rs`; see [skybox rendering](../wiki/systems/skybox.md).

## What it must do

- [x] Nearby trees 45 world units from the camera remain unobscured by distance fog.
- [x] Terrain at 150 units fades partially; terrain at 400 units is fully fogged.
- [x] Fog range uses scene distance rather than the camera-to-character framing distance.
- [x] Scene fog remains owned by character selection and is not overwritten by procedural sky updates.
- [x] Preserve skybox-behind-geometry ordering; do not reveal clouds by painting over opaque terrain.

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

Native inspection confirms clearer nearby trees and campsite detail. Cloud visibility remains limited by opaque cliff/terrain in this view, including with fog disabled; the fog correction does not claim to remove that obstruction.

## Out of scope

- Camera reorientation, terrain removal, or drawing clouds through hills to recreate the former incorrect sky overlay.
