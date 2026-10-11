# Diablo IV Appearance geometry

The [offline triangle counter](../../specs/d4-triangle-count.md) reads paired extracted `.app` metadata and payload files. CASC is their archive container, not the mesh format. This diagnostic does not load Diablo assets into the game engine.

## Layout evidence

Field offsets and record sizes come from MIT-licensed `DiabloTools/d4data` revision `33e0bfbb5f717d3e560d14a7fb27d5a7f17fd2c0`, schema build `3.2.1.73552`. The extracted `druM_base00.app` pair came from the Windows install reporting `3.2.2.73764`. The local installation's metadata pointer tables were verified to match the schema; independent range/index traversal supports this fixture's layout compatibility, not universal support for either build. The locally saved reference is `data/diablo4-analysis/reference/d4data/`; its `buildVersion.txt` reports `3.2.1.73552`.

Metadata begins with `0xDEADBEEF`; its object starts at byte 16. Variable-array descriptors occupy 16 bytes: eight zero padding bytes, then a 32-bit offset and byte length. Metadata offsets are relative to byte 16; geometry payload offsets are absolute in the paired payload file. The counter rejects nonzero array padding. The metadata SNO ID at byte 16 must match the payload ID at byte 0. This checks the declared asset ID, not content authenticity or a build/version identifier.

| Record | Size | Fields consumed, offsets relative to record |
|---|---:|---|
| AppearanceDefinition | 312 | Structure at 8; its chunk array at 32 |
| GeoChunk | 88 | LOD array 24; vertex-buffer array 40; index-buffer array 56 |
| GeoSet | 40 | Subobject array 0 |
| GeoChunkVertexBuffer | 80 | Format 0; stride 4; external IDs 40/64/68; vertex byte-array 48 |
| GeoChunkIndexBuffer | 24 | 16-bit index array 0; external buffer ID 16 |
| SubObject | 240 | Flags 0; hash 100; buffer indices 104/108; segment array 200 |
| SubObjectSegment | 32 | Vertex count 16; vertex byte-offset 20; index count 24; index element-offset 28 |

Only vertex format 6 with stride 44 and inline buffers is enabled. This is the observed Druid layout, not a generalized vertex decoder. Indices are little-endian unsigned 16-bit values, as specified by the schema's `DT_WORD` element type. Each segment is interpreted as a triangle list, checked for whole triplets and indices below its own vertex count. Counts include stored triplets; geometric degeneracy is not evaluated. The parser does not detect arbitrary topology: whole triplets and segment-local bounds validate the supported triangle-list interpretation, not unknown layouts.

Negative signed segment counts/offsets and buffer indices fail explicitly. Nonzero serialized-array padding, misaligned vertex byte offsets, invalid consumed array sizes/ranges and empty geometry results also fail. Unconsumed Appearance fields are not validated.

## Druid investigation

Local files: metadata 78,728 bytes, payload 10,177,648 bytes. The explicit chunk table yields one chunk, five LODs, two vertex buffers and two index buffers. LOD subobject counts are 32, 31, 31, 31 and 29, with one segment per subobject: 154 segments. Independent Python traversal of the schema-defined records found no invalid vertex ranges, index ranges or segment-local indices.

The old AppToOBJ algorithm scanned for sentinel patterns and guessed the index table from nearby bytes. Its inferred table began too early and produced out-of-file offsets. The schema instead resolves the actual index descriptor table at metadata byte 59,944. No converter code was copied.

The appearance contains multiple subobjects and potential customization/visibility alternatives. Summing all stored geometry is not a defensible polygon count for one rendered Druid. The parser returns separate records identified by chunk, LOD, subobject, segment, raw flags and hash. The CLI contract requires these rows without an aggregate character total; its final proof remains pending. Flags and hashes are retained raw, not interpreted as active-look selectors.

## Proof scope

- Parser `2eb8a9261` contains 13 synthetic paired-file tests, reported GREEN before commit: LOD/segment separation, offset units, negative segment ranges, missing buffer references, bad magic, out-of-range indices, asset mismatch, incomplete triangles, truncated files, external vertex buffers, unknown vertex format and malformed record-array size. This audit ran no tests.
- External index IDs, stride mismatch, array padding, vertex alignment, negative buffer indices and empty geometry are explicit rejection paths in code, not separately asserted by those 13 tests.
- Real-data compatibility currently has independent schema traversal evidence; CLI integration verification is tracked in the [spec](../../specs/d4-triangle-count.md).
- Other models/builds/layouts and rendered active-look selection are unverified.

## Sources

- [d4data definitions](https://github.com/DiabloTools/d4data/blob/33e0bfbb5f717d3e560d14a7fb27d5a7f17fd2c0/definitions.json) — named records, offsets, sizes and index element type.
- [d4data parser](https://github.com/DiabloTools/d4data/blob/33e0bfbb5f717d3e560d14a7fb27d5a7f17fd2c0/parse.js) — SNO array serialization.
- [License attribution](../../../tools/licenses/d4data.txt) — adapted field offsets/record sizes; includes the complete MIT notice, copyright © 2023 blizzhackers, matching the locally saved `LICENSE`. The pinned revision is recorded provenance, not independently re-fetched by this audit.
- [Counter implementation](../../../tools/src/bin/d4_triangle_count/appearance.rs).

## See Also

- [Counter contract](../../specs/d4-triangle-count.md).
- [CASC architecture](../../casc-architecture.md) — separate archive/extraction concerns.
- [[casc-format]] — archive format overview.
