# Shared WoW Format Readers

Verified: 2026-10-18. Client M2/skin, WMO, ADT/WDT binary readers moved from `godot/core/src/asset/` into the public `world-of-osso/wow-formats` repository. `godot/core/Cargo.toml` pins all four crates to the same git revision; both workspace lockfiles record that revision.

## Boundaries

| Crate | Moved code | Optional rendering data/helpers |
| --- | --- | --- |
| `wow-chunk` | `read_bytes.rs` and the original ADT `ChunkIter` | None |
| `wow-m2` | `m2_format/`, particle defaults, batch/texture/composite code and `rgba_blit` | `animation`, `particles`, `ribbons`, `lights`, `cameras`, `events`, `textures`; default `render` enables all |
| `wow-wmo` | `wmo_format/` | Default `render`, `fog`, `portals` |
| `wow-adt` | `adt_format/` and `asset/wdt.rs` | Default `geometry` mesh helpers; raw chunk metadata is always available |

No cross-format dependencies: model references remain FDIDs. External dependencies are `binrw` and `glam` 0.32. Geometry consumers disable default features. M2 track-dependent features enable `animation`; geometry-only M2 does not compile texture/material readers or animation, particle, ribbon, light, camera and event modules.

The engine's `asset::{m2_format,wmo_format,adt_format,read_bytes,wdt,m2_batch_data,m2_texture}` and `m2_texture_composite_data` names are direct `pub use` re-exports, not duplicate implementations. `m2.rs`, `adt.rs`, `wmo.rs`, runtime material/animation/simulation policy and engine-axis adapters remain in core. The pure global-sequence timeline helper lives with M2 tracks and is re-exported by `core::m2`. Equipment no longer includes a parser source file by relative path; it imports the same shared bone-name module.

## Provenance and proof scope

Clean-room extraction from engine revision `55e1f8687`: no decompiled or leaked client source was opened, searched or reconstructed. Existing public-documentation/open-source provenance comments remain with the parsers. Parser bodies and error policies are preserved; changes are module paths, crate visibility and feature boundaries.

Original format modules contained **259 tests**: **225 synthetic parser tests** move to the new repository; **34 authored-asset fixtures** remain in engine integration tests. Other existing core integration fixtures stay in place. A before/after name multiset tracks conservation independently of test-module paths. No assets are copied or published.

Baseline helper `--test -p game-engine-core --no-fail-fast` at `55e1f8687`: **1063 passed, 1 failed, 4 ignored**. The existing BLP palette fixture cannot find `data/textures/180071.blp`; extraction does not alter that unrelated failure. Standalone full-feature tests pass225 and geometry-only tests pass149; every individual M2 optional feature compiles. After extraction, core passes838 with the same1 failure/4 ignored; adding the225 moved tests exactly reproduces baseline1063/1/4, including an identical1068-name multiset. Godot consumers/adapters pass242 tests in two disjoint filter sets (one additional pre-existing ignored test). See the [proof ledger and exact gate filters](../../handoffs/wow-formats-extraction.md).

Phase 2 (game-server reader replacement and byte-identical LOS/nav bake) requires separate lead approval and has not started.

## Sources

- [Core dependencies](../../../godot/core/Cargo.toml) — authoritative pinned revisions
- [Core format exports](../../../godot/core/src/asset.rs) — original namespace preservation
- [Retained parser fixtures](../../../godot/core/tests/format_fixtures.rs) — client data boundary
- `world-of-osso/wow-formats/README.md`, `extraction-inventory.json` — crate features and original file inventory

## See Also

- [M2 format](../formats/m2-format.md)
- [WMO format](../formats/wmo-format.md)
- [ADT format](../formats/adt-format.md)
