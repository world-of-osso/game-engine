# Character-Select Lighting Overwrite

Verified: 2026-09-12. A read-only, headless probe establishes that the character-select scene's configured directional-light setup is overwritten by the sky update; it does not establish Retail lighting values or a lighting fix.

## Verified behavior

`OnEnter(CharSelect)` creates `TerrainFillLight` at 35,000 lux and `CampfireLight` at 12,000 lux. The first `SkyPlugin` update then changes both to 1,000 lux, the same cool sRGB color `[0.5457181, 0.61576194, 0.6768209]`, and the same rotation. Global ambient remains `[0.92, 0.80, 0.60]` at brightness 150.

The scene dump reports stored `NodeProps::Light` setup metadata, not live `DirectionalLight` components, so its 35,000/12,000 values do not disprove the overwrite.

Temporary diagnostic test-source inclusion was restored after the probe. No production rendering setting changed. The live client was not restarted, signaled, attached to, or queried through IPC; only its executable identity was read.

## Boundaries

Image ROI review rejected the candidate pairs, so numerical image-derived EV or exposure estimates are not valid. The local data does not demonstrate an exact Retail Adventurer's Rest light record or a map-inheritance path. No exact Retail setting, brightness match, or production correction is claimed.

## Sources

- `data/diagnostics/lighting-comparison-20260912/report.md` — probe result, scope, and rejected comparison boundary.
- `data/diagnostics/lighting-comparison-20260912/probe.stderr` — emitted setup, post-update, and ambient values.

## See Also

- [[character-select-waterfall-loading]] — visible waterfall is accepted; overall brightness remains separate.
- [[charselect-ground-patch-dark-terrain]] — terrain normal and material response are distinct factors.
