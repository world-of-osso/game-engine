# Release FPS reprofile — 2026-11-11

VERIFIED: This reprofile measures a standard Stormwind idle pose and a Ruby Life Pools out-of-combat pose with the pinned release client at 1920×1080. The profiles rank main-thread system medians. A named render-function ranking and a valid same-pose FPS A/B are not available.

## Poses and frame results

VERIFIED: In Stormwind, `Wrffps` stood at WoW `(-8855, 645, 97)`, yaw `3.141593`, at noon. The `idle_systems` segment recorded 5,064 frames, p50/p95/p99 frame times of 4.336/7.836/9.445 ms, and thread-sampler CPU of 3.766 ms/frame on the main thread and 3.727 ms/frame on the render thread.

VERIFIED: In Ruby Life Pools (map 2521), `Rubyfps` stood at client `(1450.0, 330.0121, 295.0)` out of combat. The 2,706-frame segment recorded p50/p95/p99 frame times of 12.437/19.715/23.299 ms, main/render thread-sampler CPU of 8.359/11.225 ms/frame, and 38 mean units.

VERIFIED: The separate Godot render CPU/GPU monitors read 3.014/3.105 ms in Stormwind and 8.555/4.987 ms in Ruby. These monitor values differ from the thread sampler and must not be treated as the same measurement.

UNVERIFIED: Do not attribute the gap between the two pose medians only to scene complexity. Their recorded 1/5/15-minute host loads were 24.21/21.07/24.88 in Stormwind and 28.35/46.43/48.05 in Ruby.

## Main-thread system medians

VERIFIED: The following are the five largest recorded per-system medians for each pose, in ms/frame. These values are not additive and do not rank render-thread functions.

| Rank | Stormwind | ms | Ruby out of combat | ms |
|---|---|---:|---|---:|
| 1 | Culling | 0.397 | Culling | 2.100 |
| 2 | Spell visuals | 0.174 | Spells | 0.206 |
| 3 | Ground detail | 0.131 | Targeting | 0.186 |
| 4 | Spells | 0.129 | Spell visuals | 0.183 |
| 5 | Player input | 0.090 | Player input | 0.161 |

UNVERIFIED: A named top-five render-function ranking is not supported. The Ruby `perf` report lost 10.46% of samples and shows raw Godot offsets instead of usable symbols, so it cannot establish a trustworthy function order.

## Packed surface-stream change and output parity

VERIFIED: Commit `39a6f13e72936b0e5526a2eb5f48666b8990ed84` retains the packed position and index streams used to build M2/WMO `ArrayMesh` surfaces. `RenderSurfaceCache` uses those streams instead of reading the surface arrays back from Godot.

VERIFIED: The baseline and after field-dump directories each contain 25 batch outputs, and all 25 files match byte-for-byte. A sorted filename-and-content aggregate SHA-256 is `d3fae098996ab36a023742596817734433a317ced0b8df8450ce04ed930fa187` for both sets.

VERIFIED, single-run observations: The extended uncapped field fixture recorded first-use times of 62/89/62/57/26 ms before and 53/70/50/47/20 ms after for flat/rug/stairs/wall/training. These are separate fixture runs, not a controlled FPS A/B. The 120-frame fixture passed on both versions. The default 30-frame limit was inconsistent: a baseline control passed, but other baseline and after attempts exceeded it.

## Same-pose FPS A/B status

VERIFIED: A fixed-pose Stormwind pair used the same `Wrffps` spawn to within 0.0001 yd and the same scene counts. Its measured values were:

| Build | p50 frame time | Main-thread CPU | Render-thread CPU | 1/5/15-minute host load |
|---|---:|---:|---:|---|
| Before | 7.533 ms | 6.205 ms | 6.828 ms | 21.18/25.69/24.28 |
| After | 8.759 ms | 7.267 ms | 7.815 ms | 36.58/30.56/26.20 |

UNVERIFIED: This pair does not establish a code effect because host load changed sharply. No Ruby before/after FPS pair was run. A quiet-host, same-pose A/B remains open; do not use the exploratory p50 delta as a performance claim.

## Build checks

VERIFIED: The debug and release extension builds exited successfully, `cargo fmt --all --check` exited 0, and the saved debug build output contains no Rust warning lines. The after release library SHA-256 is `f2364893f5d6dc5fd5b579dd9f068ffcaf9fc16d00421529cbd9493b123df8db`.

## Sources

- `data/diagnostics/fps-reprofile-f4d96c3be-2026-11-11/stormwind-release.log` — Stormwind release segments and system medians.
- `data/diagnostics/fps-reprofile-f4d96c3be-2026-11-11/ruby-party-release.log` and `ruby-party-segments/ooc.json` — Ruby release segment and system medians.
- `data/diagnostics/fps-reprofile-f4d96c3be-2026-11-11/ruby-party-release.perf.data.report.txt` — limited `perf` report.
- `data/diagnostics/fps-reprofile-f4d96c3be-2026-11-11/drape-before-dump.log`, `drape-after-extended.log`, `drape-before-dump-extended/`, and `drape-after-dump-extended/` — field timings and output bytes.
- `data/diagnostics/fps-reprofile-f4d96c3be-2026-11-11/ab-stormwind-before-fixed-segments/` and `ab-stormwind-after-fixed-segments/` — fixed-pose exploratory pair.
- [Render-surface cache](../../../godot/rust/src/render_surface.rs), [M2 mesh builder](../../../godot/rust/src/assets/mod.rs), and [WMO mesh builder](../../../godot/rust/src/wmo/scene.rs).

## See Also

- [[crowd-combat-fps]] — Battle Shout draping and first-use surface costs.
- [[follower-combat-fps]] — Ruby Life Pools profiles.
- [[godot-stormwind-fps]] — Stormwind performance history.
