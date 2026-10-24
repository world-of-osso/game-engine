# ADT Placement and Object-Reference Fields

Individual MDDF/MODF inputs, name resolution, companion references and active doodad-set projections. Static comparison: E native baseline `354bdb7dbf177047ed46d63168acec531f5c4426`; B `8a6c74036cfe3d00275fd75ad1aeae1f4ea7d0df` (Vanilla1.12.1/build5875 monolithic ADT). No source execution/imports, tests, builds, extraction or runtime/version compatibility proof. Modern `_obj*` flags are not inferred from equal-sized Vanilla records.

**O**=`godot/core/src/asset/adt_format/adt_obj.rs`, **N**=`godot/rust/src/terrain/objects.rs`, **K**=`godot/core/src/campsite_object_data.rs`; **B**=`/home/osso/Repos/benilla/crates/benilla-adt/src/lib.rs`, **F**=B `crates/benilla-formats/src/terrain.rs`. Offsets below are little-endian record offsets, not Rust struct layout. O130–152 declares exactly36B MDDF and64B MODF; O260–346 decodes/converts them. B304–348 reads equal strides but skips MODF extents and final word.

## MDDF:36 bytes

| Offset | Serialized field | E read/retained/derived → actual native consumer | B contrast |
|---|---|---|---|
| `0x00` | `name_id: u32` | Retained plus resolved `fdid`/`path`, O245–285; N754–781 resolves the ADT model; N801,959–975 loads/transforms it | Retained; F470–486 indexes resolved `root.models` and builds `Doodad.model` |
| `0x04` | `unique_id: u32` | Retained; N797–804 uses spawn/dedup/name identity | Retained; copied into cooked doodad F485 |
| `0x08` | `position[0]: f32` | Retained; N965/K63–65,118–140 placement conversion | Retained; F476–480 computes transformed world position |
| `0x0c` | `position[1]: f32` | Same conversion input | Same B transform, not asserted equal E coordinate convention |
| `0x10` | `position[2]: f32` | Same conversion input | Same B transform |
| `0x14` | `rotation[0]: f32` | Retained; N966,973–975 affine transform | Retained; F482 copies rotation |
| `0x18` | `rotation[1]: f32` | Same native transform input | Same copy |
| `0x1c` | `rotation[2]: f32` | Same native transform input | Same copy |
| `0x20` | `scale_raw: u16` | O260–285 converts to public f32 scale `/1024`; N972–975 uses it. No public raw-u16 scale member | B retains raw `scale: u16`; F483 converts `/1024` |
| `0x22` | `flags: u16` | Retained raw; O243–285 tests `0x40` to choose direct FDID versus offset/string resolution. Other bit consumers not established in inspected spawn path | Raw flags retained by parser; F470–486 does not carry them into cooked doodad |

## MODF:64 bytes

| Offset | Serialized field | E retention/production consumer | B contrast |
|---|---|---|---|
| `0x00` | `name_id: u32` | Retained/resolved using `0x8`; N815–822 calls native `wmo/assets.rs:128–144` WMO resolution/load | Retained; F490–510 resolves `root.wmos` |
| `0x04` | `unique_id: u32` | Retained; N812–822,908–911 lifecycle/dedup | Retained/cooked identity F504 |
| `0x08` | `position[0]: f32` | Retained; N887–888/K67–88 conversion | Retained; F498–502 world transform |
| `0x0c` | `position[1]: f32` | Same conversion | Same transform |
| `0x10` | `position[2]: f32` | Same conversion | Same transform |
| `0x14` | `rotation[0]: f32` | Retained; N888,896–898 affine transform | Retained/copied rotation F503 |
| `0x18` | `rotation[1]: f32` | Same native transform | Same copy |
| `0x1c` | `rotation[2]: f32` | Same native transform | Same copy |
| `0x20` | `_lower_bounds[0]: f32` | `extents_min[0]`; K103–109 builds bounds; **N744–748 calls `wmo_bounds` and inserts nearby bounds** | Skipped/unretained, B342 comment |
| `0x24` | `_lower_bounds[1]: f32` | Same retained bounds/nearby consumer | Skipped |
| `0x28` | `_lower_bounds[2]: f32` | Same retained bounds/nearby consumer | Skipped |
| `0x2c` | `_upper_bounds[0]: f32` | `extents_max[0]`; same bounds/nearby consumer | Skipped |
| `0x30` | `_upper_bounds[1]: f32` | Same retained bounds/nearby consumer | Skipped |
| `0x34` | `_upper_bounds[2]: f32` | Same retained bounds/nearby consumer | Skipped |
| `0x38` | `flags: u16` | Raw retained; `0x4` scale-enabled, `0x8` FDID, O298–335; `0x80` selects MWDR/MWDS set mode O54–73 | Raw retained B343; F490–510 does not copy it into cooked `WmoInstance` |
| `0x3a` | `doodad_set: u16` | Retained; O60–73 selects active sets; N820–821,907 hands them to WMO build | Retained/cooked F505; B world terrain-stream spawn/mod.rs532 selects WMO doodads |
| `0x3c` | `name_set: u16` | Retained; N829 uses model label; no additional use established here | Retained/cooked F506; B world terrain-stream spawn/mod.rs323 places name-set identity; B documents WMOAreaTable variant |
| `0x3e` | `scale_raw: u16` | O315–320 derives public f32 `/1024` only with `0x4`; otherwise1.0. N895–898 applies scale | Explicitly unread final u16, B347; no semantic name assigned by B reader. Its1.12 cooked WMO has no scale member |

E's stored position arrays and doodad/WMO conversion functions must be read literally; this inventory does not impose a common transform. B F467–510 uses `[32*TILE-position[2],32*TILE-position[0],position[1]]`; this is a B cooked-coordinate formula, not evidence E should use it. B production terrain-stream spawning consumes cooked placements (`crates/benilla-world/src/terrain_stream/spawn/mod.rs:89–103,298–323,532`); the F projections above establish which raw fields survive that handoff.

## Names, reference lists and doodad-set indirection

O76–114,154–218,245–258 handles these distinct serialized dependencies. Names/offsets are not additional placement fields.

| Chunk/member | Serialized representation | E retention / use | B contrast |
|---|---|---|---|
| MMDX model path bytes | NUL-terminated byte strings | Collected blob; MMID entry resolves placement name to owned lossy-UTF8 `path`; O154–192,220–258 | B249–286 collects blob; B273–277 resolves MMID entries to owned `models` |
| MMID model-name offset[i] | LE u32,4-byte stride | Temporary offset table; O245–258 selects by `name_id` unless direct FDID flag | B offset table retained temporarily and used for `models` projection |
| MWMO WMO path bytes | NUL-terminated byte strings | Collected/resolved to placement `path` through MWID | B analogous `wmos` projection B278–282 |
| MWID WMO-name offset[i] | LE u32,4-byte stride | Temporary name table; modern MODF flag selects direct ID versus this lookup | B indexed-string resolution, not modern direct-FDID flag support |
| MCRD doodad reference[i] | LE u32 | Companion MCNK subchunk `DRCM`; O207–218 → `ChunkObjectRefs.doodad_refs`, returned in `AdtObjData.chunk_refs`. Native terrain use not established in bounded search | No corresponding companion parser in inspected Vanilla ADT path |
| MCRW WMO reference[i] | LE u32 | Companion `WRCM`; same retained wrapper as `wmo_refs`; native use not established | Same bounded nonmatch |
| MWDR `first` | u32 `+0` of8B pair | O103–106 projects `[first,last]`; O60–73 uses inclusive range for flag-`0x80` MODF | No corresponding reader in inspected B path |
| MWDR `last` | u32 `+4` | Inclusive final MWDS index, same selector; not exclusive range end | Same bounded nonmatch |
| MWDS set index[i] | LE u16,2-byte stride | O107–112 projects `wmo_doodad_sets`; MWDR selection → actual N820–821 build input | Same bounded nonmatch |

O60–73's ordinary mode activates set0 plus `placement.doodad_set`; `0x80` mode with a present MWDR pair uses only its inclusive referenced MWDS entries, filtering missing MWDS indices. A missing MWDR pair returns `[0, placement.doodad_set]`, as does ordinary mode; a reversed pair yields no entries. These are observed source branches, not a new fallback policy. `fdid`, `path`, normalized scale and active-set vectors are **derived/helper fields**, not extra record bytes. `AdtObjData.doodads/wmos/chunk_refs` are aggregate arrays. The temporary `Obj0Chunks` blobs/indices are transport containers, not alternate on-disk headers. `chunks_exact` array/pair readers discard incomplete suffixes; no tail-preservation claim. Unknown flags/tags stay unnamed. Root MCRF absence must not be generalized to companion MCRD/MCRW support.

## Sources

- O8–74,76–218,220–346 — raw declarations, name/reference/chunk readers and placement projections.
- N730–748,761–829,887–911,965–975; K63–109,118–140 — actual native model, bounds, transform and identity consumers.
- B173–214,249–286,304–348; F467–510; `crates/benilla-world/src/terrain_stream/spawn/mod.rs:89–103,298–323,532` — older raw fields, cooked placement conversion and production handoff.

## See Also

- [[benilla-adt-header-fields]] — MCNK/root offset and version boundaries.
- [[benilla-adt-layer-fields]] — terrain samples/material inputs.
- [[benilla-adt-extension-fields]] — separate LOD object placement records.
- [[benilla-terrain-water-fields]] — WDT global-WMO placements and liquids.
- [[benilla-wmo-root-aux-fields]] — MODD local WMO doodads, not ADT MDDF.
- [[benilla-asset-field-inventory]] — parent scope and evidence policy.
