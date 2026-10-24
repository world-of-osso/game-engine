# ADT MCNK Header Field Handling

Individual serialized header inputs and their projections in game-engine (E) versus Benilla (B). Static source inspection only: no source execution, imports, builds, tests or asset/corpus validation. E baseline `354bdb7dbf177047ed46d63168acec531f5c4426`; B `8a6c74036cfe3d00275fd75ad1aeae1f4ea7d0df`, Vanilla1.12.1/build5875 monolithic ADT. E uses modern root/texture/object companion paths. Equal offsets do not establish equal version semantics.

## Source keys and framing

- **D**: `godot/core/src/asset/adt_format/adt.rs`; **P**: sibling `adt/parsing.rs`; **A**: `godot/core/src/adt.rs`.
- **B**: `/home/osso/Repos/benilla/crates/benilla-adt/src/lib.rs`.
- E declaration D237–269 consumes **116 serialized bytes** via BinRead; P581–591 starts subchunks at payload byte128. This is not a claim that Rust `size_of::<McnkHeader>()` equals serialized size: the preceding size check uses the Rust type. Bytes `0x74..0x7f` have no declared field and are skipped by the fixed subchunk boundary.
- B353–412 requires128 bytes, reads named header projections and locates subchunks through selected header offsets; its checked magic lookup accepts an offset to the chunk magic or payload-relative adjustment. E instead walks its root MCNK subchunk stream. Unrecognized input bytes stay unassigned.

## E fields and B offset-qualified contrast

Each E row is a distinct declared field. Offsets are MCNK-payload-relative, integers/floats little-endian. E retention is P594–631; B named projections are B358–384, temporary offset reads B414–493.

| Offset | E field/type; declaration | E retained/derived/dropped | B read at same bytes, not an asserted modern equivalent |
|---|---|---|---|
| `0x00` | `flags: u32`, D238 | Five boolean predicates below; other bits lost from this projection | Raw `flags` retained, B359; impassable and liquid-mask predicates below |
| `0x04` | `index_x: u32`, D239 | `McnkData.index_x` | Same named member retained, B360 |
| `0x08` | `index_y: u32`, D240 | `McnkData.index_y` | Same named member retained, B361 |
| `0x0c` | `_n_layers: u32`, D241 | Read but not copied into cooked chunk | No read in B header/offset reader |
| `0x10` | `_n_doodad_refs: u32`, D242 | Read then dropped | No read in that B reader |
| `0x14` | `_holes_high_res: u64`, D244 | `holes_high_res=Some(value)` only with high-res-hole flag; otherwise dropped | Two separate u32 offset reads at `0x14`/`0x18` locate MCVT/MCNR, B414–448; no high-res-hole member |
| `0x1c` | `_ofs_mcly: u32`, D245 | Dropped; streamed subchunk parsing | Temporary MCLY locator, B446–463; not a retained header member |
| `0x20` | `_ofs_mcrf: u32`, D246 | Dropped | No MCRF read in this B path |
| `0x24` | `_ofs_mcal: u32`, D247 | Dropped | Temporary MCAL locator, B466–474 |
| `0x28` | `_size_mcal: u32`, D248 | Dropped | Bounds copied MCAL bytes, B471–474; deliberately uses header size rather than subchunk size |
| `0x2c` | `_ofs_mcsh: u32`, D249 | Dropped | Temporary MCSH locator, B477–488 |
| `0x30` | `_size_mcsh: u32`, D250 | Dropped | No read in B path; MCSH uses located subchunk size |
| `0x34` | `_area_id: u32`, D252 | Renamed `area_id` | `area_id` retained, B362 |
| `0x38` | `_n_map_obj_refs: u32`, D253 | Dropped | No read in B path |
| `0x3c` | `_holes_low_res: u16`, D255 | `holes_low_res` | Same named member retained, B363 |
| `0x3e` | `_unknown_but_used: u16`, D256 | Dropped; source name supplies no additional interpretation | No read in B path |
| `0x40` | `low_quality_texture_map: [u8;16]`, D258 | Eight LE u16 rows → `texture_selection` | B retains only eight bytes here as `pred_tex`, B364–368; next eight bytes separately named below |
| `0x50` | `no_effect_doodad: u64`, D260 | LE bytes → eight `detail_exclusion` rows | B retains these eight bytes as `unknown_8bytes`, B374–378, not as E's named exclusion map |
| `0x58` | `_ofs_mcse: u32`, D261 | Dropped; MCSE records parsed from stream | No read in B path |
| `0x5c` | `_n_sound_emitters: u32`, D262 | Dropped | No read in B path |
| `0x60` | `_ofs_mclq: u32`, D263 | Dropped in root-header projection; water reader is separate | Temporary MCLQ locator, B489–493 |
| `0x64` | `_size_mclq: u32`, D264 | Dropped in this projection | Bounds copied liquid block payload, B492–493 |
| `0x68` | `pos_y: f32`, D266 | Becomes `pos[1]` | Retained as B `position[0]`, B380 |
| `0x6c` | `pos_x: f32`, D267 | Becomes `pos[0]` | Retained as B `position[1]`, B381 |
| `0x70` | `pos_z: f32`, D268 | Becomes `pos[2]` | Retained as B `position[2]`, B382 |

B's additional **`no_effect_doodad: [u8;8]` at `0x48`**, B369–373, occupies the second half of E's16-byte texture-selection map. It must not be relabeled as E's `0x50` map. B's `unknown_8bytes` stays opaque even though E names a map at that offset.

## Named flag predicates and downstream boundaries

| Wire flag | E projection / consumer | B distinction |
|---|---|---|
| `0x1` | `has_mcsh`, D22,63,72; parsed flag retained; shadow-map data handled separately, not proof of native baked-shadow rendering | B raw flag retained; shadow data located independently |
| `0x2` | `impass`, D23,64,73; retained in internal `McnkData.flags` but **not copied into public `adt::Chunk`**, A30–52,98–140; native consumer not established | B `MCNK_IMPASSABLE` and `McnkHeader::impassable`, B132–155; retained raw flag supports whole-chunk predicate |
| `0x40` | `has_mccv`, D24,65,74; missing MCCV becomes an explicit error in P741–749 | B MCCV is not represented in the inspected parser/terrain-adapter path, not a repository-wide absence claim |
| `0x8000` | `do_not_fix_alpha_map`, D25,66,75; controls companion alpha edge policy via D484–508 and `adt_tex.rs:234–300,436–483` | No corresponding named B predicate established |
| `0x10000` | `high_res_holes`, D26,67,76; selects u64 hole projection in P625–626 | No corresponding B high-res-hole projection established |
| Other E bits | Not retained by `McnkFlags::from_bits`, D70–79 | B raw u32 survives; B `0x3c` liquid bits select count/kind of MCLQ blocks, B498–545; see water worksheet |

E chunk index, position, heights and hole masks reach public `Chunk` through A30–47,77–140; core origin/height-grid projections are D399–464. Native `godot/rust/src/terrain/streaming.rs:181–191` reads `index_x/index_y/area_id` for area queries. Texture selection/exclusion and hole masks feed `godot/core/src/ground_detail.rs:125–170` (`DetailChunk::from_adt`/`detail_effect_at`), reached by native `terrain/ground_detail.rs:229–236`; they are not additional wire fields. [[benilla-adt-layer-fields]] traces detail consumers. E root MCRF omission is **not** absence of modern object references: object companions parse MCRD/MCRW independently; see [[benilla-adt-placement-fields]].

## Sources

- D21–102,237–269,399–508 and P581–631,702–765 — serialized header, decoded/cooked fields, geometry/stream conversion and subchunk policy.
- A30–47,77–140; native `godot/rust/src/terrain/streaming.rs:181–191` — public retention and area-query consumer.
- B132–155,353–498,498–545 — Vanilla header, actual offset reads and liquid predicates.
- `godot/core/src/asset/adt_format/adt_tex.rs:234–300,436–483` — alpha policy inputs, not a second header layout.

## See Also

- [[benilla-asset-field-inventory]] — broader inventory and evidence limits.
- [[benilla-adt-layer-fields]] — terrain samples, layers and actual native consumers.
- [[benilla-adt-placement-fields]] — companion placement/reference fields.
- [[benilla-terrain-water-fields]] — separate water, WDT and WDL inputs.
- [[benilla-adt-extension-fields]] — blend mesh, flight bounds and LOD inputs.
