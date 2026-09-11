# Character-Select Ground Patch and Dark Terrain

The bright campsite island is an explicit 42×42 `StandardMaterial` grass plane over a much darker ADT `TerrainMaterial` scene. It reproduces with the same binary against both canonical and retained-worktree data, so it is neither a registry-UI regression nor explained solely by missing worktree assets. The exact terrain-shader cause of the darkness remains unproven.

## Observed Runtime Boundary

On September 11, 2026, the same canonical binary was run for 30 seconds with `BEVY_ASSET_ROOT` set separately to canonical `game-engine` data and the retained `game-engine-rsx-login` worktree. Both captures show the bright campsite patch and very dark surrounding terrain.

Both runs reported:

- `terrain:2703_31_37` displayed
- 256 spawned ADT terrain chunks
- 13 loaded ground textures
- displayed `costalislandskybox.m2`
- `WarbandTerrain` and `CampsiteGroundPatch` entities
- char-select ambient plus both directional lights

The dark pixels are textured dark brown/gray, not uniform camera-clear color. The visible WMO is marked `is_displayed=false`, so missing environment geometry remains a contributor to the sparse backdrop but does not explain the material boundary.

Evidence: `data/diagnostics/charselect-ground-20260911/{canonical-settled,worktree-settled}/` and `comparison.md`.

## Confirmed Patch Mechanism

`src/scenes/char_select/scene/background.rs` creates `CampsiteGroundPatch` only after primary warband terrain succeeds and a terrain height is available at the campsite focus:

- `CAMPSITE_GROUND_PATCH_SIZE = 42.0`
- grass texture FDID `187126`
- repeated UV scale `9.0`
- vertical offset `0.03`
- independent Bevy `StandardMaterial`

Surrounding ADT chunks use the custom `TerrainMaterial` shader. They are not one continuous ground material, so their lighting and texture treatment can visibly diverge.

## Regression History

`ee3742b6` introduced the patch as part of a character-select floor workaround. `bbaa3e51` removed it explicitly as a “StandardMaterial bright island on TerrainMaterial terrain.” `d335cd0c` re-added `spawn_focused_ground_patch` while addressing warnings. The current screenshot is that known bright-island symptom.

The custom terrain path cannot be declared absent: it loaded chunks and textures in both captures. Static-shadow darkening alone is not enough to explain black-looking terrain because its shader floor is `0.55`; terrain-only spawning also lifts vertex RGB to at least `0.75`. The custom shader’s manually built PBR input differs from Bevy `StandardMaterial`, but this has not been isolated as the cause.

## Next Diagnostic

Before changing rendering, reproduce the canonical capture and compare one variable at a time:

1. Hide only `CampsiteGroundPatch` to measure the real terrain baseline.
2. Capture terrain with its normal custom material versus a controlled `StandardMaterial` diagnostic material on the same mesh.
3. Inspect why the nearby WMO is not displayed and whether it is required for the campsite backdrop.

Do not retain the patch as a terrain-lighting fix. It masks the dark-terrain issue locally and creates a visibly incompatible island.

## Sources

- [background.rs](../../../src/scenes/char_select/scene/background.rs) — patch spawning and material path
- [scene_tree.rs](../../../src/scenes/char_select/scene_tree.rs) — warband terrain spawn
- [terrain.rs](../../../src/rendering/terrain/terrain.rs) — terrain-only vertex-color floor
- [terrain.wgsl](../../../assets/shaders/terrain.wgsl) — custom terrain shading
- [authored-skybox-black-output](authored-skybox-black-output.md) — separate authored-skybox limitations

## See Also

- [[authored-skybox-black-output]] — authored skybox output is a separate issue
- [[terrain-tile-ordering]] — terrain tile loading/ordering
- [[skybox]] — sky and environment-light lifecycle
