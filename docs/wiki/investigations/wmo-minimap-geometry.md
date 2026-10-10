# WMO minimap geometry

Verified: 2026-10-09. **Phase 1 only; indoor minimap implementation remains blocked.** A 128-world-unit block, 2 source texels/unit and group-bbox minimum origin aligns Stockade and two additional indoor WMOs. It does not explain all six requested groups of WMO 16156. A follow-up scores all 55 spatial groups (41 with direct DB2 rows): split-family union fits parent 2 at 0.8511, but unsplit indoor group 0 remains at 0.0185. Split topology is established; Retail minimap-family selection is not. Retail Mainline remains the behavioral authority; an exporter implementation is evidence to test, not a replacement authority.

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
| WoWDBDefs `WMOMinimapTexture.dbd`; wowdev WMO page | DBDefs defines columns/layouts and sparsity, not texel geometry. Initial wowdev fetch returned 403; the follow-up successfully inspected WMO revision 37115, including split fields, MOGX, MPY2 and MOVX. These define geometry/topology, not a minimap selection rule. wow.export's MOMO handling is an alpha-format wrapper, not a modern minimap transform. |

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

## Split-family follow-up — 2026-10-09

### What the sources establish

[Wowdev WMO revision 37115](https://wowdev.wiki/index.php?title=WMO&oldid=37115#Split_Groups) documents one-level parent/child topology: flags2 `0x40` marks the parent, `0x80` marks a child; offset `0x40` is the parent's first-child index or the child's parent index, and offset `0x42` chains siblings until −1. Portals connect to the parent, not directly to children. WebWowViewerCpp's `wmoObject.cpp:934–953,1062–1085` implements exactly that **portal-view** substitution and union. Neither source says that Retail minimap rendering performs the same substitution. Applying it to minimaps remains a tested hypothesis, not established Retail behavior.

The same wiki describes MOGX's leading `queryFaceStart` as a per-polygon ground-type offset into MOQG, MPY2 as 16-bit flags/material IDs replacing MOPY, and MOVX as a possible 32-bit index replacement. No minimap origin, scale, group alias or DB2-row validity field is described. All 18 local delve groups use MOVI, not MOVX; every MOVI index is within MOVT and every MPY2 record count matches the triangle count. Their MOGX chunks are **256 bytes**, not just the wiki's documented four-byte value; the leading value is recorded, not reinterpreted as a minimap index. An initial diagnostic failed on `unpack` expecting exactly four bytes, then was corrected to read the leading field with `unpack_from`; both logs are retained.

Additional pinned source checks:

| Source | Bounded result |
|---|---|
| Noggit3 `59e58add868608339fe292970196985d4ce050ff` | Editor/terrain minimap; no split-family WMOMinimapTexture selector found. |
| [Noggit Red](https://gitlab.com/prophecy-rp/noggit-red), `b274edb175af4bf7481c43145d81df361291fd5a` | Followed the Marlamin repository's redirect to the actual source. `WMO.cpp` requires MOPY/MOVI in the old-format loading path; minimap widget selects ADT tiles. No modern split or WMOMinimapTexture mapping found. |
| wow.export `c2fd7bde36a712be78a5da896c995b84fbfa2545` | Minimap layout uses the DB2 GroupNum's MOGI bbox directly; does not normalize children to split parents. Exporter's `split_groups` option means separate OBJ exports, not MOGP split-family selection. |
| WebWowViewerCpp `1a8cccbeffc46231c6497e6b3f5bfbf3507d8071` | Implements split-family portal traversal and MPY2 decoding, not WMOMinimapTexture selection. |
| wowserhq `wowser` `5fcd3e607db7551a74caf6d7ffa4dd785a264dc8`, `blizzardry` `bef2d743bdbbfa66391238368f7274051adc3e66` | Old group loading uses numbered group files; blizzardry reserves the trailing MOGP fields rather than decoding split topology. No minimap-family selector found. |
| wowserhq `scene` `cbe1211ab658965667bd4bcd61561b11a944028e`, `format` `77ba8e03613cfb557f42617582d6536866c529a7` | Current scene has no WMO/map-object render path in the inspected tree; format's `mapobj/io/group.ts` exposes flags2 but reads offset `0x40` as padding2. No split minimap rule found. |

Exact tracked-file/line matches and repository revisions are saved in `split-source-scan.json`; wiki HTML/text are saved locally. These negative findings are bounded source evidence, not a claim about closed-source Retail internals.

### Fixed-rule rerun: every spatial group

`split-fit.py` follows the actual sibling chain, never a guessed numeric range. For a split child it selects the parent's tile rows, parent bbox minimum and parent+children upward-floor union; for a parent it uses that union; otherwise it uses the group's own rows and floor. Block pitch stays 128 units, cropped tile anchoring and floor filter stay unchanged. This is **candidate map selection** only. Child selection does not establish that Retail ignores its child-numbered rows.

IoU below includes projected floor **outside** the canvas in its denominator; no clipping can improve the result. It equals clipped IoU for these family-selected canvases. All 55 spatial groups were evaluated; 41 have their own rows. Each of delve groups 3–17 maps to parent 2 and receives the same single family fit, not 15 independent successful fits.

| WMO | Group | Selected tile group | IoU | Floor mask pixels |
|---|---:|---:|---:|---:|
| Stockade 723 | 0 | 0 | 0.9289 | 706 |
| 723 | 1 | 1 | 0.9638 | 135 |
| 723 | 2 | 2 | 0.9496 | 135 |
| 723 | 3 | 3 | 0.9515 | 100 |
| 723 | 4 | 4 | 0.9706 | 100 |
| 723 | 5 | 5 | 1.0000 | 112 |
| 723 | 6 | 6 | 1.0000 | 112 |
| 723 | 7 | 7 | 0.9825 | 114 |
| 723 | 8 | 8 | 0.9825 | 114 |
| 723 | 9 | 9 | 1.0000 | 112 |
| 723 | 10 | 10 | 0.8992 | 218 |
| 723 | 11 | 11 | 0.9417 | 116 |
| 723 | 12 | 12 | 0.9750 | 117 |
| 723 | 13 | 13 | 0.9750 | 117 |
| 723 | 14 | 14 | **0.8383** | 141 |
| 723 | 15 | 15 | 0.9095 | 221 |
| 723 | 16 | 16 | 0.9087 | 198 |
| 723 | 17 | 17 | 0.9638 | 135 |
| 723 | 18 | 18 | 0.9433 | 136 |
| 723 | 19 | 19 | 0.9528 | 104 |
| 723 | 20 | 20 | 0.9519 | 102 |
| 723 | 21 | 21 | 0.8954 | 219 |
| 723 | 22 | 22 | 0.9339 | 116 |
| 723 | 23 | 23 | 0.9669 | 117 |
| 723 | 24 | 24 | 0.9669 | 118 |
| 723 | 25 | 25 | 0.9515 | 203 |
| 723 | 26 | — | no rows; exterior | — |
| Barn 55 | 0 | 0 | **0.8421** | 17 |
| 55 | 1 | 1 | 0.9815 | 106 |
| 55 | 2 | — | no rows; exterior | — |
| Hive 1043 | 0 | — | no rows; exterior | — |
| 1043 | 1 | 1 | **0.7311** | 261 |
| 1043 | 2 | 2 | **0.7665** | 373 |
| 1043 | 3 | 3 | **0.7302** | 295 |
| 1043 | 4 | 4 | **0.6986** | 197 |
| 1043 | 5 | 5 | **0.7856** | 382 |
| 1043 | 6 | 6 | 0.8539 | 492 |
| Delve 16156 | 0 | 0 | **0.0185** | 432 |
| 16156 | 1 | 1 | **0.2911**; exterior | 607 |
| 16156 | 2 | 2 | 0.8511 | 20929 |
| 16156 | 3–17, each | 2 | 0.8511 | 20929 |

The full 55-row table is also saved in `split-family-table.md`; machine-readable per-group results include own-row fits, floor/art/intersection counts, chunk sizes, tile dimensions and input hashes (`split-family-fits.json`). Each mask pixel is a 2×2-world-unit sample cell. The ≥0.85 gate fails; no threshold was lowered:

- Stockade 14: 140/141 floor pixels overlap, but art contains 166 pixels. Near-threshold failure is primarily unmatched painted area (26 pixels), not a gross scale mismatch; Retail parity is still unproved.
- Barn 0: 16 overlapping pixels out of 17 floor / 18 art pixels. Tiny sampling footprint explains sensitivity, not an exemption granted by the user.
- Hive 1–5: 99.5–100% of floor samples lie in the art, but art exceeds the selected floor projection by 96, 111, 109, 85 and 103 pixels respectively. Floor-filter exclusions/painted margins are plausible explanations, not proven exemptions. Substantial-floor groups remain below threshold.
- Delve 0: unsplit indoor group, 432 floor pixels versus 23354 art pixels, IoU 0.0185. The earlier **different** parent-family-against-group-0-art experiment reaches only 0.7445; it cannot justify remapping this unsplit group or meet the gate. Delve 1 is exterior (MOGP `0x8` set, `0x2000` clear), so its score is reported but does not select an indoor map.

### Child-numbered rows: not settled

| Delve child | FDID | BLP size | Nonblack art samples | Own-floor/own-row IoU, unclipped |
|---|---:|---|---:|---:|
| 8 | 5623190 | 256×128 | 573 | 0.0268 |
| 10 | 5538173 | 256×64 | 531 | 0.2187 |
| 16 | 5635977 | 256×128 | 572 | 0.0713 |
| 17 | 5689798 | 256×128 | 572 | 0.0346 |

These rows contain actual nonempty painted art, not blank placeholder images. Their current-child extents fail the tested transform; the unclipped scores penalize floor outside their single-block canvases, unlike the earlier clipped common-fit figures. Nonempty art does **not** establish whether Retail uses it, regards it as an authoring remnant, or resolves it through another tile family. No inspected source establishes placeholder semantics or a Retail selection rule. GroupNum is not proven to be the current spatial-group index for modern split WMOs.

**Remaining gap:** authoritative Retail containing-group → WMOMinimapTexture tile-family mapping for unsplit indoor delve group 0 and child-numbered rows, plus the art/floor boundary responsible for sub-threshold hive groups. The split-parent portal rule is settled; extending it to minimap selection is not. Phase 2 remains blocked; no native run attempted (0/3), no screenshot fabricated.

## Evidence and reproduction

All CPU fit commands ran through the existing `scripts/agent/agent-run wmominimap2` wrapper. The follow-up command is `scripts/agent/agent-run wmominimap2 python3 data/diagnostics/wmominimap-2026-10-09/split-fit.py`; `split-fit.log` records EXIT 0 for 55 groups. `split-evidence-manifest.json` hashes its script/results/logs, fetched wiki/source scan and 37 family-rule PNGs. No code revision changed these numerical inputs. Research uses no external Python libraries and installs none. From this checkout:

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

Next research needs authoritative Retail evidence for mapping containing split/unsplit groups to the minimap tile family, especially delve group 0 and children with independent DB2 rows. Geometry should be checked after that selection is established; fitting the wrong group's art cannot establish texel math. Phase 2 indoor detection, world-position math, composition, zoom switching, TDD and native Stockade capture remain unimplemented/unproved. Native budget used: 0 of 3. The split-family follow-up above retains this stop decision after all 55 spatial groups, including previously omitted exterior delve group 1, were scored.

## Sources

- [Wowdev WMO revision 37115](https://wowdev.wiki/index.php?title=WMO&oldid=37115) — split topology and modern geometry fields; inspected successfully in the follow-up, no minimap selection contract.
- [Minimap contract](../../specs/minimap.md#indoor-wmo-map-unimplemented) — requirements and current blocker.
- [Reference catalog](../reference/open-source-wow-clients.md) — source selection.
- [wow.export minimap implementation](https://github.com/Kruithne/wow.export/blob/c2fd7bde36a712be78a5da896c995b84fbfa2545/src/js/wmo-minimap.js) — candidate transform and cropped-tile anchoring.
- [WebWowViewerCpp WMO header](https://github.com/Deamon87/WebWowViewerCpp/blob/1a8cccbeffc46231c6497e6b3f5bfbf3507d8071/wowViewerLib/src/engine/persistance/header/wmoFileHeader.h) and [traversal](https://github.com/Deamon87/WebWowViewerCpp/blob/1a8cccbeffc46231c6497e6b3f5bfbf3507d8071/wowViewerLib/src/engine/objects/wmo/wmoObject.cpp) — split fields and parent/child handling, not minimap selection.
- [WoWDBDefs schema](https://github.com/wowdev/WoWDBDefs/blob/master/definitions/WMOMinimapTexture.dbd) — schema only; no spatial equation.

## See Also

- [[minimap]] — existing outdoor/native implementation.
- [Local CASC extraction](../../casc-extraction.md) — archive provenance and extraction constraints.
