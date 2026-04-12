# Skybox

Skybox rendering uses authored WoW sky M2 models resolved through a DB2 lookup chain. The `SkyboxM2Material` render path disables depth writes, depth comparison, and shadow/prepass participation.

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

## LightSkybox Flags

`LightSkybox.db2` carries more than just the skybox FDID. The current debug-path reading is:

- `field[1]` = `LightSkybox::Flags`
- `field[2]` = `SkyboxFileDataID`

The engine now decodes these flags in `light_lookup.rs` and uses them in `skyboxdebug` default mode:

- `CombineProceduralAndSkybox` keeps the procedural sky dome visible alongside the authored skybox
- `ProceduralFogColorBlend` keeps distance fog visible in the debug scene

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

- [skybox-authored-lookup.md](../skybox-authored-lookup.md) — lookup chain, verified path, fallback behavior
- [casc-db2-keys.md](../casc-db2-keys.md) — TACT key for LightSkybox.db2

## See Also

- [[rendering-pipeline]] — SkyboxM2Material render flags
- [[asset-pipeline]] — CASC extraction, DB2 decryption
- [[authored-skybox-black-output]] — current authored skybox render failure
