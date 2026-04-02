# Authored Skybox Lookup

This note describes how authored WoW skyboxes are currently resolved in `game-engine`, where the lookup still falls back, and which pieces are temporary.

## Lookup Chain

For warband scenes, the current authored lookup path is:

```text
WarbandScene position
  -> Light.csv row selection
  -> LightParamsID
  -> LightParams.db2
  -> LightSkyboxID
  -> LightSkybox.db2
  -> SkyboxFileDataID
  -> community-listfile.csv
  -> authored .m2 path
  -> local CASC extraction into data/models/skyboxes/
```

Relevant code:

- [warband_scene.rs](/syncthing/Sync/Projects/world-of-osso/game-engine/src/warband_scene.rs)
- [light_lookup.rs](/syncthing/Sync/Projects/world-of-osso/game-engine/src/light_lookup.rs)
- [casc_resolver.rs](/syncthing/Sync/Projects/world-of-osso/game-engine/src/asset/casc_resolver.rs)
- [casc_local.rs](/syncthing/Sync/Projects/world-of-osso/game-engine/src/bin/casc_local.rs)

## What Works Today

The current local client data has at least one verified authored path:

```text
LightParamsID 5615
  -> LightSkyboxID 653
  -> SkyboxFileDataID 5412968
  -> environments/stars/11xp_cloudsky01.m2
```

`skyboxdebug` can force this path directly with either:

```bash
cargo run --bin game-engine -- --screen skyboxdebug --light-skybox-id 653
```

or:

```bash
cargo run --bin game-engine -- --screen skyboxdebug --skybox-fdid 5412968
```

## Why Some Scenes Still Fall Back

The remaining fallback is no longer a TACT key problem.

Current limitation:

- some `Light.csv` / `LightParamsID` values used by warband scenes do not appear directly in the local modern `LightParams.db2`
- when that happens, authored lookup stops before `LightSkyboxID` resolution
- those scenes currently fall back to:

```text
environments/stars/costalislandskybox.m2
```

One current known example:

```text
scene 4
  -> LightParamsID 6577 from Light.csv
  -> no matching current local LightParams.db2 row
  -> fallback skybox
```

## Temporary Behavior

- Char select and `skyboxdebug` both use the same authored lookup path first.
- If authored lookup fails, they fall back to the shared WoW skybox model above.
- The renderer currently uses a safe unlit `StandardMaterial` skybox path, not a final WoW-specific shader path.

## Next Work

- tighten `Light.csv` to `LightParams.db2` mapping for unresolved scenes
- expand authored row coverage beyond the currently verified path
- tune skybox depth and fog behavior against reference clients
- later, reuse active light-volume-driven skybox selection for in-world scenes
