# Skybox

Skybox rendering uses authored WoW sky M2 models resolved through a DB2 lookup chain. The `SkyboxM2Material` render path disables depth writes, depth comparison, and shadow/prepass participation.

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

`skyboxdebug` currently resolves authored skyboxes correctly but still renders an effectively black frame, including the known-good `LightSkyboxID 653 -> 11xp_cloudsky01.m2` override. See [[authored-skybox-black-output]].

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

- `src/rendering/skybox/mod.rs` — independent camera IBL initialization and existing cubemap setup.
- `src/rendering/skybox/skybox_m2_material.rs` — authored UV/transparency evaluation and conditional material mutation.
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
