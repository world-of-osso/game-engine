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

## Fallback Behavior

When `Light.csv → LightParams.db2` mapping fails (some warband scene LightParamsIDs have no matching row in the local modern DB2), the scene falls back to:
```
environments/stars/costalislandskybox.m2
```

Known example: scene 4 uses LightParamsID 6577, which has no matching local LightParams.db2 row.

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
