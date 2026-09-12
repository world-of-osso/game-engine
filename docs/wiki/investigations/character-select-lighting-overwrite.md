# Character-Select Lighting Overwrite

Verified: 2026-09-12. A read-only headless probe established the overwrite boundary. Follow-up asset validation established the required ownership boundary: fake character-select directional lights must be removed, while authored M2 point lights must be retained through attachment paths. Neither establishes Retail brightness values.

## Verified behavior

`OnEnter(CharSelect)` creates `TerrainFillLight` at 35,000 lux and `CampfireLight` at 12,000 lux. The first `SkyPlugin` update then changes both to 1,000 lux, the same cool sRGB color `[0.5457181, 0.61576194, 0.6768209]`, and the same rotation. Global ambient remains `[0.92, 0.80, 0.60]` at brightness 150.

The scene dump reports stored `NodeProps::Light` setup metadata, not live `DirectionalLight` components, so its 35,000/12,000 values do not disprove the overwrite.

Temporary diagnostic test-source inclusion was restored after the probe. No production rendering setting changed. The live client was not restarted, signaled, attached to, or queried through IPC; only its executable identity was read.

## Root-cause scope

The earlier probe described the faulty setup accurately but did not justify exempting its fabricated lights from sky updates. The campsite census found zero embedded lights in all 55 loaded M2 models, including campfire `4182539`; environment lighting is still required. The replacement contract is one explicitly sky-owned environmental sun plus generated camera IBL, with sky updates restricted to that sun. Unrelated directional lights and M2 point lights remain outside sky ownership.

The M2 parser correction `5dc4386f` proves modern light records are 156 bytes: a 16-byte prefix plus seven 20-byte animation tracks. Actual cauldron `4238519` retains two type-1 lights and lanterns `5149702`/`5140152` retain four each. The cauldron has both a 3333ms model-local intensity track and a 3333ms global-sequence intensity track, so attachment code cannot sample time zero forever.

## Boundaries

Image ROI review rejected the candidate pairs, so numerical image-derived EV or exposure estimates are not valid. The local data does not demonstrate an exact Retail Adventurer's Rest light record or a map-inheritance path. No exact Retail setting or brightness match is claimed. Attachment retention, animated-light timing, environmental-sun ownership, and rendered validation remain pending at this record.

## Sources

- `data/diagnostics/lighting-comparison-20260912/report.md` — probe result, scope, and rejected comparison boundary.
- `data/diagnostics/lighting-comparison-20260912/probe.stderr` — emitted setup, post-update, and ambient values.
- `data/diagnostics/m2-lighting-authority-20260912/loaded-model-light-counts.json` — campsite-model light census.
- `data/diagnostics/m2-lighting-authority-20260912/parser/report.md` — asset-backed 156-byte parser RED/GREEN and animation census.
- `data/diagnostics/m2-lighting-root-fix-20260912/test-red/result.json` — attachment and ownership regression RED.

## See Also

- [[character-select-waterfall-loading]] — visible waterfall is accepted; overall brightness remains separate.
- [[charselect-ground-patch-dark-terrain]] — terrain normal and material response are distinct factors.
- [[skybox]] — environmental sun and camera IBL ownership.
