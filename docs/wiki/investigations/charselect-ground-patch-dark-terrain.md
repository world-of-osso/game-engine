# Character-Select Ground Patch and Dark Terrain

The bright campsite island is an explicit 42×42 `StandardMaterial` grass plane over a much darker ADT `TerrainMaterial` scene. It reproduces with the same binary against both canonical and retained-worktree data, so it is neither a registry-UI regression nor explained solely by missing worktree assets. The dark terrain is caused by incorrectly decoded MCNR normals; no production correction has shipped.

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

## Root Cause: MCNR Axis Decode

`parse_mcnr` currently maps raw bytes `[b0, b1, b2]` to Bevy normal `[b2, b1, -b0]`. The dominant upward component is therefore placed on Bevy X, causing PBR direct and ambient lighting to shade the terrain as if its surface normals point sideways.

A standalone verifier reconstructed all 16,384 center-vertex geometric normals from MCVT heights in `2703_31_37.adt` and ranked all 48 signed raw-byte permutations. Current decode mean alignment is `0.089730`; the supported mapping `[b0, b2, -b1]` is `0.997198` overall and `0.995139` across 9,409 sloped centers. The next-best mapping is `0.696836`.

Isolated shader captures corroborate the geometry result: terrain is bright before PBR lighting, dark after lighting, and brightens when the lighting normal is forced upward. The forced normal is diagnostic only; it discards slope information and is not a valid fix.

No parser or shader change has been made. Add parser regressions from the verified asset and a rendered terrain check before replacing the decode. Do not retain the grass patch as a terrain-lighting fix: it bypasses the broken terrain normal/material path and creates a visibly incompatible island.

## Sources

- [background.rs](../../../src/scenes/char_select/scene/background.rs) — patch spawning and material path
- [scene_tree.rs](../../../src/scenes/char_select/scene_tree.rs) — warband terrain spawn
- [terrain.rs](../../../src/rendering/terrain/terrain.rs) — terrain-only vertex-color floor
- [parsing.rs](../../../src/asset/adt_format/adt/parsing.rs) — current MCNR decode
- [terrain.wgsl](../../../assets/shaders/terrain.wgsl) — custom terrain shading
- `data/diagnostics/charselect-ground-20260911/verified-root-cause.md` — geometry and shader-probe evidence
- [authored-skybox-black-output](authored-skybox-black-output.md) — separate authored-skybox limitations

## See Also

- [[authored-skybox-black-output]] — authored skybox output is a separate issue
- [[terrain-tile-ordering]] — terrain tile loading/ordering
- [[skybox]] — sky and environment-light lifecycle
