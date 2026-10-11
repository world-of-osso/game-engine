# Diablo IV triangle counter

Offline diagnostic in `tools/src/bin/d4_triangle_count.rs`, accepting extracted Diablo IV Appearance metadata and payload files. [Format evidence and limitations](../wiki/formats/d4-appearance.md) describe the supported layout.

## What it must do

- [x] Decode explicit chunk/LOD/subobject/segment records; return each segment's declared vertices and validated triangle-list count separately, without adding alternative LODs or appearance variants together.
- [x] Validate metadata magic and matching metadata/payload SNO IDs, consumed record-array sizes, buffer references, byte ranges, segment-local index bounds, and complete triangles; reject malformed consumed data explicitly.
- [x] Interpret segment vertex offsets as bytes and index offsets as 16-bit elements.
- [x] Support only vertex format 6, stride 44, inline little-endian u16 triangle lists; reject other format/stride values and external buffer IDs rather than scanning for plausible byte patterns.
- [ ] Accept two input paths and emit TSV columns `chunk`, `lod`, `subobject`, `segment`, `flags`, `hash`, `vertices`, `triangles`; errors use stderr and a failing exit status.
- [ ] Verify current installed-build Druid data through the command-line binary against an independent schema-based count.

## How it works

- [Appearance layout, provenance and proof scope](../wiki/formats/d4-appearance.md).
- [Adapted schema license](../../tools/licenses/d4data.txt).

## Implementation inventory

- `tools/src/bin/d4_triangle_count.rs` — input/output CLI boundary.
- `tools/src/bin/d4_triangle_count/appearance.rs` — bounds-checked offline decoding and behavioral fixtures.
- `tools/tests/d4_triangle_count_cli.rs` — process-level CLI tests.

## Tests asserting this spec

- Unit tests in `tools/src/bin/d4_triangle_count/appearance.rs`: concrete paired-file fixtures; separate LODs/segments, nonzero offsets, invalid indices/ranges/arrays, asset mismatch and unsupported buffers/formats.
- `tools/tests/d4_triangle_count_cli.rs`: five process-level tests cover help, argument validation and unreadable metadata/payload paths; RED then GREEN at `aa548e038`.
- Parser `2eb8a9261`: 13 synthetic tests reported GREEN before commit; this documentation audit does not rerun them or certify CLI/real-file integration.

## Known gaps (current cycle)

- [ ] Independent integration verification of current Druid files and CLI output.

## Out of scope

- CASC reading/extraction, rendering, textures, animation and game deployment.
- Selecting a rendered character's active appearance/look, customization, equipment or visibility. The output inventories stored segments, not a single rendered character's draw list.
- Unobserved vertex layouts, external buffers and other index topologies; unsupported cases must fail explicitly.
