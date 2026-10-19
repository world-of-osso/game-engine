# Shared WoW Format Readers

Verified: 2026-10-18. Client M2/skin, WMO, ADT/WDT binary readers moved from `godot/core/src/asset/` into the public `world-of-osso/wow-formats` repository. `godot/core/Cargo.toml` pins all four crates to the same git revision; both workspace lockfiles record that revision.

## Boundaries

| Crate | Moved code | Optional rendering data/helpers |
| --- | --- | --- |
| `wow-chunk` | `read_bytes.rs` and the original ADT `ChunkIter` | None |
| `wow-m2` | `m2_format/`, particle defaults, batch/texture/composite code and `rgba_blit` | `animation`, `attachments`, `materials`, `particles`, `ribbons`, `lights`, `cameras`, `events`, `textures`; default `render` enables all |
| `wow-wmo` | `wmo_format/` | Default `render`, `fog`, `portals` |
| `wow-adt` | `adt_format/` and `asset/wdt.rs` | Default `geometry` mesh helpers; raw chunk metadata is always available |

No cross-format dependencies: model references remain FDIDs. External dependencies are `binrw` and `glam` 0.32. Geometry consumers disable default features. M2 track-dependent features enable `animation`; geometry-only M2 does not compile texture/material readers or animation, particle, ribbon, light, camera and event modules.

The engine's `asset::{m2_format,wmo_format,adt_format,read_bytes,wdt,m2_batch_data,m2_texture}` and `m2_texture_composite_data` names are direct `pub use` re-exports, not duplicate implementations. `m2.rs`, `adt.rs`, `wmo.rs`, runtime material/animation/simulation policy and engine-axis adapters remain in core. The pure global-sequence timeline helper lives with M2 tracks and is re-exported by `core::m2`. Equipment no longer includes a parser source file by relative path; it imports the same shared bone-name module.

## Provenance and proof scope

Clean-room extraction from engine revision `55e1f8687`: no decompiled or leaked client source was opened, searched or reconstructed. Existing public-documentation/open-source provenance comments remain with the parsers. Parser bodies and error policies are preserved; changes are module paths, crate visibility and feature boundaries.

Original format modules contained **259 tests**: **225 synthetic parser tests** move to the new repository; **34 authored-asset fixtures** remain in engine integration tests. Other existing core integration fixtures stay in place. A before/after name multiset tracks conservation independently of test-module paths. No assets are copied or published.

Baseline helper `--test -p game-engine-core --no-fail-fast` at `55e1f8687`: **1063 passed, 1 failed, 4 ignored**. The existing BLP palette fixture cannot find `data/textures/180071.blp`; extraction does not alter that unrelated failure. Standalone full-feature tests pass225 and geometry-only tests pass149; every individual M2 optional feature compiles. After extraction, core passes838 with the same1 failure/4 ignored; adding the225 moved tests exactly reproduces baseline1063/1/4, including an identical1068-name multiset. Godot consumers/adapters pass242 tests in two disjoint filter sets (one additional pre-existing ignored test). See the [proof ledger and exact gate filters](../../handoffs/wow-formats-extraction.md).

## Geometry-consumer API split

The separately approved server migration uses `materials` for raw skin batches/materials and `attachments` for static seat lookup; neither requires animation. The full default `render` feature still includes all original client parsing modules. Eye/head posing can enable `animation` independently of emitters, lights, cameras, events and texture composition.

Raw geometry APIs preserve consumer policies: skin sections expose authored high-bit `raw_triangle_start` alongside the unchanged cumulative `triangle_start`; first-bone-key reading keeps external/global sources distinct; static attachment lookup ignores activation tracks; M2 normals retain authored face signs; WDT exposes all MAID slots separately from MAIN-active tiles; ADT's height-only view retains file-axis origins/holes without requiring normals or companion shadows. WMO exposes its existing raw vector/index array readers for server selection without enabling presentation helpers.

Server phase 2 landed with identical Stockade LOS/nav binaries. Runtime ground's subsequent metadata-only consolidation uses projected MPHD/MAID/MODF/MOHD/GFID views, preserving first-match/mandatory/FDID-only policy and not validating unrelated renderer fields. Its server ledger compares complete ordered metadata bytes plus fresh Stockade bakes. Client pin changes retain the same default features and require the engine consumer gate. The runtime-heights revision adds separate raw MCNK header (origin/area/hole masks) and MCVT projections; the client's existing whole-tile parser uses the same header projection without changing its chunk-selection/error semantics. Server query/interpolation, MH2O and hole-widening policy remain outside the shared readers.

## Runtime-height pin gate

Library `ed180cecd7228471944dcdf1d427468e5cc6e7b5` adds four header/MCVT projection tests; default246 and no-default166 pass, check/fmt pass. Engine source `95a7f014ee888b688c5af33e8ca4841edbfa874e` (includes current master), protocol `f9a91753bbbc15ad7ff7392955b52c4da989d22e`: helper core `adt`9/0 and native Godot `terrain::`45/0, helper check and workspace fmt0. The terrain filter covers authored tile parsing and streamed area/surface queries. An initial `wmo::collision_tests` probe selected zero tests and is not evidence. No engine runtime behavior or features changed. Independent lead integration remains pending.

Server before/after raw height/query/negative receipts and fresh Stockade guards are recorded in its [height ledger](../../../../game-server/docs/handoffs/wow-formats-runtime-heights.md); interpolation, hole widening and MH2O are not moved.

## MH2O raw-record projection

Pinned revision2a91b224 exposes the existing12B directory,24B instance and16B attribute readers without changing their bodies or client high-level parsing. The server consumes these projections while retaining its different bitmap/vertex interpretation and validation/error order; neither parser switches modes or attempts reconciliation. Client UV/depth/raw-object arrays and material-deferred LiquidObject decoding remain unchanged. Independent per-consumer before/after receipts are described in the server [MH2O ledger](../../../../game-server/docs/handoffs/wow-formats-mh2o-records.md):180 files28,034,481B match exactly and116 source hashes remain unchanged. On engine sourcef141dc100/protocolf9a91753, helper water integrations20/0 (liquid_water13,terrain_water_height4,adt_water_geometry3), native terrain45/0 and ground18/0, helper check/fmt0. Subsequent changes are docs-only; independent lead integration remains pending.

## Sources

- [Core dependencies](../../../godot/core/Cargo.toml) — authoritative pinned revisions
- [Core format exports](../../../godot/core/src/asset.rs) — original namespace preservation
- [Retained parser fixtures](../../../godot/core/tests/format_fixtures.rs) — client data boundary
- `world-of-osso/wow-formats/README.md`, `extraction-inventory.json` — crate features and original file inventory

## See Also

- [M2 format](../formats/m2-format.md)
- [WMO format](../formats/wmo-format.md)
- [ADT format](../formats/adt-format.md)
