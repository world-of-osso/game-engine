# Char Create 3D Investigation — 2026-04-11

## Brief

- `--screen charcreate` still shows UI but no visible 3D scene in this environment.
- This is **not** explained by the bloom + point-light bug: local `bloomEnabled` is `false`.
- This is **not** just a char-create bug: `--screen debugcharacter` also renders black while model/geoset setup logs look correct.
- This is **not** a screenshot timing issue: a later IPC screenshot from a running char-create instance is still black.
- Going through `--screen charselect` and then entering char create makes the 3D model render, which rules out a renderer/GPU compatibility problem.
- Current best read: this is a direct-entry scene wiring / initialization-order bug. Some prerequisite established on the `CharSelect -> CharCreate` path is missing when starting directly in `CharCreate` or other direct-entry 3D debug screens.

## Issues

1. **Char select black screen** — FIXED
2. **Char create model not switching on race change** — open
3. **Char create standalone (`--screen charcreate`) model not visible** — open

## Char Select Black Screen (fixed)

**Root cause**: `spawn_sky_dome` inserted `DistanceFog` and `GeneratedEnvironmentMapLight` on the camera, overwriting char select's fog and killing PBR rendering.

**Fix**: Use `spawn_sky_dome_entity` (dome mesh only) instead of `spawn_sky_dome`. Insert `SkyEnvMapHandle` resource separately for terrain shader ambient lighting. Replaced 220k SpotLight campfire with directional warm fill. Removed `CampsiteGroundPatch` (`StandardMaterial` bright island on `TerrainMaterial` terrain).

**Commits**: `2d02c9d`, `cad9794`, `bbaa3e5`

## Char Create Race Switch (open)

**Symptom**: Clicking a race button highlights it in the UI (proves `CharCreateState.selected_race` updates) but the 3D model doesn't change.

**What works in tests**:
- `CharCreateAction::parse("select_race:2")` → `SelectRace(2)` ✓
- `apply_race_change` updates `CharCreateState.selected_race` ✓
- `sync_model` detects `displayed.race != state.selected_race` and respawns ✓
- Full model despawn + respawn with new meshes ✓
- `app.update()` with `UiPlugin` + `CharCreatePlugin` processes the click ✓
- Race button has `onclick`, is `mouse_enabled`, is hittable, survives `screen.sync` ✓

**What doesn't work**: The runtime. Every isolated test passes. The full production app doesn't trigger `sync_model` after a race click. The disconnect between test and runtime is unresolved.

**Likely area**: System scheduling — `char_create_update_visuals` (which calls `screen.sync`) and `char_create_mouse_input` run unordered in the same Update set. `screen.sync` may rebuild frames mid-frame, or a different plugin in the full stack interferes.

## Char Create Standalone Model (open)

**Symptom**: `--screen charcreate` shows UI but no 3D model. Screenshots at 10 and 60 frames both empty.

**What works**:
- `setup_scene` runs and spawns 2 models (male+female) ✓
- `sync_appearance` runs and un-hides geoset meshes (Mesh[1], [2], [6], [17] visible at runtime) ✓
- Camera at correct position, model at origin, ray-AABB intersection proves model in view ✓
- Test with `CharCreateScenePlugin` + 3 `app.update()` cycles shows visible geosets ✓

**What doesn't work**: The 3D content doesn't render in the full production app despite entities being correctly set up. The IPC `dump-tree` confirms visible meshes exist. The ground plane (`StandardMaterial`) also doesn't render.

**Likely area**: Missing rendering prerequisite in standalone mode. Char select → char create works because char select's sky dome setup leaves behind `GeneratedEnvironmentMapLight` or other PBR state. Adding `GeneratedEnvironmentMapLight` to the char create camera didn't fix it (tested). The `SkyEnvMapHandle` resource is inserted. Something else from the full rendering pipeline is needed that only gets initialized when passing through another state first.

## Takeover Notes — 2026-04-11

Additional evidence gathered after the original write-up:

- **Bloom is not the cause for this repro**: local `/home/osso/.config/game-engine/options_settings.ron` has `bloomEnabled: false`, so the standalone `--screen charcreate` black 3D output persists with bloom disabled.
- **Not a screenshot timing issue**: launching `--screen charcreate`, waiting for CASC extraction to finish, then taking a later IPC screenshot still produces the same black 3D output.
- **Broader than char create**: `--screen debugcharacter` also renders black in the same environment while its setup logs and customization logs show the model and visible geosets are present.
- **UI is not covering the model**: `CharCreateBackground` is transparent, and the same black result appears in `debugcharacter`, which does not use the char-create UI.
- **Full sky bootstrap was tested and did not change the result**: wiring char create to use `spawn_sky_dome` exposed that `ProceduralCloudMaps` is missing on direct initial-screen entry; adding a fallback to create those maps allowed the sky bootstrap to run, but the 3D screenshot remained black.

**Updated conclusion**: the renderer is not the problem. Going through `CharSelect -> CharCreate` works, which proves the direct-entry path is missing setup that the char-select path establishes first. Treat this as a direct-entry wiring / initialization-order bug until proven otherwise.

## Files Changed

- `src/scenes/char_select/scene/setup.rs` — sky dome fix, character position fix
- `src/scenes/char_select/scene/lighting.rs` — directional campfire, tuned ambient/fill
- `src/scenes/char_select/scene/background.rs` — removed ground patch
- `src/scenes/char_select/scene_tree.rs` — label fixes
- `src/scenes/char_create/scene.rs` — `ensure_sky_env_map`, tests
- `src/rendering/skybox/mod.rs` — `pub(crate)` for `spawn_sky_dome_entity`, `build_sky_cubemap`, `insert_default_sky_env_map`
