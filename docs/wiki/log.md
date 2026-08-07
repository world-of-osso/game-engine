# Wiki Log

Chronological record of wiki operations.

## [2026-08-07] update | Document IPC performance diagnostics

Updated `AGENTS.md` and `investigations/procedural-cloud-regeneration.md` for commit `9003b421`: `game-engine-cli performance` reports `fps`, `frame_time_ms`, and `focused`; screenshot `FPS: 1.00` overlays are capture-frame artifacts, not timing evidence.

## [2026-08-07] investigation | Remove synchronous procedural cloud regeneration

Updated `systems/rendering-pipeline.md` and added `investigations/procedural-cloud-regeneration.md` for commit `b2b07e5b`: 512×1024 six-octave cloud textures regenerated synchronously every five seconds, with profiler self samples placing about 60% of sampled CPU in simplex cloud functions. Shader UV/time scrolling already animates clouds, so runtime regeneration was removed while preserving the three startup textures and visual settings. Clean user baselines are 27.98 FPS focused and 27.89 FPS unfocused; no post-fix improvement is claimed yet. IPC screenshots can show a transient 1.00 FPS overlay and are not valid FPS evidence; capture does not leave screenshot entities in the scene.

## [2026-04-30] update | Add Scenemachine M2 loading reference

Updated `reference/open-source-wow-clients.md` with Scenemachine as a C# reference for loading M2 scene/model data.

## [2026-04-09] ingest | Initial bulk ingest of 32 existing docs

Ingested all existing documentation from `docs/` into the wiki structure. Created pages across systems/, formats/, investigations/, design/, and reference/ categories.

## [2026-04-11] update | Document authored skybox black-output repro

Added `investigations/authored-skybox-black-output.md`, updated `systems/skybox.md`, and recorded the current `skyboxdebug` repro showing effectively black output for both default authored lookup and forced `LightSkyboxID 653`.

## [2026-04-21] update | Document LightParams sky-affecting flag composition

Updated `systems/skybox.md` with the implemented `LightParams::Flags` contract (`DontInheritSkybox`, `HideSun`, `HideMoon`, `HideStars`, `HideCelestialObject`, `OverrideCelestialSphere`, `HeightFogAbovePlane`) and how those flags now alter `skyboxdebug` procedural baseline/fog composition.

## [2026-04-21] update | Trace modern authored skybox shader/effect path

Updated `investigations/authored-skybox-black-output.md` with a detailed trace for `11xp_cloudsky01.m2` modern shader batches (`0x4014`, `0x8012`, `0x8016`), including stage binding, combine-mode routing, UV mode mapping, and the current WGSL combine-coverage gap for `0x8012`/`0x8016`.
## [2026-05-01] update | Document direct DB2 CASC access

Updated [[db2-format]] and [[asset-pipeline]] to record that DB2 bytes can be read directly from CASC via `AssetResolver::resolve_bytes`, with `ensure_db2_path` as a cache/debug path. Added `Frostshake/WDBx` as external verifier/export tooling rather than a runtime dependency.
