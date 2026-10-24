# Benilla / game-engine ADT Extension and LOD Fields

Field-level static inventory of E's root extensions, blend-mesh inputs and `_lod.adt` reader. Retention is not native rendering support. Standard MCNK/header/MCVT, texture layers, ordinary placements, MH2O/MCLQ, WDT and WDL belong to separate inventories; only the extension subchunks and LOD liquid/object records are expanded here.

Snapshot: **2026-10-24**. E baseline `354bdb7dbf177047ed46d63168acec531f5c4426`; B `8a6c74036cfe3d00275fd75ad1aeae1f4ea7d0df`. Roots and evidence vocabulary follow [[benilla-asset-field-inventory]] and pins match [[benilla-wmo-root-fields]]. Local text inspection only: no source execution/import, build, test, network, download, extraction or runtime observation. Revision labels are the requested comparison baselines, not a newly executed checkout/hash verification.

**Source keys:** A=`E/godot/core/src/asset/adt_format/adt.rs`; P=`adt/parsing.rs`; L=`adt/lod_chunks.rs` relative to A's directory; F=`E/godot/core/src/adt.rs`; N=`E/godot/rust/src/terrain/`; B0=`B/crates/benilla-adt/src/lib.rs`. Offsets below are payload-relative or record-relative, never file offsets. All multibyte values are little-endian. Vector/array rows explicitly enumerate component offsets; repeated homogeneous elements use a formula rather than implying a single scalar field.

## Complete source footprint and counting convention

The inventory covers **24 chunk families**: five root extensions, four MCNK extension/helper families (including the adjacent MCSE reader), and all fifteen tags explicitly recognized by L. It is not a count of all ADT fields or runtime capabilities. Counts below are serialized scalar slots per record, not Rust struct-member counts; variable arrays use their actual cardinality. Opaque bytes and words are included, not omitted from the denominator.

| Family | Full decoded footprint | Scalar slots per entry / payload | Source |
|---|---|---|---|
| MFBO | 36-byte prefix | 18 × i16 | P:305–333 |
| MBMH | 28-byte records | 7 × u32 | A:207–215; P:223–225 |
| MBBB | 28-byte records | u32 + 6 × f32 = 7 | A:219–223; P:227–229 |
| MBNV | 44-byte records | 8 × f32 + 12 × u8 = 20 | A:227–232; P:231–233 |
| MBMI | Entire even payload | N × u16 | P:235–250 |
| MCBB (not a tag named MBBBatch) | 20-byte records | 5 × u32 | A:197–203; P:190–207 |
| MCLV | 580-byte prefix | 145 × 4 bytes, including ignored byte 3 | P:102–126 |
| MCDD | 64-byte prefix | 64 × u8 | P:201–221 |
| MCSE | 28-byte records | u32 + 6 × f32 = 7 | A:189–193; P:171–188 |
| MVER | 4-byte prefix | 1 × u32 | P:335–337 |
| MLHD | 28-byte prefix | u32 + 6 × f32 = 7 | P:339–357 |
| MLVH | Entire multiple-of-4 payload | N × f32 | P:359–373 |
| MLLL | 20-byte records | f32 + 4 × u32 = 5 | P:375–401 |
| MLND | 20-byte records | 4 × u16 + 3 × u32 = 7 | P:400–428 |
| MLVI / MLSI | Each entire even payload | N × u16 each | P:430–449; A:566–567 |
| MLLD | Entire uninterpreted payload | N × u8, no field subdivision | A:569–571 |
| MLLN | 24-byte prefix | 6 × u32 | P:451–467 |
| MLLI | Entire even payload | N × u16 | P:430–449,491–504 |
| MLLV | 12-byte records | 3 × f32 | P:469–489 |
| MLDD / MLMD | Each 40-byte records | 2 × u32 + 7 × f32 + u32 = 10 each | P:506–543; A:572,574 |
| MLDX / MLMX | Each 28-byte records | 7 × f32 each | P:545–579; A:573,575 |

This accounts for every byte in each accepted fixed record. Prefix readers MFBO/MVER/MLHD/MLLN/MCLV/MCDD do not retain bytes after their documented prefix. MLLD is the explicit whole-payload opaque exception. No invented schema subdivides its bytes.

## Framing, collection and failure boundaries

A:319–363 `ChunkIter` reads tag `[u8;4] +0` and payload length `u32 +4`, then advances by `8 + length`. These two framing fields are consumed, not retained as result fields. Payload overrun produces an error; fewer than eight trailing bytes terminate iteration without preservation. Unknown complete chunks are skipped. This is not an unknown-chunk round-trip API.

P:765–797 root dispatch recognizes KNCM, O2HM, OBFM, HMBM, BBBM, VNBM and IMBM. The five extension tags map respectively to MFBO/MBMH/MBBB/MBNV/MBMI; MCNK and MH2O are scope handoffs. Root MVER/MHDR are not decoded by this collector. At least one MCNK is required. Repeated singleton extension tags replace the earlier borrowed payload; MCNK payloads accumulate.

P:270–303: no blend tag means `blend_mesh=None`. Any blend tag requires MBMH, MBNV and MBMI; MBBB is optional and becomes an empty bounds vector. Record arrays reject nonmultiples of their strides; MBMI rejects odd lengths. No cross-table reference/range/count validation is established by this assembly function. MFBO errors propagate. A:526–554 keeps MH2O parse errors separately as `water_error`, unlike fatal extension errors; MH2O fields remain outside this page.

P:695–738 routes MCLV via VLCM, MCBB via BBCM and MCDD via DDCM; repeated recognized subchunks replace the earlier decoded value. MCSE has a literal `b"MCSE"` match, **not** the expected reversed-looking ESCM: inventory records source spelling, not corrected format intent. Missing extension values become None, empty batches or empty emitters. Extension decoding does not establish that a flagged/expected payload exists beyond the checks shown.

## Native-consumer boundary used by every row

**R (root extension retained, native use unresolved):** A:526–554 assembles parsed root data; F:98–131 copies blend mesh and flight bounds into public `adt::Root`. Static lookup across `godot/rust/src` located no reads of `blend_mesh` or `flight_bounds` beyond empty fixture construction in N `streaming.rs:610–611`. This bounds the claim to inspected native Rust sources, not every tool/client or future revision.

**C (chunk extension retained, native use unresolved):** P:590–630 and F:98–122 copy `vertex_lighting`, `blend_batches`, `detail_doodad_disable` and `sound_emitters`. Native field-name lookup found no consumers of the first three in `godot/rust/src`. N `material.rs:217–245` builds ordinary chunk geometry and passes **vertex_colors**, not vertex_lighting, to `build_mesh`; N `mod.rs:137–200` constructs that mesh. SoundEmitter has parser/API retention here; a sound-system consumer is not established by this bounded terrain audit.

**D (LOD retained, native use unresolved):** A:557–580 assembles `ParsedLodData`; F:94–96 exposes `parse_lod`. Static lookup of `parse_lod`, `_lod.adt`, LOD result names and extension fields in native Rust terrain paths found no production LOD binding. N `horizon.rs:1–80` instead imports `parse_wdl` and loads WDL horizon tiles. WDL horizon rendering is not evidence that these ADT LOD records render. Native tree blend shapes and texture layer blend modes are different inputs, not blend-mesh consumers.

Each row names R/C/D instead of repeating the bounded lookup. None of these labels means original-client semantic authentication or runtime support.

## Root extension inputs

All values below are retained exactly as decoded unless explicitly transformed. Reads are `BinRead::read_le` for MBMH/MBBB/MBNV, direct LE scalar decoding for MBMI/MFBO. MBMH/MBBB/MBNV use P:252–268's stride-validated generic array reader.

| Serialized field | Offset / type / read | Retention and located boundary |
|---|---|---|
| MFBO upper samples | `+2*i`, `i=0..8`, i16 | First nine values → `FlightBounds.max_heights[i]`; R. |
| MFBO lower samples | `+18+2*i`, `i=0..8`, i16 | Next nine → `min_heights[i]`; R. Upper-first ordering is P's comment/implementation, not independent asset proof. |
| MBMH map_object_id | `+0`, u32 | `headers[].map_object_id`; R. No proven MODF association in native path. |
| MBMH texture_id | `+4`, u32 | `headers[].texture_id`; R. Do not infer FDID versus table-index semantics from this name. |
| MBMH unknown | `+8`, u32 | `headers[].unknown`; R; opaque, no semantics assigned. |
| MBMH index_count | `+12`, u32 | Same-named retained member; R. |
| MBMH vertex_count | `+16`, u32 | Same-named retained member; R. |
| MBMH index_start | `+20`, u32 | Same-named retained member; R. |
| MBMH vertex_start | `+24`, u32 | Same-named retained member; R. |
| MBBB map_object_id | `+0`, u32 | `bounds[].map_object_id`; R. |
| MBBB min | `+4,+8,+12`, [f32;3] | `bounds[].min`, no swizzle in reader; R. |
| MBBB max | `+16,+20,+24`, [f32;3] | `bounds[].max`, no swizzle in reader; R. |
| MBNV position | `+0,+4,+8`, [f32;3] | `vertices[].position`, raw coordinate order; R. |
| MBNV normal | `+12,+16,+20`, [f32;3] | `vertices[].normal`, no normalization; R. |
| MBNV uv | `+24,+28`, [f32;2] | `vertices[].uv`; R. |
| MBNV color[0] | `+32..35`, [u8;4] | `vertices[].color[0]`, raw four bytes; R. No channel semantics inferred. |
| MBNV color[1] | `+36..39`, [u8;4] | `vertices[].color[1]`, raw four bytes; R. |
| MBNV color[2] | `+40..43`, [u8;4] | `vertices[].color[2]`, raw four bytes; R. |
| MBMI index[i] | `+2*i`, u16 | `indices[i]`; R. Decoder does not validate triangle topology or referenced vertex range. |

## MCNK extension/helper inputs

These extension payloads are in scope without reopening the standard MCNK header/height/layer inventory. “MBBBatch” is not a declared type or recognized tag in A/P: the concrete type is `BlendBatch`, serialized through MCBB. MCDD's array is distinct from the MCNK header's eight-byte detail-exclusion map.

| Serialized field | Offset / type / read | Retention and located boundary |
|---|---|---|
| MCBB mbmh_index | `+0`, u32 / BinRead | `BlendBatch.mbmh_index`; C. No range validation against headers here. |
| MCBB index_count | `+4`, u32 / BinRead | Same-named member; C. |
| MCBB index_first | `+8`, u32 / BinRead | Same-named member; C. |
| MCBB vertex_count | `+12`, u32 / BinRead | Same-named member; C. |
| MCBB vertex_first | `+16`, u32 / BinRead | Same-named member; C. |
| MCLV blue[i] | `4*i+0`, u8 / indexing, i=0..144 | `vertex_lighting[i][2] = byte/128.0`; C. |
| MCLV green[i] | `4*i+1`, u8 / indexing | `vertex_lighting[i][1] = byte/128.0`; C. |
| MCLV red[i] | `4*i+2`, u8 / indexing | `vertex_lighting[i][0] = byte/128.0`; C. |
| MCLV fourth byte[i] | `4*i+3`, u8 span | Required by 580-byte size check but **not read into output**; alpha is manufactured as 1.0. No opaque-byte retention; C. |
| MCDD disable[i] | `+i`, u8, i=0..63 / copy | Full prefix copied into `detail_doodad_disable[i]`; C. Bit/cell interpretation is not established by this copy. |
| MCSE sound_entry_id | `+0`, u32 / BinRead | `SoundEmitter.sound_entry_id`; C, sound binding unresolved here. |
| MCSE position | `+4,+8,+12`, [f32;3] / BinRead | Same-named member, no coordinate projection here; C. |
| MCSE size_min | `+16,+20,+24`, [f32;3] / BinRead | Same-named member; C. No radius/box semantics added. |

MCBB/MCSE require exact stride multiples. MCLV/MCDD require at least 580/64 bytes and ignore extra bytes. Retained MCLV RGB can exceed 1.0 (255/128); do not describe it as ordinary byte/255 normalization.

## LOD recognition and grouping

L:48–80 recognizes **all** of: MVER/REVM, MLHD/DHLM, MLVH/HVLM, MLLL/LLLM, MLND/DNLM, MLVI/IVLM, MLSI/ISLM, MLLD/DLLM, MLDD/DDLM, MLDX/XDLM, MLMD/DMLM, MLMX/XMLM, MLLN/NLLM, MLLI/ILLM and MLLV/VLLM. The first seven are required by L:113–145. Required means present, not nonempty; MVER is read but not compared to a supported-version constant here.

Singleton tags replace earlier payloads. MLLN starts/replaces a pending header and clears pending indices; MLLI is accepted only with a pending header, otherwise ignored by the default branch. Repeated MLLI replaces pending indices. MLLV requires both pending header and indices, consumes them and appends one ordered liquid group. End-of-file pending state errors. A replacement MLLN can discard an unfinished earlier group; this collector is not a strict duplicate/orphan detector for every case. Unrelated chunks may intervene. No count correspondence among header words, nodes, levels, indices, visibility and placement arrays is established by these functions.

### LOD terrain records

| Serialized field | Offset / type / read | Retention and located boundary |
|---|---|---|
| MVER version | `+0`, u32 / read_u32 | `ParsedLodData.version`; D. Minimum four bytes; extra bytes ignored. |
| MLHD flags | `+0`, u32 / read_u32 | `header.flags`; D. No bit meanings established. |
| MLHD bounds_min[0] | `+4`, f32 / read_f32 | `header.bounds_min[0]`; D. |
| MLHD bounds_max[0] | `+8`, f32 / read_f32 | `header.bounds_max[0]`; D. |
| MLHD bounds_min[1] | `+12`, f32 / read_f32 | `header.bounds_min[1]`; D. |
| MLHD bounds_max[1] | `+16`, f32 / read_f32 | `header.bounds_max[1]`; D. |
| MLHD bounds_min[2] | `+20`, f32 / read_f32 | `header.bounds_min[2]`; D. |
| MLHD bounds_max[2] | `+24`, f32 / read_f32 | `header.bounds_max[2]`; D. Interleaved source is deinterleaved, not coordinate-swizzled. |
| MLVH height[i] | `+4*i`, f32 / read_f32 | `heights[i]`; D. Entire multiple-of-four payload. |
| MLLL vertex_step | record `+0`, f32 / read_f32 | `levels[].vertex_step`; D. Name alone does not prove mesh use. |
| MLLL payload[0] | `+4`, u32 / read_u32 | `levels[].payload[0]`; D; opaque. |
| MLLL payload[1] | `+8`, u32 / read_u32 | `levels[].payload[1]`; D; opaque. |
| MLLL payload[2] | `+12`, u32 / read_u32 | `levels[].payload[2]`; D; opaque. |
| MLLL payload[3] | `+16`, u32 / read_u32 | `levels[].payload[3]`; D; opaque. |
| MLND words16[0] | `+0`, u16 / from_le_bytes | `nodes[].words16[0]`; D; opaque. |
| MLND words16[1] | `+2`, u16 / from_le_bytes | `nodes[].words16[1]`; D; opaque. |
| MLND words16[2] | `+4`, u16 / from_le_bytes | `nodes[].words16[2]`; D; opaque. |
| MLND words16[3] | `+6`, u16 / from_le_bytes | `nodes[].words16[3]`; D; opaque. |
| MLND words32[0] | `+8`, u32 / read_u32 | `nodes[].words32[0]`; D; opaque. |
| MLND words32[1] | `+12`, u32 / read_u32 | `nodes[].words32[1]`; D; opaque. |
| MLND words32[2] | `+16`, u32 / read_u32 | `nodes[].words32[2]`; D; opaque. |
| MLVI index[i] | `+2*i`, u16 / from_le_bytes | `indices[i]`; D; no vertex-range validation here. |
| MLSI index[i] | `+2*i`, u16 / from_le_bytes | `skirt_indices[i]`; D; name is parser storage, not proof of rendered skirts. |

MLHD accepts a 28-byte prefix. MLLL/MLND reject partial 20-byte records. Scalar arrays reject nonmultiples of their element width. No extra opaque tail survives these record-array readers because malformed tails fail rather than being copied.

### LOD liquid records (not MH2O/MCLQ)

| Serialized field | Offset / type / read | Retention and located boundary |
|---|---|---|
| MLLD opaque directory | `+0..len`, bytes / to_vec | `liquid_directory=Some(LodLiquidDirectory { raw })`; D. Arbitrary length including zero; no decoded count/offset contract. Missing chunk → None. |
| MLLN words[0] | `+0`, u32 / read_u32 | `liquids[].header.words[0]`; D; opaque. |
| MLLN words[1] | `+4`, u32 / read_u32 | `liquids[].header.words[1]`; D; opaque. |
| MLLN words[2] | `+8`, u32 / read_u32 | `liquids[].header.words[2]`; D; opaque. |
| MLLN words[3] | `+12`, u32 / read_u32 | `liquids[].header.words[3]`; D; opaque. |
| MLLN words[4] | `+16`, u32 / read_u32 | `liquids[].header.words[4]`; D; opaque. |
| MLLN words[5] | `+20`, u32 / read_u32 | `liquids[].header.words[5]`; D; opaque. |
| MLLI index[i] | `+2*i`, u16 / from_le_bytes | `liquids[].indices[i]`; D. No patch-header count validation here. |
| MLLV vertex[0] | record `+0`, f32 / read_f32 | `liquids[].vertices[][0]`; D, no axis interpretation added. |
| MLLV vertex[1] | `+4`, f32 / read_f32 | `liquids[].vertices[][1]`; D. |
| MLLV vertex[2] | `+8`, f32 / read_f32 | `liquids[].vertices[][2]`; D. |

MLLN requires 24 bytes and discards later bytes; MLLI requires even length; MLLV requires a multiple of 12. Empty group vector is valid. All six header words remain opaque: no liquid type, count, offset or bounds semantics are inferred from numerical values or array position.

### LOD object records (not ordinary MDDF/MODF)

Each paired table below applies independently to **both** named chunks: there are two separate serialized populations and result vectors, not one shared record instance. MLDD → `m2_placements`; MLMD → `wmo_placements`. Each is optional, missing → empty; stride is 40, no trailing bytes accepted. Reads are P:506–543 `read_u32`/`read_f32` with no conversion.

| Serialized field in each MLDD and MLMD record | Offset / type | Retention and located boundary |
|---|---|---|
| id | `+0`, u32 | `LodObjectPlacement.id`; D. |
| asset_id | `+4`, u32 | `asset_id`; D. No validated FDID/name-index interpretation here. |
| position[0] | `+8`, f32 | `position[0]`; D. |
| position[1] | `+12`, f32 | `position[1]`; D. |
| position[2] | `+16`, f32 | `position[2]`; D. |
| rotation[0] | `+20`, f32 | `rotation[0]`; D. No angle unit/order inferred. |
| rotation[1] | `+24`, f32 | `rotation[1]`; D. |
| rotation[2] | `+28`, f32 | `rotation[2]`; D. |
| scale | `+32`, f32 | `scale`; D. Not MDDF's u16/1024 decoder. |
| flags | `+36`, u32 | `flags`; D. Opaque bit semantics. |

MLDX → `m2_visibility`; MLMX → `wmo_visibility`. Each optional 28-byte-record array is parsed independently by P:545–579; missing → empty. No matching-length, ID join, radius validity or min/max ordering check is established here.

| Serialized field in each MLDX and MLMX record | Offset / type / read | Retention and located boundary |
|---|---|---|
| bounds_min[0] | `+0`, f32 / read_f32 | `LodObjectVisibility.bounds_min[0]`; D. |
| bounds_min[1] | `+4`, f32 / read_f32 | `bounds_min[1]`; D. |
| bounds_min[2] | `+8`, f32 / read_f32 | `bounds_min[2]`; D. |
| bounds_max[0] | `+12`, f32 / read_f32 | `bounds_max[0]`; D. |
| bounds_max[1] | `+16`, f32 / read_f32 | `bounds_max[1]`; D. |
| bounds_max[2] | `+20`, f32 / read_f32 | `bounds_max[2]`; D. |
| radius | `+24`, f32 / read_f32 | `radius`; D. No native culling binding established. |

## Derived and helper representations — not additional disk fields

| Representation | Proven construction / boundary |
|---|---|
| `BlendMeshData` | Aggregates four decoded vectors `headers/bounds/vertices/indices`; P:270–303. No extra serialized field, and optional MBBB becomes empty. |
| `FlightBounds.min_heights/max_heights` | Named copies of the two nine-i16 planes; no interpolation grid coordinates or flight enforcement are produced by P:305–333. |
| MCLV `[f32;4]` | Reordered/scaled RGB plus constant alpha; not preservation of four source bytes. |
| `ParsedAdtData`, public `Root` | `blend_mesh` and `flight_bounds` are optional retained aggregates; water/water_error are separate parse outcomes. F copies, not native binding. |
| `McnkSubchunkAccum`, result tuple, `McnkData`, public `Chunk` | Intermediate options/defaults and member copies are successive representations of the same extension inputs, not duplicate serialized fields. |
| `AdtRootChunks`, `LodRootChunks` | Borrowed chunk-payload routing slots; no disk offset/count structure is implied by a Rust Option. |
| `PendingLodLiquidGroup`, `LodLiquidChunkGroup` | Parser order state and grouped borrowed slices; not serialized headers. |
| `ParsedLodData` | All thirteen declared fields: version, header, heights, levels, nodes, indices, skirt_indices, liquid_directory, liquids, m2_placements, m2_visibility, wmo_placements, wmo_visibility. Fifteen is the recognized-tag count, not the member count. |
| `LodHeader`, `LodLevel`, `LodQuadTreeNode` | Retained record wrappers above. Their names do not supply meanings for opaque payload/word arrays. |
| `LodLiquidDirectory`, `LodLiquidPatchHeader`, `LodLiquidPatch` | Opaque raw directory, six-word header, and header/indices/vertices aggregate; no directory-driven grouping interpretation. |
| `LodObjectPlacement`, `LodObjectVisibility` | Shared Rust record shapes applied to separate M2/WMO populations; no new fields beyond the tables. |
| `ChunkHeightGrid` | A:51–59,449–464 (origin derivation A:421–447): index_x/index_y copied; origin_x/origin_z derived via chunk/tile origin; base_y from pos[2]; heights copied. Standard MCNK/MCVT inputs owned elsewhere, not extension bytes. |
| `center_surface` | A:507–523 selects chunk (8,8), otherwise middle chunk, then computes vertex row9/col4 position. Derived helper, not a root serialized member. |
| `chunk_positions` | A:536 copies parsed MCNK positions into a vector; no extra source fields. |

## Benilla comparison and unresolved semantics

B0:195–210 models a Vanilla monolithic `RootAdt` and only a Root variant of `ParsedAdt`. B0:235–289 dispatches MTEX, MMDX/MMID, MWMO/MWID, MDDF/MODF and MCNK, skipping other top-level tags. Thus **none of the five E root extension families or fifteen LOD tags has a matched result in this inspected B root parser**. This is a bounded parser nonmatch, not a global claim that all Benilla crates lack analogous functionality.

B0:352–505 `read_mcnk` uses header-offset lookup for MCVT, MCNR, MCLY, MCAL, MCSH and MCLQ; its `McnkChunk` at B0:157–170 has no corresponding MCLV/MCBB/MCDD/MCSE fields. Its older monolithic offset walk must not be equated with E's sequential modern extension dispatch or transplanted as the layout for modern split files. B's ordinary placements and MCLQ are separate inventories, not substitutes for MLDD/MLMD or MLLN groups.

Unresolved: MBMH unknown/texture identifier semantics; MBNV three color-vector roles; MLLL four opaque words; MLND seven opaque words; MLLN six opaque words; MLLD internal layout; LOD flags/coordinate conventions and placement/visibility associations. Static native lookup establishes retention boundaries, not asset validity, original-client behavior, render/collision/flight support, or a proof that no consumer exists outside the inspected paths. No runtime parity claim is made.

## Sources

- E `godot/core/src/asset/adt_format/adt.rs` — complete extension/LOD declarations, constants, framing, helper projections and parsed assembly.
- E `godot/core/src/asset/adt_format/adt/parsing.rs` — extension readers, exact offsets/strides, normalization, defaults and root/subchunk dispatch.
- E `godot/core/src/asset/adt_format/adt/lod_chunks.rs` — full fifteen-tag LOD recognition, required chunks and order-dependent liquid grouping.
- E `godot/core/src/adt.rs` — public retained Root/Chunk projections and `parse_lod` entry point.
- E `godot/rust/src/terrain/{assets,material,mod,streaming,horizon}.rs` — root loading, actual ordinary terrain mesh/color inputs, empty fixture extension fields and separate WDL horizon path; bounded native field-name lookup across `godot/rust/src`.
- B `crates/benilla-adt/src/lib.rs` — inspected Vanilla root model, bounded top-level dispatch and MCNK offset reader; no whole-repository absence inference.
- [Wiki schema](../SCHEMA.md) and [index](../index.md) — page conventions and existing comparison context. The integrated inventory commit also updates parent/index/log navigation; those links are documentation wiring, not additional source-field evidence.

## See Also

- [[benilla-asset-field-inventory]] — comparison pins, evidence vocabulary and broader asset audit.
- [[benilla-wmo-root-fields]] — matching snapshot/pin conventions and separate WMO root inputs.
- [[terrain]] — native terrain system context; not a substitute for the field-level boundaries above.
- [[open-source-wow-clients]] — version-specific reference limits.
