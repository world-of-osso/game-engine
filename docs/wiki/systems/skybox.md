# Skybox

Skybox rendering combines a procedural dome for explicit LightParams rows with no authored skybox and authored WoW M2 models for rows that select one. The `SkyboxM2Material` render path disables depth writes and shadow/prepass participation. Authored sky fragments use far background depth while retaining scene-depth comparison.

## Authored sky behind scene geometry

The [authored skybox depth contract](../../specs/authored-skybox-depth.md) requires scene objects to occlude authored sky regardless of the sky mesh's finite dimensions. `42ed3cec` writes reverse-Z far depth from the sky fragment shader; texture combining, opacity, and layer ordering remain unchanged.

The GPU regression reproduces the old error with sky geometry closer to the camera than opaque and masked foreground cards. After correction, red foreground and green foliage remain visible while blue sky survives in unobstructed pixels and cutout holes, for opaque and blended sky pipelines. Existing material tests cover the retained settings and ordering.

The gray area in the character-selection depth diagnostic is not proof of missing sky. A terrain-only magenta override identified substantial gray coverage as opaque fogged terrain; it did not account for every gray pixel. Rendering sky behind that terrain must not paint over it. Fog-color/distance tuning is outside this depth correction. Evidence: `data/diagnostics/charselect-tree-clipping-20260911/{fix-red,fix-green,terrain-coverage-probe,verification,final-native}/`.

## Character-selection fog visibility

`dfc34aa4` replaces portrait-camera-relative fog with world-space falloff from 75 to 300 units. Previously the 6.5-unit solo camera made fog fully opaque at 32.5 units, inside the nearby tree area. These new distances are explicit presentation tuning, not decoded Blizzard fog data. Camera framing no longer changes the fog range; global sky updates still exclude character-selection cameras.

Following the user's WoW reference, `d3ddc0c5` caps fog color mixing at 50% and uses a blue-green haze color. Bevy's existing fog RGB blend retains distant surface detail; geometry opacity and depth are unchanged. The terrain-to-sky fade proposal was not implemented, and diagnostic camera changes were restored.

The [visibility contract](../../specs/character-selection-visibility.md) verifies clear nearby trees, partial distant fading with retained detail, and retained fog ownership. Thirty-three focused cases and checks pass. The bounded `opacity-native` reference audit confirms improved distant readability and haze direction without claiming pixel-perfect WoW lighting/assets or cloud restoration. A controlled fog-disabled capture reveals opaque cliff/terrain behind the campsite: reducing fog does not reveal clouds through that geometry. Skybox depth ordering remains intact. Evidence: `data/diagnostics/charselect-fog-20260911/`.

## InWorld procedural sky selection

InWorld resolves the local clear `LightParamsID` from `Light.csv`. It spawns the existing camera-child procedural `SkyDome` only when the decoded local `LightParams` row explicitly has raw `LightSkyboxID = 0`. It does not treat missing DB2 data, an unknown row, or a failed authored model load as permission to fall back; those remain diagnosable failures.

The live Azeroth reproduction on September 10, 2026 selected map 0 Light row 1 at Bevy `[-8977.593, 81.04212, 179.76495]`, whose clear `LightParamsID` is 12. Local DB2 decoding confirmed `LightParams 12 → LightSkyboxID 0`; that is a procedural-sky contract, not an authored-M2 lookup failure. Commit `ec826ee7` had removed normal InWorld dome spawning on April 12, 2026, leaving only the dark-navy clear color. Commit `21feec27` restored the existing dome lifecycle. Commit `58d4b12a` then corrected its interior visibility: Back culling requires inward triangle winding. It also updates newly added dome materials when settled `GameTime` would otherwise skip color propagation.

## Procedural cloud continuity

`389e0185` removes precision-block artifacts and repeat-edge seams from the generated cloud map. The prior generator mixed high-bit seeds into floating coordinate offsets around 45–61 million; at that magnitude `f32` spacing is 4, collapsing nearby lattice samples. It also generated a nonperiodic image while the sampler repeated it. The replacement uses periodic integer-hashed gradient fBm, preserving dimensions, frequency controls, density shaping, and deterministic seeded variation.

`077599df` corrects the shader-side longitude wrap: the primary spherical coordinate remains unwrapped until repeat sampling, and the second layer uses an integer longitude frequency of 2.0 rather than fractional 1.9 after a pre-scale `fract`. This prevents a seam at the spherical longitude boundary. RED observed a 28-level GPU discontinuity; corrected rendered proof remains pending.

## InWorld environment lighting

Authored skybox geometry does not initialize scene IBL. `initialize_inworld_camera_ibl` runs independently of skybox visuals, only in InWorld with the Lighting stage enabled. Active `WowCamera` entities lacking both `GeneratedEnvironmentMapLight` and `EnvironmentMapLight` receive the existing generated-map setup at intensity300, using current interpolated `LightKeyframes` colors. The component filters make initialization idempotent and preserve explicit environment overrides.

This repairs the missing setup connection: the live world camera had no environment component or `SkyEnvMapHandle`, while global ambient was zero. It does not change global ambient, exposure, directional light, shadows, fog, or authored shader-combine behavior. The registered-system fixtures check the source cubemap/component contract; native brightness and GPU filtering still require separate observation.

Contract and fixtures: [InWorld scene spec](../../specs/inworld-scene-isolation.md#inworld-environment-lighting), `src/rendering/skybox/tests/inworld_ibl.rs`.

## Light.csv Lookup Chain

```
WarbandScene position
  → Light.csv row selection
  → LightParamsID
  → LightParams.db2 (field: LightSkyboxID)
  → LightSkybox.db2 (field: SkyboxFileDataID)
  → community-listfile.csv → authored .m2 path
  → CASC extraction → data/models/skyboxes/
```

Relevant code: `src/warband_scene.rs`, `src/light_lookup.rs`, `src/asset/casc_resolver.rs`.

`Light.csv` does not provide a bag of interchangeable skybox candidates. The
resolver now treats the `LightParamsID_*` columns as authored circumstances and
uses `LightParamsID_0` (clear, above-water) by default instead of scanning for
the first slot that happens to resolve a `LightSkyboxID`.

## Standalone skybox-debug color updates

`SkyboxDebug` also spawns the procedural baseline dome when its selected authored-skybox flags allow it. Before `83cf11ec`, shared sky-color and environment updates ran only for InWorld and CharSelect, leaving that standalone dome at default-white uniforms. `SkyboxDebug` now participates in the same update predicate, so its existing dome initializes from `LightKeyframes` and refreshes when game time changes.

The registered-system regression covers initial color propagation and a later time refresh. It does not prove rendered pixels; standalone visual capture remains pending. This screen still does not select Azeroth's procedural-only `LightParams 12` path: that lookup is exercised only by InWorld.

## LightSkybox Flags

`LightSkybox.db2` carries more than just the skybox FDID. The current debug-path reading is:

- `field[1]` = `LightSkybox::Flags`
- `field[2]` = `SkyboxFileDataID`

The engine now decodes these flags in `light_lookup.rs` and uses them in `skyboxdebug` default mode:

- `CombineProceduralAndSkybox` keeps the procedural sky dome visible alongside the authored skybox
- `ProceduralFogColorBlend` keeps distance fog visible in the debug scene

## LightParams Flags

`LightParams.db2` also carries sky-affecting flags that alter how authored skybox + procedural baseline compose.

The engine now decodes `LightParams::Flags` from the same layout hash used for local data (`0xCAE394E7`) and applies the following in `skyboxdebug` default mode:

- `DontInheritSkybox` suppresses procedural celestial baseline for authored composition, and blocks fallback skybox display when the local clear-slot params explicitly request no inherited skybox.
- `HideSun`, `HideMoon`, `HideStars`, `HideCelestialObject`, `OverrideCelestialSphere` suppress procedural celestial baseline so authored skybox output is not mixed with hidden procedural celestial visuals.
- `HeightFogAbovePlane` can force procedural fog visibility even when `LightSkybox::ProceduralFogColorBlend` is unset.

Implemented bit names follow wowdev `DB/LightParams` + `EnumeratedString` enum `Unknown_385`.

Verified fixture:

```
LightSkyboxID 653
  → flags 0b01111
  → SkyboxFileDataID 5412968
  → environments/stars/11xp_cloudsky01.m2
```

The debug screenshot regression for this path should use alive-scene skyboxes only. `deathskybox.m2` is not a valid control for normal alive-scene sky validation.

## Verified Path

```
LightParamsID 5615
  → LightSkyboxID 653
  → SkyboxFileDataID 5412968
  → environments/stars/11xp_cloudsky01.m2
```

Force this path for debugging:
```bash
cargo run --bin game-engine -- --screen skyboxdebug --light-skybox-id 653
# or
cargo run --bin game-engine -- --screen skyboxdebug --skybox-fdid 5412968
```

## Known Issue

`skyboxdebug` currently resolves authored skyboxes correctly but still renders an effectively black frame, including the known-good `LightSkyboxID 653 -> 11xp_cloudsky01.m2` override. This is separate from the corrected ordinary InWorld procedural-dome omission. See [[authored-skybox-black-output]].

## Material animation updates

Skybox M2 UV offsets and transparency are still evaluated every update to preserve authored animation. Commit `e0aa5809` compares the evaluated values before mutably borrowing `SkyboxM2Material`; static tracks and repeated override times no longer emit asset modification events, while changed UV or transparency values update together. Two behavioral regression tests cover static and animated material cases. No whole-engine CPU claim follows from those tests.

## Fallback Behavior

When a warband scene has no local scene-specific skybox row with a resolvable `LightSkyboxID`, it falls back to:
```
environments/stars/costalislandskybox.m2
```

Known example: scene 1 should now use this fallback instead of treating the global `Light.csv` row that led to `deathskybox.m2` as authored campsite data.

## TACT Key Requirement

`LightSkybox.db2` is encrypted. The required TACT key (`0xD1055199767FB373`) comes from `wowdev/TACTKeys`, not from WoWDBDefs. Loaded from `data/tactkeys/WoW.txt`. Once decrypted, the remaining work is accurate DB2 row/field mapping, not key availability. See [casc-db2-keys.md](../casc-db2-keys.md).

## Next Work

- Tighten `Light.csv` to `LightParams.db2` row mapping for unresolved scenes
- Tune skybox depth and fog behavior against reference clients
- Reuse light-volume-driven skybox selection for in-world scenes

## Sources

- `src/rendering/skybox/inworld_skybox.rs` — explicit procedural-vs-authored InWorld selection and camera-child dome lifecycle.
- `src/rendering/lighting/light_lookup.rs` — raw `LightSkyboxID = 0` predicate from decoded LightParams.
- `src/rendering/skybox/mod.rs` — existing dome construction, sky color updates, and independent camera IBL initialization.
- `src/rendering/skybox/skybox_m2_material.rs — authored UV/transparency evaluation and conditional material mutation.
- `src/rendering/skybox/skybox_m2_material_tests.rs` — static/animated asset-event regression coverage.
- `src/rendering/skybox/tests/inworld_ibl.rs` — registered-system initialization and preservation fixtures.

- [skybox-authored-lookup.md](../skybox-authored-lookup.md) — lookup chain, verified path, fallback behavior
- [casc-db2-keys.md](../casc-db2-keys.md) — TACT key for LightSkybox.db2
- [DB/LightParams (wowdev)](https://wowdev.wiki/DB/LightParams) — LightParams flags notes and bit semantics
- [EnumeratedString (wowdev)](https://wowdev.wiki/EnumeratedString) — enum names for `Unknown_385` (`NoDarkenDepth`, `DontInheritSkybox`, `HideSun`, `HideMoon`, `HideStars`, `OverrideCelestialSphere`, `HeightFogAbovePlane`)

## See Also

- [[rendering-pipeline]] — SkyboxM2Material render flags
- [[asset-pipeline]] — CASC extraction, DB2 decryption
- [[authored-skybox-black-output]] — current authored skybox render failure
- [[procedural-sky-dome-visibility]] — procedural-dome winding and late-material color root cause
