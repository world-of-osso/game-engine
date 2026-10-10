# WMO minimap geometry

Verified: 2026-10-09. **Phase 1 only; indoor minimap implementation remains blocked.** A 128-world-unit block, 2 source texels/unit and group-bbox minimum origin aligns Stockade and two additional indoor WMOs. It does not explain all six requested groups of WMO 16156. Modern split groups explain part of that discrepancy, but not a complete tile-selection/coordinate rule. Retail Mainline remains the behavioral authority; an exporter implementation is evidence to test, not a replacement authority.

## Scope and evidence

Branch `wmominimap2`, slot `/home/osso/.worktrees/game-engine-wmomap`, started from `origin/master`. No client/server code edits, builds, tests, native launches or UDP listeners. No package/system changes. All evidence is uncommitted under:

`data/diagnostics/wmominimap-2026-10-09/`

The slot's data directories link to canonical data. No evidence was put in AgentShared: the requested native `indoor.png` would misrepresent an unimplemented feature.

| WMOID | Root FDID / listfile name | Sampled groups | Tiles |
|---|---|---|---|
| 723 | 108631 / `dungeon/az_stormwindprisons/stormwindjail.wmo` | 0–25 | 26 local tiles |
| 16156 | 5356285 / `expansion10/delves/11xp_arathorzealots01.wmo` | 0, 2, 8, 10, 16, 17 | 34 tiles in sampled groups; 35 total local rows |
| 55 | 106899 / `azeroth/buildings/human_barn_silo/barn.wmo` | 0, 1 | 2 newly extracted tiles |
| 1043 | 110917 / `dungeon/md_hive/mini-hive_d.wmo` | 1–6 | 6 newly extracted tiles |

Thus the brief's “cave 16156” is specifically the Arathor-zealot delve asset, not an unidentified generic cave. Each sampled group's MOGP has indoor bit `0x2000` set. Root 55 group 2 and root 1043 group 0 have no selected DB2 minimap rows and were not included in the fit. The two additional WMO tile extractions used existing `casc-local`, agent slice `wmominimap2`, local `/syncthing/World of Warcraft`, not CDN. `extract-55-1043.log`: 8 extracted, 0 failed.

Local `.build.info` identifies Retail `12.1.0.69933`, build key `dcfc90fffd79ba00406ae46f5f657592`; input CSV is `data/db2/12.1.0.69933/WMOMinimapTexture.csv`. A fresh local extraction of all 59 selected roots/groups was byte-identical to the existing model inputs (`fresh-model-comparison.json`, `fresh-extraction.log`). This rules out a stale *local model copy* for this sample; it does not prove the DB2 rows' art was authored against the same grouping.

## Primary-source search

| Source, pinned revision | Result |
|---|---|
| Retail cached `Blizzard_Minimap/Mainline/Minimap.lua` and API documentation | Lua delegates rendering/zoom to the native minimap; no WMO block-size/origin formula found. Housing's static overlay is unrelated to WMO floor tiles. |
| Kruithne/wow.export `c2fd7bde36a712be78a5da896c995b84fbfa2545`, `src/js/wmo-minimap.js` | Explicit implementation: MOGI minimum X/Y times 2; block pitch 256 pixels; image Y inverted; smaller BLPs bottom-left anchored without resizing. This supplies the principal hypothesis. |
| WebWowViewerCpp `1a8cccbeffc46231c6497e6b3f5bfbf3507d8071` | `wmoFileHeader.h` identifies MOGP split parent/child flags and indices. `wmoObject.cpp:934–938,1069–1085` maps a split child to its parent for portal traversal and follows the child chain. MAVG is a colored ambient volume, not minimap coordinates. No WMOMinimapTexture coordinate implementation found in the targeted scan. |
| noggit3 `59e58add868608339fe292970196985d4ce050ff`; solarityclient `f5f5f4a81e5c11241f4c80c117e5dfe9b587dcee` | Noggit's minimap widget is terrain/editor mapping. Solarity's `world-minimap.md` explicitly leaves WMO tiles for future runtime resolution; its old-client zoom evidence is not Retail Mainline geometry authority. Neither supplied the missing modern rule. |
| WoWDBDefs `WMOMinimapTexture.dbd`; wowdev WMO page | DBDefs defines columns/layouts and sparsity, not texel geometry. wowdev direct fetch returned HTTP 403; not counted as inspected authoritative text. wow.export's MOMO handling is an alpha-format wrapper, not a modern minimap transform. |

Search results and precise repository SHAs/line matches are preserved in `source-scan.json`. Negative results describe this bounded search, not proof that no implementation exists elsewhere.

## Principal candidate transform

In WMO-local raw MOVT axes, X increases image-right and Y increases image-up. Let `(xmin,ymin)` be the group's MOGI minimum, `bx/by` the DB2 block indices, and `u/v` native BLP texel coordinates, with `v` measured down from the top of an image of height `H`:

```text
local_x = xmin + 128 * bx + (u + 0.5) / 2
local_y = ymin + 128 * by + (H - v - 0.5) / 2
```

The fixed block is 256 source pixels, but the BLP need not be 256×256. A 128×64 BLP occupies 64×32 world units at the block's bottom-left, **not** 128×128 units stretched to the cell. Stockade group 1 supplies exactly such a tile (FDID 211701). Stockade group 0 is 256². Delve one-block groups 8/16/17 are 256×128; group 10 is 256×64. Their texel sizes do not independently explain their much larger current group bounds.

MOGI and MOGP bounding boxes are equal for the sampled groups. Consequently this sample cannot distinguish those two origin sources. This is a local-coordinate hypothesis only; no placed-WMO/world-space transform has been implemented or accepted.

## Numerical study

`study.py` decodes BLP2 DXT1 locally using standard-library code, parses root MOHD/MOGI/GFID and group MOGP/MOVT/MOVI, and retains triangles whose normalized signed Z normal is greater than 0.65. It rasterizes their top-down coverage and compares it against decoded nontransparent, non-near-black art (`max(R,G,B)>8`). Both masks use 4× downsampled source pixels, sampled at each 4×4 cell's center. All 40 groups have floor, decoded tile and overlay PNGs.

The overlap metric is intersection-over-union (IoU); **error = 1 − IoU**. Higher overlap is better. It is not a Retail pixel-parity oracle: floor-filter exclusions, painted margins, upper floors, water, doodads and lighting affect the art mask. The concrete Stockade fits provide a positive control for the parser, decoder and candidate transform.

### Same rule across WMOs

| WMO | Common candidate IoU range | Mean IoU | Mean error | Interpretation |
|---|---:|---:|---:|---|
| Stockade 723, 26 groups | 0.8383–1.0000 | 0.9501 | 0.0499 | Strong alignment at 128-unit blocks, group minimum, unflipped local X/right and Y/up |
| Barn 55, 2 groups | 0.8421–0.9815 | 0.9118 | 0.0882 | Same rule; small footprint has only 18 sampled art pixels in group 0 |
| Hive 1043, 6 groups | 0.6986–0.8539 | 0.7610 | 0.2390 | Same scale/origin gives visible correspondence; incomplete floor-mask coverage limits score |
| Delve 16156, 6 groups | 0.0185–0.2187 | 0.0726 | 0.9274 | Same group-index rule fails badly |

`fits.json` contains each group's flags, bounds, FDIDs/native tile dimensions, group SHA-256, triangle/mask counts and all 72 candidate scores. The fixed-grid search tests MOGP, MOGI and root MOHD origins; 128/256/512-unit blocks; both axis orderings and all four flips. It includes clipping at the tile canvas boundaries. `refine.py` additionally samples uniform blocks of 32,48,64,96,128,192,256,384,512,768,1024 units with freely fitted offsets around centroid alignment. Its score penalizes geometry outside the canvas rather than rewarding arbitrary clipping. It also tests independent X/Y bbox stretching to the whole canvas; `crop-check.py` tests stretching into the actual cropped image bounds. These are bounded sampled searches, **not continuous global optima**.

### Delve counterexamples

| Group | XY bbox extent (units) | Common IoU | Best anchored candidate IoU | Free-origin sampled IoU / block / orientation | Best cropped-bbox stretch IoU |
|---|---|---:|---:|---|---:|
| 0 | 90.18 × 33.82 | 0.0185 | 0.0185 | 0.2982 / 32 / no swap, X− Y+ | 0.5334 |
| 2 | 458.30 × 359.22 | 0.0435 | 0.0441 | 0.4350 / 32 / no swap, X+ Y+ | 0.0455 |
| 8 | 149.07 × 317.16 | 0.0466 | 0.2810 | 0.1956 / 256 / swap, X+ Y+ | 0.1413 |
| 10 | 64.79 × 156.93 | 0.2187 | 0.5068 | 0.5395 / 128 / swap, X+ Y− | 0.3482 |
| 16 | 157.13 × 107.87 | 0.0734 | 0.2616 | 0.5319 / 192 / swap, X− Y+ | 0.3101 |
| 17 | 135.42 × 125.29 | 0.0350 | 0.3789 | 0.5308 / 192 / swap, X+ Y− | 0.4434 |

“Best anchored” may clip most of a group's footprint, explaining why it can exceed the free-origin result despite the latter having more parameter choices. The incompatible scales/flips are rejected as a common rule. Fitted offsets in downsampled-pixel coordinates, with coordinate conventions, remain in `refined-fits.json` and `refine.py`; multiplying them by 4 yields source-pixel offsets. These low-overlap parameters are diagnostic candidates, not accepted world origins.

### Split-group discovery

The current delve root has 18 groups. MOGP fields at offsets 60/64/66 reveal:

- Group 2: flags2 `0x40`, first child 17.
- Groups 3–17: flags2 `0x80`, parent 2; linked chain `17 → 16 → … → 3 → −1`.
- Group 0: flags2 0, no parent/child index.

Groups 8/10/16/17 therefore are **split children**, not four independent authored rooms. Their MOGP group ID is 0; parent 2 has group ID 71136. This changes the interpretation of the brief's single-block counterexamples: current spatial-group index and DB2 minimap group number cannot safely be equated solely from this data.

Rendering the union of group 2 and children 3–17 at the parent's minimum and 128-unit pitch produces IoU **0.8511** against group 2's 5×3 tile canvas (error 0.1489), versus 0.0435 for group 2's own MOVT alone. The same union also overlaps group 0's 5×3 canvas at **0.7445** (error 0.2555), although group 0 has no split-parent flag. Root and parent share the XY minimum for this comparison. Using 256/512-unit blocks drops union overlap sharply (`refined-fits.json`).

This supports a split-parent/combined-geometry hypothesis and explains why a root-sized grid appeared plausible. It does **not** establish Retail's tile-family selection for group 0 or its treatment of the one-block rows retained under child numbers. No row alias, stale-authoring claim, dynamic tile regrouping or undocumented fallback was invented. The MOGP indoor bit alone cannot choose between these maps.

## Evidence and reproduction

All commands ran through the existing `scripts/agent/agent-run wmominimap2` wrapper. Research uses no external Python libraries and installs none. From this checkout:

```text
scripts/agent/agent-run wmominimap2 python3 data/diagnostics/wmominimap-2026-10-09/study.py
scripts/agent/agent-run wmominimap2 python3 data/diagnostics/wmominimap-2026-10-09/refine.py
scripts/agent/agent-run wmominimap2 python3 data/diagnostics/wmominimap-2026-10-09/crop-check.py
```

These are CPU data-analysis commands, not builds/test suites. Each exited 0; saved logs contain outputs. No build lock was taken for non-build work, and no build/test invocation occurred.

Per-group PNG names are `<WMOID>-g<group>-tiles.png`, `-floor.png`, `-fit.png` and `-common.png`. Delve groups 0/2 also have `-split-union.png`. Floor panels independently normalize the group's bbox and are not overlay coordinates. Fit/common/union panels use the tile canvas: green = overlapping floor/art, red = floor outside art; untouched art reveals unmatched tile coverage. Tile/overlay canvases are already downscaled 4×. Personally inspected Stockade group 1 fit, delve group 0 tiles and group 8 tile/floor images; the latter visibly have incompatible footprints.

Machine-readable evidence: `fits.json`, `refined-fits.json`, `cropped-bbox-fits.json`, `roots.json`, `listfile-names.txt`, `source-scan.json`, `fresh-model-comparison.json`, plus extraction/analysis logs. Diagnostic scripts and images are intentionally not committed. `evidence-manifest.json` hashes the saved inputs/results/images so later mutation is detectable.

## Decision and remaining boundary

**No rule is validated for ALL requested groups. Stop after phase 1.** The useful result is the concrete legacy-style transform plus the modern split-group mismatch, not a fabricated indoor minimap feature.

Next research needs authoritative Retail evidence for mapping containing split/unsplit groups to the minimap tile family, especially delve group 0 and children with independent DB2 rows. Geometry should be checked after that selection is established; fitting the wrong group's art cannot establish texel math. Phase 2 indoor detection, world-position math, composition, zoom switching, TDD and native Stockade capture remain unimplemented/unproved. Native budget used: 0 of 3.

## Sources

- [Minimap contract](../../specs/minimap.md#indoor-wmo-map-unimplemented) — requirements and previous blocker.
- [Reference catalog](../reference/open-source-wow-clients.md) — source selection.
- [wow.export minimap implementation](https://github.com/Kruithne/wow.export/blob/c2fd7bde36a712be78a5da896c995b84fbfa2545/src/js/wmo-minimap.js) — candidate transform and cropped-tile anchoring.
- [WebWowViewerCpp WMO header](https://github.com/Deamon87/WebWowViewerCpp/blob/1a8cccbeffc46231c6497e6b3f5bfbf3507d8071/wowViewerLib/src/engine/persistance/header/wmoFileHeader.h) and [traversal](https://github.com/Deamon87/WebWowViewerCpp/blob/1a8cccbeffc46231c6497e6b3f5bfbf3507d8071/wowViewerLib/src/engine/objects/wmo/wmoObject.cpp) — split fields and parent/child handling, not minimap selection.
- [WoWDBDefs schema](https://github.com/wowdev/WoWDBDefs/blob/master/definitions/WMOMinimapTexture.dbd) — schema only; no spatial equation.

## See Also

- [[minimap]] — existing outdoor/native implementation.
- [Local CASC extraction](../../casc-extraction.md) — archive provenance and extraction constraints.
