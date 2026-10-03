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

- [warband_scene_data.rs](../godot/core/src/warband_scene_data.rs)
- [light_lookup_data.rs](../godot/core/src/rendering/lighting/light_lookup_data.rs)
- [skybox_debug/source.rs](../godot/rust/src/skybox_debug/source.rs)
- [assets/mod.rs](../godot/rust/src/assets/mod.rs) — runtime local-CASC extraction through `asset-resolver`
- [casc_local.rs](/syncthing/Sync/Projects/world-of-osso/asset-resolver/src/bin/casc_local.rs)

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
cargo run -- --screen skyboxdebug --light-skybox-id 653
```

or:

```bash
cargo run -- --screen skyboxdebug --skybox-fdid 5412968
```

The lookup is verified; the retired Bevy renderer output an effectively black frame for this authored skybox in `skyboxdebug`. Godot `skyboxdebug` behavior: [native skybox debug spec](specs/native-skybox-debug.md).

## Current Rendering Failure

Bevy-era record (2026-04-11; the one-shot `screenshot` binary mode was retired with Bevy — with Godot, launch the screen and capture via `target/debug/game-engine-cli screenshot <path>`). The default
warband debug path should no longer be treated as an authored control:

```bash
cargo run --bin game-engine -- --screen skyboxdebug screenshot data/skyboxdebug-default-2026-04-11.webp
cargo run --bin game-engine -- --screen skyboxdebug --light-skybox-id 653 screenshot data/skyboxdebug-light653-2026-04-11.webp
```

Observed results:

- default scene 1 lookup now resolves `data/models/skyboxes/costalislandskybox.m2`
- forced `--light-skybox-id 653` resolves `data/models/skyboxes/11xp_cloudsky01.m2`
- the forced authored path still rendered with a black center pixel: `srgba(0,0,0,0)`
- the earlier default `deathskybox.m2` path came from a global `Light.csv` fallback row and is no longer used for warband scene 1

That means two separate things:

- the lookup chain can still reach a known-authored skybox through `LightSkyboxID 653`
- warband scene 1 was never a trustworthy authored default; it now falls back to the shared campsite skybox instead of a bogus global `deathskybox` result

## Why Some Scenes Still Fall Back

The remaining fallback is no longer a TACT key problem.

Current limitation:

- some warband scenes do not have a local scene-specific `Light.csv` skybox row with a resolvable `LightSkyboxID`
- when that happens, the warband skybox path should stop before treating a global row as authored
- those scenes currently fall back to:

```text
environments/stars/costalislandskybox.m2
```

One current known example:

```text
scene 1
  -> no local scene-specific skybox Light.csv row with a resolvable LightSkyboxID
  -> global Light.csv row exposed LightParamsID 3
  -> that mapped to deathskybox.m2, but was not actually campsite-authored
  -> fallback skybox
```

## Temporary Behavior

- Char select and `skyboxdebug` both use the same authored lookup path first.
- If authored lookup fails, they fall back to the shared WoW skybox model above.
- The renderer now uses the dedicated `SkyboxM2Material` path for authored lookups. It disables depth writes/comparison and shadow/prepass participation, and should be treated as the default skybox render mode for current debugging and char-select validation.

## Next Work

- tighten `Light.csv` to `LightParams.db2` mapping for unresolved scenes
- expand authored row coverage beyond the currently verified path
- tune skybox depth and fog behavior against reference clients
- later, reuse active light-volume-driven skybox selection for in-world scenes and in-world debug scenes
